use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use super::model::{ExternalMcpStore, McpServerConfig};
use crate::error::AppResult;
use crate::platform::platform;

fn path() -> AppResult<PathBuf> {
    Ok(platform()
        .app_config_dir()?
        .join("data")
        .join("external_mcps.json"))
}

pub fn load_store() -> AppResult<ExternalMcpStore> {
    let p = path()?;
    if !p.exists() {
        return Ok(ExternalMcpStore::default());
    }
    let content = fs::read_to_string(p)?;
    Ok(parse_store(&content))
}

fn parse_store(content: &str) -> ExternalMcpStore {
    let raw: Value = match serde_json::from_str(content) {
        Ok(value) => value,
        Err(_) => return ExternalMcpStore::default(),
    };

    let legacy_modes: std::collections::HashMap<String, String> = raw
        .get("servers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("usage_mode").is_none())
        .filter_map(|item| {
            let id = item.get("id")?.as_str()?.to_string();
            let instructions = item
                .get("instructions")
                .and_then(Value::as_str)
                .unwrap_or("");
            let mode = if instructions.trim().is_empty() {
                "on_demand"
            } else {
                "always_on"
            };
            Some((id, mode.to_string()))
        })
        .collect();

    let mut store: ExternalMcpStore = serde_json::from_value(raw).unwrap_or_default();
    for server in &mut store.servers {
        if let Some(mode) = legacy_modes.get(&server.id) {
            server.usage_mode = mode.clone();
        }
    }
    store
}

pub fn save_store(store: &ExternalMcpStore) -> AppResult<()> {
    let p = path()?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(p, format!("{}\n", serde_json::to_string_pretty(store)?))?;
    Ok(())
}

pub fn list_servers() -> AppResult<Vec<McpServerConfig>> {
    let store = load_store()?;
    Ok(store.servers)
}

pub fn get_server(id: &str) -> AppResult<Option<McpServerConfig>> {
    let store = load_store()?;
    Ok(store.servers.into_iter().find(|s| s.id == id))
}

pub fn upsert_server(mut config: McpServerConfig) -> AppResult<()> {
    let mut store = load_store()?;
    config.usage_mode = if config.usage_mode.eq_ignore_ascii_case("always_on") {
        "always_on".to_string()
    } else {
        "on_demand".to_string()
    };
    let now = chrono_now();
    if config.created_at.is_empty() {
        config.created_at = now.clone();
    }
    config.updated_at = now;

    if let Some(existing) = store.servers.iter_mut().find(|s| s.id == config.id) {
        *existing = config;
    } else {
        store.servers.push(config);
    }
    save_store(&store)
}

pub fn delete_server(id: &str) -> AppResult<()> {
    let mut store = load_store()?;
    store.servers.retain(|s| s.id != id);
    save_store(&store)
}

pub fn set_server_enabled(id: &str, enabled: bool) -> AppResult<()> {
    let mut store = load_store()?;
    if let Some(server) = store.servers.iter_mut().find(|s| s.id == id) {
        server.enabled = enabled;
        server.updated_at = chrono_now();
        save_store(&store)?;
    }
    Ok(())
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::parse_store;

    #[test]
    fn legacy_server_with_instructions_keeps_always_on_behavior() {
        let store = parse_store(
            r#"{"servers":[{"id":"mem0","name":"Mem0","instructions":"must search","command":"mem0"}]}"#,
        );
        assert_eq!(store.servers[0].usage_mode, "always_on");
    }

    #[test]
    fn legacy_server_without_instructions_defaults_to_on_demand() {
        let store =
            parse_store(r#"{"servers":[{"id":"lark","name":"Lark","command":"lark-mcp"}]}"#);
        assert_eq!(store.servers[0].usage_mode, "on_demand");
    }

    #[test]
    fn explicit_usage_mode_is_not_overridden() {
        let store = parse_store(
            r#"{"servers":[{"id":"mem0","name":"Mem0","instructions":"must search","usage_mode":"on_demand","command":"mem0"}]}"#,
        );
        assert_eq!(store.servers[0].usage_mode, "on_demand");
    }
}
