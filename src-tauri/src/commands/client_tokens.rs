use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::app_state::AppState;
use crate::data::ClientToken;
use crate::error::{AppError, AppResult};
use crate::tools::registry;

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(Serialize)]
pub struct ClientTokenInfo {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub allowed_tools: Vec<String>,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub revoked_at: Option<u64>,
}

impl From<ClientToken> for ClientTokenInfo {
    fn from(value: ClientToken) -> Self {
        Self {
            id: value.id,
            name: value.name,
            scope: value.scope,
            allowed_tools: value.allowed_tools,
            created_at: value.created_at,
            expires_at: value.expires_at,
            revoked_at: value.revoked_at,
        }
    }
}

#[derive(Serialize)]
pub struct NewClientToken {
    pub info: ClientTokenInfo,
    /// Revealed only by the creation response. Never persisted or returned by list.
    pub token: String,
}

#[tauri::command]
pub fn list_client_tokens(
    state: State<'_, AppState>,
    workspace_id: String,
) -> AppResult<Vec<ClientTokenInfo>> {
    state.with_data(|store| {
        if store.get(&workspace_id).is_none() {
            return Err(AppError::Message("workspace not found".into()));
        }
        Ok(store
            .list_client_tokens(&workspace_id)
            .into_iter()
            .map(Into::into)
            .collect())
    })
}

#[tauri::command]
pub fn create_client_token(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
    scope: String,
    expires_in_days: Option<u32>,
    allowed_tools: Vec<String>,
) -> AppResult<NewClientToken> {
    let name = name.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(AppError::Message("Token 名称应为 1–64 个字符".into()));
    }
    if scope != "read_only" && scope != "full" {
        return Err(AppError::Message("无效的 Token 权限".into()));
    }
    if expires_in_days.is_some_and(|days| days == 0 || days > 365) {
        return Err(AppError::Message(
            "有效期必须为 1–365 天，留空表示不过期".into(),
        ));
    }
    if allowed_tools.len() > 46
        || allowed_tools.iter().any(|tool| {
            !registry::is_allowed_tool(tool)
                || (scope == "read_only" && !registry::READ_ONLY_TOOLS.contains(&tool.as_str()))
                || tool == "request_permissions"
        })
    {
        return Err(AppError::Message(
            "工具白名单包含不支持或超出权限的工具".into(),
        ));
    }
    let raw = format!(
        "mcpg_{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let hash = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let now = now_seconds();
    let info = state.with_data(|store| {
        let token = ClientToken {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.clone(),
            hash,
            scope: scope.clone(),
            allowed_tools,
            created_at: now,
            expires_at: expires_in_days.map(|days| now + u64::from(days) * 86_400),
            revoked_at: None,
        };
        store.insert_client_token(&workspace_id, token.clone())?;
        Ok(token.into())
    })?;
    Ok(NewClientToken { info, token: raw })
}

#[tauri::command]
pub fn revoke_client_token(
    state: State<'_, AppState>,
    workspace_id: String,
    token_id: String,
) -> AppResult<()> {
    state.with_data(|store| store.revoke_client_token(&workspace_id, &token_id, now_seconds()))
}
