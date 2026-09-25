pub mod client;
pub mod discovery;
pub mod model;
pub mod storage;

pub use client::{global_manager, test_and_probe_tools, ExternalMcpManager};
pub use discovery::discover_local_mcps;
pub use model::{DiscoveredMcpServer, ExternalMcpStore, McpServerConfig, McpToolSummary};
pub use storage::{delete_server, get_server, list_servers, set_server_enabled, upsert_server};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_mcp_server_config_serde() {
        let mut env = HashMap::new();
        env.insert("MEM0_API_KEY".to_string(), "test_key".to_string());
        let config = McpServerConfig {
            id: "mem0".to_string(),
            name: "Mem0 长期记忆".to_string(),
            description: "记忆管理".to_string(),
            instructions: "记忆规范内容".to_string(),
            usage_mode: "always_on".to_string(),
            transport: "stdio".to_string(),
            command: "docker".to_string(),
            args: vec!["run".to_string(), "--rm".to_string()],
            env,
            enabled: true,
            created_at: "123".to_string(),
            updated_at: "123".to_string(),
        };
        let serialized = serde_json::to_string(&config).expect("serialize");
        let deserialized: McpServerConfig = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(deserialized.id, "mem0");
        assert_eq!(deserialized.name, "Mem0 长期记忆");
        assert!(deserialized.is_always_on());
        assert_eq!(deserialized.args.len(), 2);
        assert_eq!(
            deserialized.env.get("MEM0_API_KEY").map(String::as_str),
            Some("test_key")
        );
    }

    #[test]
    fn test_mcp_server_config_defaults_to_on_demand() {
        let raw = r#"{
            "id":"legacy",
            "name":"Legacy MCP",
            "command":"legacy-mcp"
        }"#;
        let config: McpServerConfig = serde_json::from_str(raw).expect("deserialize legacy config");
        assert_eq!(config.usage_mode, "on_demand");
        assert!(!config.is_always_on());
    }

    #[test]
    fn test_discover_local_mcps_runs() {
        let discovered = discover_local_mcps();
        println!("Discovered {} local MCP servers:", discovered.len());
        for d in &discovered {
            println!(
                " - [{}] ID: {}, Command: {} {:?}",
                d.source, d.config.id, d.config.command, d.config.args
            );
        }
        assert!(!discovered.is_empty());
    }
}
