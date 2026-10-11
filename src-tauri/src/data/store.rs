use std::sync::Mutex;

use crate::error::{AppError, AppResult};
use crate::settings::AppSettings;
use crate::workspace::legacy_import::import_legacy_profiles_if_empty;
use crate::workspace::WorkspaceProfile;

use super::migrate::{data_file_path, load_or_migrate, maybe_backup_legacy_files, save};
use super::model::AppData;
use super::model::ClientToken;

static DATA_FILE_LOCK: Mutex<()> = Mutex::new(());

const SHARED_KEYS: &[&str] = &[
    "oauth_client_id",
    "bearer_token",
    "oauth_client_secret",
    "oauth_password",
    "oauth_token_secret",
    "actions_api_key",
    "actions_oauth_client_secret",
    "actions_oauth_password",
    "actions_oauth_token_secret",
];

#[derive(Debug)]
pub struct DataStore {
    data: AppData,
}

/// A revoked token must not remain in the credential store: it cannot be reauthorized.
fn remove_client_token(
    by_workspace: &mut std::collections::HashMap<String, Vec<ClientToken>>,
    workspace_id: &str,
    token_id: &str,
) -> bool {
    let Some(tokens) = by_workspace.get_mut(workspace_id) else {
        return false;
    };
    let original_count = tokens.len();
    tokens.retain(|token| token.id != token_id);
    let removed = tokens.len() != original_count;
    if tokens.is_empty() {
        by_workspace.remove(workspace_id);
    }
    removed
}

/// Migrate older persisted data that retained revoked clients.
fn purge_revoked_client_tokens(
    by_workspace: &mut std::collections::HashMap<String, Vec<ClientToken>>,
) -> bool {
    let mut changed = false;
    by_workspace.retain(|_, tokens| {
        let before = tokens.len();
        tokens.retain(|token| token.revoked_at.is_none());
        if tokens.len() != before || tokens.is_empty() {
            changed = true;
        }
        !tokens.is_empty()
    });
    changed
}

impl DataStore {
    pub fn load() -> AppResult<Self> {
        let _guard = lock_data_file()?;
        let path = data_file_path()?;
        let existed_before = path.exists();
        let mut data = load_or_migrate()?;
        let imported = import_legacy_profiles_if_empty(&mut data)?;
        // Tokens revoked by older releases were kept as historical rows.
        // There is no reauthorization flow, so remove these obsolete rows on upgrade.
        let had_revoked_tokens = purge_revoked_client_tokens(&mut data.client_tokens);
        let store = Self { data };
        if !existed_before || imported > 0 || had_revoked_tokens {
            store.persist_unlocked()?;
        }
        if !existed_before {
            maybe_backup_legacy_files(&path)?;
        }
        Ok(store)
    }

    pub fn list_client_tokens(&self, workspace_id: &str) -> Vec<ClientToken> {
        self.data
            .client_tokens
            .get(workspace_id)
            .map(|tokens| {
                tokens
                    .iter()
                    .filter(|token| token.revoked_at.is_none())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn insert_client_token(&mut self, workspace_id: &str, token: ClientToken) -> AppResult<()> {
        if self.get(workspace_id).is_none() {
            return Err(AppError::Message("workspace not found".into()));
        }
        let items = self
            .data
            .client_tokens
            .entry(workspace_id.to_owned())
            .or_default();
        if items
            .iter()
            .filter(|item| item.revoked_at.is_none())
            .count()
            >= 32
        {
            return Err(AppError::Message(
                "当前工作区最多允许 32 个有效 Token".into(),
            ));
        }
        items.push(token);
        self.save()
    }

    pub fn revoke_client_token(&mut self, workspace_id: &str, token_id: &str) -> AppResult<()> {
        if !remove_client_token(&mut self.data.client_tokens, workspace_id, token_id) {
            return Err(AppError::Message("Token 不存在".into()));
        }
        self.save()
    }

    pub fn read_file<R>(f: impl FnOnce(&AppData) -> AppResult<R>) -> AppResult<R> {
        let _guard = lock_data_file()?;
        let data = load_or_migrate()?;
        f(&data)
    }

    pub fn update_file<R>(f: impl FnOnce(&mut AppData) -> AppResult<R>) -> AppResult<R> {
        let _guard = lock_data_file()?;
        let mut data = load_or_migrate()?;
        let result = f(&mut data)?;
        save(&data)?;
        Ok(result)
    }

    pub fn data(&self) -> &AppData {
        &self.data
    }

    pub fn save(&self) -> AppResult<()> {
        let _guard = lock_data_file()?;
        self.persist_unlocked()
    }

    fn persist_unlocked(&self) -> AppResult<()> {
        save(&self.data)
    }

    pub fn settings(&self) -> AppSettings {
        AppSettings::from_data(&self.data)
    }

    pub fn update_settings(&mut self, settings: AppSettings) -> AppResult<()> {
        settings.apply_to(&mut self.data);
        self.save()
    }

    pub fn list(&self) -> &[WorkspaceProfile] {
        &self.data.profiles
    }

    pub fn get(&self, id: &str) -> Option<&WorkspaceProfile> {
        self.data.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn add(&mut self, profile: WorkspaceProfile) -> AppResult<()> {
        self.data.profiles.push(profile);
        self.save()
    }

    pub fn update(&mut self, profile: WorkspaceProfile) -> AppResult<()> {
        let Some(index) = self
            .data
            .profiles
            .iter()
            .position(|item| item.id == profile.id)
        else {
            return Err(AppError::Message(format!(
                "workspace not found: {}",
                profile.id
            )));
        };
        self.data.profiles[index] = profile;
        self.save()
    }

    pub fn remove(&mut self, id: &str) -> AppResult<Option<WorkspaceProfile>> {
        let Some(index) = self.data.profiles.iter().position(|item| item.id == id) else {
            return Ok(None);
        };
        let removed = self.data.profiles.remove(index);
        self.data.workspace_secrets.remove(id);
        self.data.client_tokens.remove(id);
        self.save()?;
        Ok(Some(removed))
    }

    pub fn init_workspace_secrets(&mut self, profile_id: &str) -> AppResult<()> {
        // oauth_client_secret is optional for MCP OAuth (ChatGPT PKCE); not auto-generated.
        self.set_workspace_secret(profile_id, "oauth_password", &random_secret())?;
        self.set_workspace_secret(profile_id, "oauth_token_secret", &random_secret())?;
        self.set_workspace_secret(profile_id, "bearer_token", &random_secret())?;
        self.set_workspace_secret(profile_id, "actions_api_key", &random_secret())?;
        self.set_workspace_secret(profile_id, "actions_oauth_client_secret", &random_secret())?;
        self.set_workspace_secret(profile_id, "actions_oauth_password", &random_secret())?;
        self.set_workspace_secret(profile_id, "actions_oauth_token_secret", &random_secret())?;
        Ok(())
    }

    pub fn init_shared_secrets(&mut self) -> AppResult<()> {
        let mut changed = false;
        for key in SHARED_KEYS {
            if !self.data.shared_secrets.contains_key(*key) {
                self.data
                    .shared_secrets
                    .insert(key.to_string(), shared_value_for_key(key));
                changed = true;
            }
        }
        if changed {
            self.save()?;
        }
        Ok(())
    }

    pub fn get_workspace_secret(&self, profile_id: &str, key: &str) -> AppResult<Option<String>> {
        Ok(self
            .data
            .workspace_secrets
            .get(profile_id)
            .and_then(|secrets| secrets.get(key))
            .filter(|value| !value.is_empty())
            .cloned())
    }

    pub fn set_workspace_secret(
        &mut self,
        profile_id: &str,
        key: &str,
        value: &str,
    ) -> AppResult<()> {
        self.data
            .workspace_secrets
            .entry(profile_id.to_string())
            .or_default()
            .insert(key.to_string(), value.to_string());
        self.save()
    }

    pub fn regenerate_workspace_secret(
        &mut self,
        profile_id: &str,
        key: &str,
    ) -> AppResult<String> {
        let value = shared_value_for_key(key);
        self.set_workspace_secret(profile_id, key, &value)?;
        Ok(value)
    }

    pub fn remove_workspace_secrets(&mut self, profile_id: &str) -> AppResult<()> {
        self.data.workspace_secrets.remove(profile_id);
        self.save()
    }

    pub fn get_shared_secret(&self, key: &str) -> Option<String> {
        self.data.shared_secrets.get(key).cloned()
    }

    pub fn set_shared_secret(&mut self, key: &str, value: &str) -> AppResult<()> {
        self.data
            .shared_secrets
            .insert(key.to_string(), value.to_string());
        self.save()
    }

    pub fn regenerate_shared_secret(&mut self, key: &str) -> AppResult<String> {
        let value = random_secret();
        self.set_shared_secret(key, &value)?;
        Ok(value)
    }

    pub fn get_app_secret(&self, scope: &str, item_id: &str) -> Option<String> {
        self.data
            .app_secrets
            .get(scope)
            .and_then(|items| items.get(item_id))
            .filter(|value| !value.is_empty())
            .cloned()
    }

    pub fn set_app_secret(&mut self, scope: &str, item_id: &str, value: &str) -> AppResult<()> {
        self.data
            .app_secrets
            .entry(scope.to_string())
            .or_default()
            .insert(item_id.to_string(), value.to_string());
        self.save()
    }

    pub fn delete_app_secret(&mut self, scope: &str, item_id: &str) -> AppResult<()> {
        if let Some(items) = self.data.app_secrets.get_mut(scope) {
            items.remove(item_id);
            if items.is_empty() {
                self.data.app_secrets.remove(scope);
            }
        }
        self.save()
    }
}

fn lock_data_file() -> AppResult<std::sync::MutexGuard<'static, ()>> {
    DATA_FILE_LOCK
        .lock()
        .map_err(|_| AppError::Message("data file lock poisoned".into()))
}

fn random_secret() -> String {
    format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4()).replace('-', "")
}

fn shared_value_for_key(key: &str) -> String {
    if key == "oauth_client_id" {
        format!("chatgpt-client-{}", &uuid::Uuid::new_v4().to_string()[..12])
    } else {
        random_secret()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_client_token(id: &str, revoked_at: Option<u64>) -> ClientToken {
        ClientToken {
            id: id.to_owned(),
            name: id.to_owned(),
            hash: format!("hash-{id}"),
            scope: "read_only".to_owned(),
            allowed_tools: vec![],
            created_at: 1,
            expires_at: None,
            revoked_at,
        }
    }

    #[test]
    fn revoking_deletes_only_the_selected_client_and_empty_workspace_bucket() {
        let mut buckets = std::collections::HashMap::from([
            (
                "workspace-a".to_owned(),
                vec![
                    test_client_token("alice", None),
                    test_client_token("bob", None),
                ],
            ),
            (
                "workspace-b".to_owned(),
                vec![test_client_token("other", None)],
            ),
        ]);

        assert!(!remove_client_token(&mut buckets, "workspace-a", "missing"));
        assert!(remove_client_token(&mut buckets, "workspace-a", "alice"));
        assert_eq!(buckets["workspace-a"].len(), 1);
        assert_eq!(buckets["workspace-a"][0].id, "bob");
        assert_eq!(buckets["workspace-b"].len(), 1);

        assert!(remove_client_token(&mut buckets, "workspace-a", "bob"));
        assert!(!buckets.contains_key("workspace-a"));
        assert!(!remove_client_token(&mut buckets, "workspace-a", "bob"));
    }

    #[test]
    fn old_revoked_rows_are_purged_and_hidden_on_upgrade() {
        let mut store = DataStore {
            data: AppData::default(),
        };
        store.data.client_tokens.insert(
            "workspace-a".to_owned(),
            vec![
                test_client_token("active", None),
                test_client_token("old-revoked", Some(100)),
            ],
        );
        store.data.client_tokens.insert(
            "workspace-b".to_owned(),
            vec![test_client_token("revoked-only", Some(100))],
        );

        assert_eq!(store.list_client_tokens("workspace-a").len(), 1);
        assert!(store.list_client_tokens("workspace-b").is_empty());
        assert!(purge_revoked_client_tokens(&mut store.data.client_tokens));
        assert_eq!(store.data.client_tokens["workspace-a"].len(), 1);
        assert!(!store.data.client_tokens.contains_key("workspace-b"));
        assert!(!purge_revoked_client_tokens(&mut store.data.client_tokens));
    }

    #[test]
    fn workspace_secret_roundtrip() {
        let id = uuid::Uuid::new_v4().to_string().replace('-', "");
        let mut store = DataStore::load().expect("load");
        store
            .set_workspace_secret(&id, "oauth_client_secret", "roundtrip-secret")
            .expect("set");
        let loaded = store
            .get_workspace_secret(&id, "oauth_client_secret")
            .expect("get");
        assert_eq!(loaded.as_deref(), Some("roundtrip-secret"));
        store.remove_workspace_secrets(&id).expect("remove");
    }

    #[test]
    fn shared_oauth_client_id_uses_client_id_format() {
        let value = shared_value_for_key("oauth_client_id");
        assert!(value.starts_with("chatgpt-client-"));
        assert_eq!(value.len(), "chatgpt-client-".len() + 12);
    }
}
