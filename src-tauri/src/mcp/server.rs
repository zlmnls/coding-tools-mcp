use std::sync::Arc;

use serde_json::Value;

use crate::tools::{
    call_tool, list_tools_for_profile, wrap_mcp_tool_result, SharedToolContext, ToolContext,
    Workspace,
};
use crate::workspace::AuthConfig;

pub type SharedState = SharedToolContext;

pub fn handle_request(state: &SharedState, body: &Value) -> Value {
    let method = body.get("method").and_then(Value::as_str).unwrap_or("");
    let id = body.get("id").cloned().unwrap_or(Value::Null);
    let params = body.get("params").cloned().unwrap_or(Value::Null);

    if id.is_null() && method.starts_with("notifications/") {
        return Value::Null;
    }

    let result = match method {
        "initialize" => Ok(initialize_result()),
        "ping" => Ok(serde_json::json!({})),
        "tools/list" => {
            let mut tools = list_tools_for_profile(&state.tool_profile);
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let external_tools =
                    handle.block_on(crate::external_mcp::global_manager().get_all_exposed_tools());
                tools.extend(external_tools);
            }
            Ok(serde_json::json!({ "tools": tools }))
        }
        "tools/call" => handle_tools_call(state, &params),
        _ => Err(serde_json::json!({
            "code": -32601,
            "message": format!("Method not found: {method}")
        })),
    };

    match result {
        Ok(result) => serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err(error) => serde_json::json!({ "jsonrpc": "2.0", "id": id, "error": error }),
    }
}

fn initialize_result() -> Value {
    let mut instructions = String::from("Use these tools only for local coding operations inside the configured workspace. At the start of every new ChatGPT conversation, before answering the user's first request, call history_session_bootstrap exactly once and pass the user's verbatim first request as initial_user_input. Treat bootstrap as required conversation initialization: it creates or resumes a lossless Markdown archive and returns bounded current state, not all history. Use history_session_search followed by history_session_read only when exact earlier context is needed. history_session_read returns a bounded UTF-8-safe page; follow next_cursor with the returned content hash until the relevant archive is complete. Repeated successful bootstrap calls in the same conversation resume the same session and must not create duplicates. Preserve session_key and current_path returned by bootstrap, then pass them unchanged as session_key and expected_path to every history_session_checkpoint call. After completing each user-requested task in the conversation, call history_session_checkpoint before the final response and pass that user's verbatim request as raw_user_input. Only state that progress was saved after checkpoint returns ok=true with the same session_key and path. The server cannot access ChatGPT transcript text that was not provided as a tool argument; persistence is not automatic background persistence. External MCP capability routing: enabled external MCP servers extend this environment. Always-on MCPs must be used according to their injected instructions. If a specific exposed external tool clearly matches the task, call it directly. Otherwise, when a task may depend on connected or external data such as long-term memory, enterprise documents, internal knowledge, databases, business systems, or another registered application, call mcp_match with the complete user task. If no useful match is returned, or mounted MCP tools may have changed, call mcp_list with refresh=true. Use mcp_call to execute a selected external tool when it is not directly exposed. Do not conclude that an external capability or data source is unavailable before checking registered external MCP capabilities. Skills are progressively disclosed: when a user task may benefit from a registered Skill, call skill_match with the complete task, select the most relevant result, call skill_load before executing anything, load referenced resources only when needed, and call skill_exec only for commands required by the loaded Skill. skill_load can explain pending dependencies; skill_exec requires a ready Skill. Local execution uses the host environment; sandbox execution uses the isolated environment. If no Skill matches, continue with normal tools.");

    if let Ok(servers) = crate::external_mcp::list_servers() {
        let mut on_demand_summaries = Vec::new();
        for s in servers.into_iter().filter(|s| s.enabled) {
            if s.is_always_on() {
                let description = if s.description.trim().is_empty() {
                    "Always-on external MCP capability"
                } else {
                    s.description.trim()
                };
                instructions.push_str(&format!(
                    "\n\n### Always-on external MCP [{} / {}]\nCapability: {}",
                    s.name, s.id, description
                ));
                if !s.instructions.trim().is_empty() {
                    instructions.push_str(&format!("\nInstructions:\n{}", s.instructions.trim()));
                }
            } else {
                let description = if s.description.trim().is_empty() {
                    "On-demand external MCP capability"
                } else {
                    s.description.trim()
                };
                on_demand_summaries.push(format!("- [{} / {}] {}", s.name, s.id, description));
            }
        }

        if !on_demand_summaries.is_empty() {
            instructions.push_str(
                "\n\n### On-demand external MCP capabilities\nThese capabilities are available when relevant. Use a directly exposed tool when obvious; otherwise call mcp_match before mcp_list.\n",
            );
            instructions.push_str(&on_demand_summaries.join("\n"));
        }
    }

    serde_json::json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {
            "tools": { "listChanged": false },
            "logging": {}
        },
        "serverInfo": {
            "name": "MCP-Gateway",
            "title": "MCP-Gateway",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": instructions
    })
}

fn handle_tools_call(state: &SharedState, params: &Value) -> Result<Value, Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| serde_json::json!({ "code": -32602, "message": "Missing tool name" }))?;
    let args = tool_arguments(name, params);

    // 首先检查是否命中已注册的外部 MCP 工具
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if let Some((server_config, orig_tool_name)) =
            handle.block_on(crate::external_mcp::global_manager().resolve_tool(name))
        {
            let res = handle.block_on(crate::external_mcp::global_manager().execute_tool(
                &server_config,
                &orig_tool_name,
                &args,
            ));
            let result_payload = match res {
                Ok(val) => {
                    if val.is_object() && val.get("content").is_some() {
                        val
                    } else {
                        serde_json::json!({
                            "content": [{ "type": "text", "text": serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string()) }],
                            "isError": false
                        })
                    }
                }
                Err(err_msg) => {
                    serde_json::json!({
                        "content": [{ "type": "text", "text": format!("外部 MCP 工具执行失败: {err_msg}") }],
                        "isError": true
                    })
                }
            };
            return Ok(result_payload);
        }
    }

    let canonical_name = crate::tools::registry::canonical_tool_name(name);
    let known = crate::tools::registry::exposed_tool_names(&state.tool_profile);
    if !known.iter().any(|n| n == &canonical_name) {
        return Err(serde_json::json!({
            "code": -32602,
            "message": format!("Unknown tool: {name}"),
            "data": { "reason": "unknown_tool" }
        }));
    }

    let structured = call_tool(state.as_ref(), canonical_name, &args);
    Ok(wrap_mcp_tool_result(canonical_name, &args, structured))
}

fn tool_arguments(name: &str, params: &Value) -> Value {
    let mut args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if name.starts_with("history_session_") {
        if let Some(session_key) = params
            .get("_meta")
            .and_then(|meta| meta.get("openai/session"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if !args.is_object() {
                args = serde_json::json!({});
            }
            args["_host_session_key"] = Value::String(session_key.to_string());
        }
    }
    args
}

pub fn new_state(
    workspace: Workspace,
    auth: AuthConfig,
    policy: crate::tools::policy::PolicySettings,
    tool_profile: String,
    permission_mode: String,
) -> SharedState {
    Arc::new(ToolContext::from_workspace(
        workspace,
        auth,
        policy,
        tool_profile,
        permission_mode,
    ))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use serde_json::json;

    use crate::tools::ToolContext;

    use super::{handle_request, initialize_result, tool_arguments};

    #[test]
    fn initialize_instructions_define_the_history_persistence_workflow() {
        let initialized = initialize_result();
        let instructions = initialized["instructions"].as_str().expect("instructions");
        assert!(instructions.contains("history_session_bootstrap"));
        assert!(instructions.contains("At the start of every new ChatGPT conversation"));
        assert!(instructions.contains("before answering the user's first request"));
        assert!(instructions.contains("required conversation initialization"));
        assert!(instructions.contains("initial_user_input"));
        assert!(instructions.contains("must not create duplicates"));
        assert!(instructions.contains("history_session_checkpoint"));
        assert!(instructions.contains("raw_user_input"));
        assert!(instructions.contains("history_session_search"));
        assert!(instructions.contains("history_session_read"));
        assert!(instructions.contains("follow next_cursor"));
        assert!(instructions.contains("session_key and current_path returned by bootstrap"));
        assert!(instructions.contains("session_key and expected_path"));
        assert!(instructions.contains("After completing each user-requested task"));
        assert!(instructions.contains("before the final response"));
        assert!(instructions.contains("checkpoint returns ok=true"));
        assert!(instructions.contains("not automatic background persistence"));
    }

    #[test]
    fn initialize_does_not_claim_tool_catalog_notifications_without_a_stream() {
        let initialized = initialize_result();

        assert_eq!(initialized["capabilities"]["tools"]["listChanged"], false);
    }

    #[test]
    fn chatgpt_session_metadata_is_injected_only_for_history_tools() {
        let params = json!({
            "arguments": {"session_key": "explicit"},
            "_meta": {"openai/session": "chatgpt-conversation"}
        });
        let history = tool_arguments("history_session_bootstrap", &params);
        assert_eq!(history["session_key"], "explicit");
        assert_eq!(history["_host_session_key"], "chatgpt-conversation");

        let existing = tool_arguments("read_file", &params);
        assert_eq!(existing["session_key"], "explicit");
        assert!(existing.get("_host_session_key").is_none());
    }

    #[test]
    fn host_session_key_takes_precedence_over_explicit_session_key() {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let harness = tempfile::tempdir().expect("harness tempdir");
        let state = Arc::new(
            ToolContext::for_test(workspace.path().to_path_buf(), harness.path().to_path_buf())
                .expect("tool context"),
        );
        let response = handle_request(
            &state,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "history_session_bootstrap",
                    "arguments": {
                        "session_key": "explicit-session",
                        "initial_user_input": "保存首轮原文"
                    },
                    "_meta": {"openai/session": "chatgpt-session"}
                }
            }),
        );
        let structured = &response["result"]["structuredContent"];
        assert_eq!(structured["ok"], true);
        assert_eq!(structured["session_key_source"], "platform_conversation_id");
        assert_eq!(structured["session_key"], "chatgpt-session");
        assert_eq!(structured["initial_input_captured"], true);
        let content = fs::read_to_string(workspace.path().join("docs/history-session/1.md"))
            .expect("read history file");
        assert!(content.contains("**Session key:** chatgpt-session"));
        assert!(!content.contains("**Session key:** explicit-session"));
    }

    #[test]
    fn legacy_grep_calls_are_mapped_to_the_public_grep_text_tool() {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let harness = tempfile::tempdir().expect("harness tempdir");
        fs::write(workspace.path().join("sample.txt"), "catalog needle")
            .expect("write sample file");
        let state = Arc::new(
            ToolContext::for_test(workspace.path().to_path_buf(), harness.path().to_path_buf())
                .expect("tool context"),
        );

        let response = handle_request(
            &state,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "grep",
                    "arguments": {"query": "needle", "path": "."}
                }
            }),
        );

        assert!(response.get("error").is_none());
        assert_eq!(response["result"]["structuredContent"]["ok"], true);
    }
}
