use crate::external_mcp::model::{DiscoveredMcpServer, McpServerConfig, McpToolSummary};
use crate::external_mcp::{
    delete_server, discover_local_mcps, global_manager, list_servers, set_server_enabled,
    test_and_probe_tools, upsert_server,
};

#[tauri::command]
pub async fn list_external_mcps() -> Result<Vec<McpServerConfig>, String> {
    list_servers().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_external_mcp(config: McpServerConfig) -> Result<(), String> {
    if config.id.trim().is_empty() {
        return Err("MCP 标识 (ID) 不能为空".to_string());
    }
    if config.command.trim().is_empty() {
        return Err("执行命令 (Command) 不能为空".to_string());
    }
    let manager = global_manager();
    // 尝试拉取并缓存一次工具
    if config.enabled {
        let _ = manager.refresh_server_tools(&config).await;
    } else {
        manager.remove_session(&config.id).await;
    }
    upsert_server(config).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_external_mcp(id: String) -> Result<(), String> {
    let manager = global_manager();
    manager.remove_session(&id).await;
    delete_server(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_external_mcp_enabled(id: String, enabled: bool) -> Result<(), String> {
    let manager = global_manager();
    if !enabled {
        manager.remove_session(&id).await;
    } else if let Ok(Some(cfg)) = crate::external_mcp::get_server(&id) {
        let _ = manager.refresh_server_tools(&cfg).await;
    }
    set_server_enabled(&id, enabled).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_external_mcp(config: McpServerConfig) -> Result<Vec<McpToolSummary>, String> {
    test_and_probe_tools(&config).await
}

#[tauri::command]
pub async fn discover_external_mcps() -> Result<Vec<DiscoveredMcpServer>, String> {
    Ok(discover_local_mcps())
}
