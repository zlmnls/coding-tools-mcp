use super::model::SkillStore;
use crate::error::AppResult;
use crate::platform::platform;
use std::fs;

fn path() -> AppResult<std::path::PathBuf> {
    Ok(platform()
        .app_config_dir()?
        .join("data")
        .join("skills.json"))
}
pub fn load() -> AppResult<SkillStore> {
    let p = path()?;
    if !p.exists() {
        return Ok(SkillStore::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(p)?).unwrap_or_default())
}
pub fn save(store: &SkillStore) -> AppResult<()> {
    let p = path()?;
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(p, format!("{}\n", serde_json::to_string_pretty(store)?))?;
    Ok(())
}
