use std::collections::HashMap;
use std::fs;

use serde_json::Value;

use super::model::{DiscoveredMcpServer, McpServerConfig};
use super::storage::list_servers;

pub const MEM0_DEFAULT_INSTRUCTIONS: &str = r#"# Mem0 长期记忆规范与使用规则

## 核心要求
你已接入 Mem0 MCP 工具，支持 Mem0 原生三层记忆模型。每次对话中，必须主动调用工具做记忆管理，不要等待用户命令。

## 一、记忆分类与可信度标记
所有记忆按以下格式存储：`[分类标签][可信度] 记忆内容`

### 可信度层级
| 标记 | 含义 | 来源 | 示例 |
|------|------|------|------|
| `✅` | 已确认 | 用户明确决策、正式文档 | `[产品决策][✅] 线下店值班助手统一在前置仓APP中建设` |
| `🟡` | 讨论中 | 对话中提到的待定方案 | `[技术方案][🟡] 考虑用Redis缓存，待验证数据一致性` |
| `💡` | AI推测 | AI分析需求时的推演 | `[AI建议][💡] 建议采用商品ID→BOM→包材ID查询路径` |

### 记忆分类
1. `[产品决策][✅]`：用户明确说"决定/选/确定用 xxx"、"方案是 xxx"。内容：决策结论 + 核心理由（2-3句话）。存 user 层。
2. `[需求演进][✅]`：用户说"之前的方案改了"、"调整成 xxx"。内容：变更前后对比 + 原因。存 user 层。
3. `[用户洞察][✅]`：用户画像、痛点、使用场景。存 user 层。
4. `[项目上下文][✅]`：项目名、迭代、时间节点、核心成员。存 user 层。
5. `[技术约束][✅]`：技术选型、架构限制、依赖关系。存 user 层。
6. `[讨论中][🟡]`：待定方案 + 待解决的问题（用户表达"我在考虑/还没定"）。存 user 层。
7. `[AI建议][💡]`：AI在需求分析时主动推演的方案，自动保存为推演。存 user 层。
8. `[个人偏好][✅]`：工作习惯、沟通风格、输出偏好。存 agent 层（agent_id="trae_pm_assistant"）。

## 二、记忆触发规则
- 立即保存（save_memory）：不要等用户说"记住"。识别到决策、调整、画像、项目信息时主动调用。
- 保存参数规则：默认 user_id="pm_user", agent_id="trae_pm_assistant"；个人偏好必传 agent_id；会话讨论传 run_id。

## 三、记忆召回策略
- 每次开始回答用户问题前，必须先调用 search_memories 搜索相关上下文。
- 优先使用 [✅] 记忆；[🟡] 记忆需标注"待确认"；写正式文档时不混入未验证的 [💡] 记忆。

## 四、三层记忆模型参数
- user 层：`user_id="pm_user"`（跨会话持久化的产品认知库）
- agent 层：`agent_id="trae_pm_assistant"`（AI学习如何服务该用户的偏好与风格）
- session 层：`run_id="YYYY-MM-DD_主题"`（当前会话活跃讨论与草稿）"#;

pub fn discover_local_mcps() -> Vec<DiscoveredMcpServer> {
    let registered_ids: Vec<String> = list_servers()
        .map(|list| list.into_iter().map(|s| s.id).collect())
        .unwrap_or_default();

    let mut results = Vec::new();
    let config_dir = match dirs::config_dir() {
        Some(path) => path,
        None => return results,
    };

    let search_targets = vec![
        (
            "Trae SOLO CN",
            config_dir.join("TRAE SOLO CN/User/mcp.json"),
        ),
        ("Trae CN", config_dir.join("Trae CN/User/mcp.json")),
        ("Trae", config_dir.join("Trae/User/mcp.json")),
        (
            "Claude Desktop",
            config_dir.join("Claude/claude_desktop_config.json"),
        ),
    ];

    for (source_name, path) in search_targets {
        if !path.exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(json_val) = serde_json::from_str::<Value>(&content) {
                if let Some(servers) = json_val.get("mcpServers").and_then(Value::as_object) {
                    for (server_id, server_val) in servers {
                        let command = server_val
                            .get("command")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        if command.is_empty() {
                            continue;
                        }
                        let args = server_val
                            .get("args")
                            .and_then(Value::as_array)
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(Value::as_str)
                                    .map(|s| s.to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        let mut env = HashMap::new();
                        if let Some(env_obj) = server_val.get("env").and_then(Value::as_object) {
                            for (k, v) in env_obj {
                                if let Some(s) = v.as_str() {
                                    env.insert(k.clone(), s.to_string());
                                }
                            }
                        }

                        let desc = format!("从 {} 自动发现的 MCP 服务", source_name);
                        let is_mem0 = server_id.to_lowercase() == "mem0";
                        let (name, instructions) = if is_mem0 {
                            (
                                "Mem0 长期记忆".to_string(),
                                MEM0_DEFAULT_INSTRUCTIONS.to_string(),
                            )
                        } else {
                            (server_id.clone(), String::new())
                        };

                        let is_registered = registered_ids.iter().any(|r| r == server_id);

                        results.push(DiscoveredMcpServer {
                            source: source_name.to_string(),
                            source_file: path.display().to_string(),
                            already_registered: is_registered,
                            config: McpServerConfig {
                                id: server_id.clone(),
                                name,
                                description: desc,
                                instructions,
                                usage_mode: if is_mem0 {
                                    "always_on".to_string()
                                } else {
                                    "on_demand".to_string()
                                },
                                transport: "stdio".to_string(),
                                command,
                                args,
                                env,
                                enabled: true,
                                created_at: String::new(),
                                updated_at: String::new(),
                            },
                        });
                    }
                }
            }
        }
    }

    results
}
