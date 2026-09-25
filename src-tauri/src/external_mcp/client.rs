use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;

use super::model::{McpServerConfig, McpToolSummary};

static REQ_COUNTER: AtomicU64 = AtomicU64::new(100);

fn next_id() -> u64 {
    REQ_COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn normalize_match_text(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

fn bigrams(value: &str) -> HashSet<String> {
    let chars: Vec<char> = normalize_match_text(value).chars().collect();
    if chars.is_empty() {
        return HashSet::new();
    }
    if chars.len() == 1 {
        return [chars[0].to_string()].into_iter().collect();
    }
    chars
        .windows(2)
        .map(|pair| pair.iter().collect::<String>())
        .collect()
}

fn text_match_score(query: &str, candidate: &str) -> f64 {
    let query_norm = normalize_match_text(query);
    let candidate_norm = normalize_match_text(candidate);
    if query_norm.is_empty() || candidate_norm.is_empty() {
        return 0.0;
    }
    if query_norm.contains(&candidate_norm) || candidate_norm.contains(&query_norm) {
        return 1.0;
    }
    let query_grams = bigrams(query);
    let candidate_grams = bigrams(candidate);
    let denominator = query_grams.len().min(candidate_grams.len());
    if denominator == 0 {
        return 0.0;
    }
    let overlap = query_grams.intersection(&candidate_grams).count();
    overlap as f64 / denominator as f64
}

fn external_tool_match_score(task: &str, config: &McpServerConfig, tool: &McpToolSummary) -> f64 {
    let server_identity = format!("{} {}", config.id, config.name);
    let tool_identity = format!("{} {}", tool.name, tool.exposed_name);
    let score = text_match_score(task, &tool.description)
        .max(text_match_score(task, &config.description) * 0.9)
        .max(text_match_score(task, &tool_identity) * 0.85)
        .max(text_match_score(task, &server_identity) * 0.75);

    // 过滤极弱的偶然字符重合，并把输出稳定在 4 位小数。
    if score < 0.08 {
        0.0
    } else {
        (score * 10_000.0).round() / 10_000.0
    }
}

/// 解析可执行程序路径，确保在非终端环境中也能找到 docker/npx 等命令。
fn resolve_command_path(cmd: &str) -> String {
    let trimmed = cmd.trim();
    if trimmed.is_empty() || trimmed.contains('/') {
        return trimmed.to_string();
    }
    if let Ok(p) = which::which(trimmed) {
        return p.to_string_lossy().into_owned();
    }
    #[cfg(unix)]
    {
        if let Ok(output) = std::process::Command::new("/bin/zsh")
            .args(["-lc", &format!("command -v {trimmed}")])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        {
            if output.status.success() {
                if let Ok(p) = String::from_utf8(output.stdout) {
                    let p = p.trim();
                    if p.starts_with('/') {
                        return p.to_string();
                    }
                }
            }
        }
    }
    trimmed.to_string()
}

pub struct StdioMcpSession {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

impl StdioMcpSession {
    pub async fn spawn(config: &McpServerConfig) -> Result<Self, String> {
        let cmd_path = resolve_command_path(&config.command);
        let mut cmd = Command::new(&cmd_path);
        cmd.args(&config.args);
        cmd.envs(&config.env);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("启动外部 MCP 进程失败 ({}): {}", config.command, e))?;

        let stdin = child.stdin.take().ok_or("无法获取子进程 stdin")?;
        let stdout = child.stdout.take().ok_or("无法获取子进程 stdout")?;
        let reader = BufReader::new(stdout);

        let mut session = Self {
            child,
            stdin,
            reader,
        };

        // 执行握手流程
        session.handshake().await?;
        Ok(session)
    }

    async fn send_raw(&mut self, payload: &Value) -> Result<(), String> {
        let mut line = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("写入外部 MCP stdin 失败: {e}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| format!("刷新外部 MCP stdin 失败: {e}"))?;
        Ok(())
    }

    async fn read_response(
        &mut self,
        expected_id: u64,
        timeout_dur: Duration,
    ) -> Result<Value, String> {
        let res = timeout(timeout_dur, async {
            let mut line = String::new();
            loop {
                line.clear();
                let bytes = self
                    .reader
                    .read_line(&mut line)
                    .await
                    .map_err(|e| format!("读取外部 MCP stdout 失败: {e}"))?;
                if bytes == 0 {
                    return Err("外部 MCP 进程输出提前结束 (EOF)".to_string());
                }
                let trimmed = line.trim();
                if trimmed.is_empty() || !trimmed.starts_with('{') {
                    continue;
                }
                if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
                    if val.get("id").and_then(Value::as_u64) == Some(expected_id) {
                        return Ok(val);
                    }
                }
            }
        })
        .await;

        match res {
            Ok(Ok(val)) => Ok(val),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!("外部 MCP 请求超时 (id={expected_id})")),
        }
    }

    pub async fn handshake(&mut self) -> Result<(), String> {
        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "MCP-Gateway",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        });
        self.send_raw(&init_req).await?;
        let resp = self.read_response(1, Duration::from_secs(20)).await?;
        if let Some(err) = resp.get("error") {
            return Err(format!("初始化外部 MCP 失败: {err}"));
        }

        // 发送 initialized 通知
        let init_notify = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let _ = self.send_raw(&init_notify).await;
        Ok(())
    }

    pub async fn list_tools(&mut self) -> Result<Vec<McpToolSummary>, String> {
        let req_id = next_id();
        let list_req = json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "method": "tools/list",
            "params": {}
        });
        self.send_raw(&list_req).await?;
        let resp = self.read_response(req_id, Duration::from_secs(30)).await?;
        if let Some(err) = resp.get("error") {
            return Err(format!("外部 MCP tools/list 错误: {err}"));
        }

        let raw_tools = resp
            .get("result")
            .and_then(|r| r.get("tools"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut list = Vec::new();
        for t in raw_tools {
            let name = t
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let description = t
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let input_schema = t
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| json!({"type": "object", "properties": {}}));
            list.push(McpToolSummary {
                name: name.clone(),
                exposed_name: name,
                description,
                input_schema,
            });
        }
        Ok(list)
    }

    pub async fn call_tool(&mut self, tool_name: &str, arguments: &Value) -> Result<Value, String> {
        let req_id = next_id();
        let call_req = json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });
        self.send_raw(&call_req).await?;
        let resp = self.read_response(req_id, Duration::from_secs(120)).await?;
        if let Some(err) = resp.get("error") {
            return Err(format!("外部 MCP 调用失败: {err}"));
        }
        resp.get("result")
            .cloned()
            .ok_or_else(|| "外部 MCP 响应缺少 result 字段".to_string())
    }

    pub async fn shutdown(&mut self) {
        let _ = self.child.kill().await;
    }
}

/// 测试并探测外部 MCP，返回其暴露的所有工具列表
pub async fn test_and_probe_tools(config: &McpServerConfig) -> Result<Vec<McpToolSummary>, String> {
    let mut session = StdioMcpSession::spawn(config).await?;
    let mut tools = session.list_tools().await?;
    session.shutdown().await;
    for t in tools.iter_mut() {
        t.exposed_name = format!("{}__{}", config.id, t.name);
    }
    Ok(tools)
}

/// 全局长连接客户端池，避免每次工具调用时重新创建进程与握手。
pub struct ExternalMcpManager {
    sessions: Arc<Mutex<HashMap<String, StdioMcpSession>>>,
    tool_cache: Arc<Mutex<HashMap<String, Vec<McpToolSummary>>>>,
}

impl ExternalMcpManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            tool_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 刷新并缓存指定 server 的工具列表
    pub async fn refresh_server_tools(
        &self,
        config: &McpServerConfig,
    ) -> Result<Vec<McpToolSummary>, String> {
        if !config.enabled {
            let mut cache = self.tool_cache.lock().await;
            cache.remove(&config.id);
            return Ok(vec![]);
        }
        let tools = test_and_probe_tools(config).await?;
        let mut cache = self.tool_cache.lock().await;
        cache.insert(config.id.clone(), tools.clone());
        Ok(tools)
    }

    /// 获取所有启用 server 的工具集合，用于 `tools/list`
    pub async fn get_all_exposed_tools(&self) -> Vec<Value> {
        let configs = match super::storage::list_servers() {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let mut cache = self.tool_cache.lock().await;
        let mut result = Vec::new();

        for config in configs.into_iter().filter(|s| s.enabled) {
            let tools = if let Some(cached) = cache.get(&config.id) {
                cached.clone()
            } else {
                // 缓存未命中时尝试探测一次
                drop(cache);
                let probed = test_and_probe_tools(&config).await.unwrap_or_default();
                cache = self.tool_cache.lock().await;
                cache.insert(config.id.clone(), probed.clone());
                probed
            };

            for tool in tools {
                let mut desc = if tool.description.is_empty() {
                    format!("[外部 MCP: {}] 提供", config.name)
                } else {
                    format!("[外部 MCP: {}] {}", config.name, tool.description)
                };
                // 只有常驻型 MCP 才把完整规范附加到直接暴露的工具描述上。
                // 按需型 MCP 保持简洁，避免多个外部服务把上下文撑大。
                if config.is_always_on() && !config.instructions.trim().is_empty() {
                    desc.push_str(&format!(
                        "\n\n[使用规范与规则 / Instructions]:\n{}",
                        config.instructions.trim()
                    ));
                }
                result.push(json!({
                    "name": tool.exposed_name,
                    "description": desc,
                    "inputSchema": tool.input_schema
                }));
            }
        }
        result
    }

    /// 解析工具调用，返回 (server_config, 内部工具名)
    pub async fn resolve_tool(&self, name: &str) -> Option<(McpServerConfig, String)> {
        let configs = super::storage::list_servers().ok()?;
        let enabled_configs: Vec<_> = configs.into_iter().filter(|s| s.enabled).collect();

        // 1. 如果包含 `__` 前缀，如 `mem0__search_memories`
        if let Some((server_id, tool_name)) = name.split_once("__") {
            if let Some(cfg) = enabled_configs.iter().find(|s| s.id == server_id) {
                return Some((cfg.clone(), tool_name.to_string()));
            }
        }

        // 2. 如果不带前缀，直接查各 server 缓存中的原生工具名
        let cache = self.tool_cache.lock().await;
        for cfg in enabled_configs {
            if let Some(tools) = cache.get(&cfg.id) {
                if tools.iter().any(|t| t.name == name) {
                    return Some((cfg, name.to_string()));
                }
            }
        }
        None
    }

    /// 执行外部工具调用
    pub async fn execute_tool(
        &self,
        config: &McpServerConfig,
        tool_name: &str,
        args: &Value,
    ) -> Result<Value, String> {
        let mut sessions = self.sessions.lock().await;
        let session = if let Some(s) = sessions.get_mut(&config.id) {
            s
        } else {
            let new_session = StdioMcpSession::spawn(config).await?;
            sessions.insert(config.id.clone(), new_session);
            sessions.get_mut(&config.id).unwrap()
        };

        match session.call_tool(tool_name, args).await {
            Ok(val) => Ok(val),
            Err(first_err) => {
                // 如果调用失败（可能是进程退出或管道损坏），清理并尝试重连一次
                sessions.remove(&config.id);
                let mut retry_session = StdioMcpSession::spawn(config)
                    .await
                    .map_err(|e| format!("外部 MCP 重连失败: {e} (原错误: {first_err})"))?;
                let val = retry_session.call_tool(tool_name, args).await?;
                sessions.insert(config.id.clone(), retry_session);
                Ok(val)
            }
        }
    }

    pub async fn remove_session(&self, server_id: &str) {
        let mut sessions = self.sessions.lock().await;
        if let Some(mut s) = sessions.remove(server_id) {
            s.shutdown().await;
        }
        let mut cache = self.tool_cache.lock().await;
        cache.remove(server_id);
    }

    /// 详细列出已注册外部 MCP 及其所有工具定义，用于 mcp_list 工具
    pub async fn list_servers_detailed(
        &self,
        server_id_filter: Option<&str>,
        refresh: bool,
        full: bool,
    ) -> Result<Vec<Value>, String> {
        let configs = match super::storage::list_servers() {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let mut results = Vec::new();
        for config in configs {
            if let Some(fid) = server_id_filter {
                if !fid.trim().is_empty() && config.id != fid.trim() {
                    continue;
                }
            }
            if !config.enabled {
                let mut item = json!({
                    "id": config.id,
                    "name": config.name,
                    "description": config.description,
                    "usage_mode": config.usage_mode,
                    "enabled": false,
                    "tools": []
                });
                if full {
                    item["instructions"] = json!(config.instructions);
                }
                results.push(item);
                continue;
            }

            let tools = if refresh {
                self.refresh_server_tools(&config).await.unwrap_or_default()
            } else {
                let cache = self.tool_cache.lock().await;
                if let Some(cached) = cache.get(&config.id) {
                    cached.clone()
                } else {
                    drop(cache);
                    let probed = test_and_probe_tools(&config).await.unwrap_or_default();
                    let mut cache = self.tool_cache.lock().await;
                    cache.insert(config.id.clone(), probed.clone());
                    probed
                }
            };

            let tool_items: Vec<Value> = tools
                .into_iter()
                .map(|t| {
                    if full {
                        json!({
                            "name": t.name,
                            "exposed_name": t.exposed_name,
                            "description": t.description,
                            "input_schema": t.input_schema
                        })
                    } else {
                        json!({
                            "name": t.name,
                            "exposed_name": t.exposed_name,
                            "description": t.description
                        })
                    }
                })
                .collect();

            let mut item = json!({
                "id": config.id,
                "name": config.name,
                "description": config.description,
                "usage_mode": config.usage_mode,
                "enabled": true,
                "tools": tool_items
            });
            if full {
                item["instructions"] = json!(config.instructions);
            }
            results.push(item);
        }
        Ok(results)
    }

    /// 根据完整用户任务，从已注册外部 MCP 中匹配最相关的工具。
    /// 第一版使用确定性的字符 n-gram 相似度，不依赖额外模型或网络服务。
    pub async fn match_tools(
        &self,
        task: &str,
        limit: usize,
        refresh: bool,
    ) -> Result<Vec<Value>, String> {
        let task = task.trim();
        if task.is_empty() {
            return Ok(vec![]);
        }

        let configs = super::storage::list_servers().map_err(|e| e.to_string())?;
        let mut matches = Vec::new();

        for config in configs.into_iter().filter(|s| s.enabled) {
            let tools = if refresh {
                self.refresh_server_tools(&config).await.unwrap_or_default()
            } else {
                let cache = self.tool_cache.lock().await;
                if let Some(cached) = cache.get(&config.id) {
                    cached.clone()
                } else {
                    drop(cache);
                    let probed = test_and_probe_tools(&config).await.unwrap_or_default();
                    let mut cache = self.tool_cache.lock().await;
                    cache.insert(config.id.clone(), probed.clone());
                    probed
                }
            };

            for tool in tools {
                let score = external_tool_match_score(task, &config, &tool);
                if score <= 0.0 {
                    continue;
                }
                matches.push(json!({
                    "server_id": config.id,
                    "server_name": config.name,
                    "usage_mode": config.usage_mode,
                    "tool": tool.name,
                    "exposed_name": tool.exposed_name,
                    "description": tool.description,
                    "score": score
                }));
            }
        }

        matches.sort_by(|a, b| {
            let a_score = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
            let b_score = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches.truncate(limit.clamp(1, 10));
        Ok(matches)
    }

    /// 执行外部 MCP 工具调用，支持指定 server_id 或自动按工具名路由，用于 mcp_call 工具
    pub async fn call_external_tool_flexible(
        &self,
        server_id: Option<&str>,
        tool_name: &str,
        args: &Value,
    ) -> Result<Value, String> {
        let configs = super::storage::list_servers().map_err(|e| e.to_string())?;
        let enabled_configs: Vec<_> = configs.into_iter().filter(|s| s.enabled).collect();

        let (target_config, inner_tool) = if let Some(sid) =
            server_id.filter(|s| !s.trim().is_empty())
        {
            let cfg = enabled_configs
                .into_iter()
                .find(|s| s.id == sid)
                .ok_or_else(|| format!("未找到启用的外部 MCP 服务: {sid}"))?;
            let inner = tool_name
                .strip_prefix(&format!("{sid}__"))
                .unwrap_or(tool_name);
            (cfg, inner.to_string())
        } else if let Some((resolved_cfg, inner)) = self.resolve_tool(tool_name).await {
            (resolved_cfg, inner)
        } else {
            return Err(format!(
                    "未识别的外部 MCP 工具: {tool_name}。请先调用 mcp_match 匹配能力；若挂载可能已变化，再调用 mcp_list(refresh=true) 刷新查看"
                ));
        };

        self.execute_tool(&target_config, &inner_tool, args).await
    }
}

static MANAGER: std::sync::OnceLock<ExternalMcpManager> = std::sync::OnceLock::new();

pub fn global_manager() -> &'static ExternalMcpManager {
    MANAGER.get_or_init(ExternalMcpManager::new)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::{external_tool_match_score, McpServerConfig, McpToolSummary};

    fn sample_config() -> McpServerConfig {
        McpServerConfig {
            id: "lark".to_string(),
            name: "飞书文档".to_string(),
            description: "读取和编辑飞书文档、Wiki 与知识库".to_string(),
            instructions: String::new(),
            usage_mode: "on_demand".to_string(),
            transport: "stdio".to_string(),
            command: "lark-mcp".to_string(),
            args: vec![],
            env: HashMap::new(),
            enabled: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn matcher_recognizes_chinese_external_capability() {
        let config = sample_config();
        let tool = McpToolSummary {
            name: "read_document".to_string(),
            exposed_name: "lark__read_document".to_string(),
            description: "读取飞书文档和 Wiki 内容".to_string(),
            input_schema: json!({"type":"object"}),
        };
        let score = external_tool_match_score("帮我读取这个飞书文档并总结内容", &config, &tool);
        assert!(score >= 0.35, "unexpected score: {score}");
    }

    #[test]
    fn matcher_filters_unrelated_capability() {
        let config = sample_config();
        let tool = McpToolSummary {
            name: "read_document".to_string(),
            exposed_name: "lark__read_document".to_string(),
            description: "读取飞书文档和 Wiki 内容".to_string(),
            input_schema: json!({"type":"object"}),
        };
        let score = external_tool_match_score("计算本地 Rust 项目的测试覆盖率", &config, &tool);
        assert_eq!(score, 0.0);
    }
}
