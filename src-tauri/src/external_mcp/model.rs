use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub instructions: String,
    #[serde(default = "default_usage_mode")]
    pub usage_mode: String,
    #[serde(default = "default_transport")]
    pub transport: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_transport() -> String {
    "stdio".to_string()
}

fn default_usage_mode() -> String {
    "on_demand".to_string()
}

fn default_true() -> bool {
    true
}

impl McpServerConfig {
    pub fn is_always_on(&self) -> bool {
        self.usage_mode.eq_ignore_ascii_case("always_on")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolSummary {
    pub name: String,
    pub exposed_name: String,
    #[serde(default)]
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredMcpServer {
    pub source: String,
    pub source_file: String,
    pub config: McpServerConfig,
    pub already_registered: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExternalMcpStore {
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}
