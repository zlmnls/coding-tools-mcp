use crate::error::{AppError, AppResult};
use crate::skills::{
    self,
    model::{InstallPlan, InstallSession, SkillCandidate, SkillMetadata},
};
use std::path::PathBuf;

#[tauri::command]
pub fn list_skills() -> AppResult<Vec<SkillMetadata>> {
    skills::list().map_err(AppError::Message)
}

#[tauri::command]
pub fn discover_skills(
    workspace_path: Option<String>,
    custom_roots: Vec<String>,
) -> AppResult<Vec<crate::skills::discovery::DiscoveredSkill>> {
    let workspace = workspace_path.map(PathBuf::from);
    let roots = custom_roots.into_iter().map(PathBuf::from).collect();
    skills::discover(workspace, roots).map_err(AppError::Message)
}

#[tauri::command]
pub fn list_skill_source_roots(
    workspace_path: Option<String>,
    custom_roots: Vec<String>,
) -> AppResult<Vec<crate::skills::discovery::SkillSourceRoot>> {
    let workspace = workspace_path.map(PathBuf::from);
    let roots = custom_roots
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    Ok(crate::skills::discovery::source_roots(
        workspace.as_deref(),
        &roots,
    ))
}

#[tauri::command]
pub fn register_discovered_skill(path: String) -> AppResult<SkillMetadata> {
    skills::register(PathBuf::from(path)).map_err(AppError::Message)
}

#[tauri::command]
pub fn register_discovered_skills_batch(paths: Vec<String>) -> AppResult<Vec<SkillMetadata>> {
    let path_bufs = paths.into_iter().map(PathBuf::from).collect();
    skills::register_batch(path_bufs).map_err(AppError::Message)
}

#[tauri::command]
pub fn register_skill(path: String) -> AppResult<SkillMetadata> {
    skills::register(PathBuf::from(path)).map_err(AppError::Message)
}

#[tauri::command]
pub fn unregister_skill(id: String) -> AppResult<()> {
    skills::unregister(&id).map_err(AppError::Message)
}

#[tauri::command]
pub fn set_skill_execution(
    id: String,
    mode: skills::model::SkillExecutionMode,
    allow_execution: bool,
) -> AppResult<SkillMetadata> {
    let mut store = skills::storage::load().map_err(|e| AppError::Message(e.to_string()))?;
    let registration = store
        .registrations
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::Message("skill not found".into()))?;
    registration.execution_mode = mode;
    registration.allow_execution = allow_execution;
    let updated = registration.clone();
    skills::storage::save(&store).map_err(|e| AppError::Message(e.to_string()))?;
    skills::metadata(&updated).map_err(AppError::Message)
}

#[tauri::command]
pub fn load_skill(id: String, resource: Option<String>) -> AppResult<serde_json::Value> {
    skills::load_content(&id, resource.as_deref()).map_err(AppError::Message)
}

#[tauri::command]
pub fn run_skill_terminal(
    id: String,
    command: String,
) -> AppResult<crate::skills::terminal::TerminalResult> {
    skills::terminal::run(&id, &command).map_err(AppError::Message)
}

#[tauri::command]
pub fn install_skill_dependencies(id: String) -> AppResult<()> {
    skills::installer::install(&id).map_err(AppError::Message)
}

#[tauri::command]
pub fn skill_install_plan(id: String) -> AppResult<InstallPlan> {
    let meta = skills::list()
        .map_err(AppError::Message)?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::Message("skill not found".into()))?;
    Ok(InstallPlan { skill_id: id, commands: meta.dependencies.iter().filter_map(|dep| match dep.kind { skills::model::DependencyKind::Node => Some("npm install --prefix runtime".into()), skills::model::DependencyKind::Python => Some("python -m venv runtime/python && runtime/python/bin/pip install -r requirements.txt".into()), _ => None }).collect(), network: !meta.dependencies.is_empty(), target: meta.sandbox.runtime, missing_system_dependencies: Vec::new() })
}

#[tauri::command]
pub fn create_skill_install_session() -> AppResult<InstallSession> {
    skills::install_session::create_session(true).map_err(AppError::from)
}

#[tauri::command]
pub fn run_skill_install_terminal(
    session_id: String,
    command: String,
) -> AppResult<crate::skills::terminal::TerminalResult> {
    skills::install_session::run_terminal(&session_id, &command).map_err(AppError::Message)
}

#[tauri::command]
pub fn detect_installed_skills(session_id: String) -> AppResult<Vec<SkillCandidate>> {
    skills::install_session::detect_candidates(&session_id).map_err(AppError::Message)
}

#[tauri::command]
pub fn register_detected_skill(session_id: String, path: String) -> AppResult<SkillMetadata> {
    skills::install_session::register_from_session(&session_id, &path).map_err(AppError::Message)
}

#[tauri::command]
pub fn cancel_skill_install_session(session_id: String) -> AppResult<()> {
    skills::install_session::cleanup_session(&session_id).map_err(AppError::Message)
}
