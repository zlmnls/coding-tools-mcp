pub mod discovery;
pub mod install_session;
pub mod installer;
pub mod model;
pub mod parser;
pub mod sandbox;
pub mod storage;
pub mod terminal;

use model::*;
use parser::{is_memory_skill, parse_skill};
use sandbox::SkillSandbox;
use std::fs;
use std::path::{Path, PathBuf};

pub fn metadata(reg: &SkillRegistration) -> Result<SkillMetadata, String> {
    let source = Path::new(&reg.source_path);
    let (
        name,
        description,
        _,
        tags,
        resources,
        mut dependencies,
        triggers,
        capabilities,
        requires_execution,
    ) = parse_skill(source)?;
    let sandbox = SkillSandbox::create(&reg.id, source, reg.permission.clone(), reg.network)
        .map_err(|e| e.to_string())?;
    for dep in &mut dependencies {
        dep.installed = dependency_available(&sandbox, dep);
    }
    let availability = if dependencies.iter().any(|d| !d.installed) {
        SkillAvailability::DependenciesPending
    } else {
        SkillAvailability::Ready
    };
    Ok(SkillMetadata {
        id: reg.id.clone(),
        name,
        description,
        tags,
        triggers,
        capabilities,
        requires_execution,
        source_path: source.display().to_string(),
        enabled: reg.enabled,
        availability,
        resources,
        dependencies,
        sandbox: sandbox.dto(),
        execution_mode: reg.execution_mode.clone(),
        allow_execution: reg.allow_execution,
    })
}

fn dependency_available(sandbox: &SkillSandbox, dep: &SkillDependency) -> bool {
    match dep.kind {
        DependencyKind::Node => sandbox.runtime.join("node_modules").exists(),
        DependencyKind::Python => sandbox.runtime.join("python").exists(),
        _ => true,
    }
}

pub fn list() -> Result<Vec<SkillMetadata>, String> {
    let store = storage::load().map_err(|e| e.to_string())?;
    Ok(store
        .registrations
        .iter()
        .filter_map(|r| metadata(r).ok())
        .collect::<Vec<_>>())
}

pub fn discover(
    workspace: Option<PathBuf>,
    custom_roots: Vec<PathBuf>,
) -> Result<Vec<discovery::DiscoveredSkill>, String> {
    let store = storage::load().map_err(|e| e.to_string())?;
    let registered_paths = store
        .registrations
        .iter()
        .filter_map(|item| {
            item.source_path
                .canonicalize()
                .ok()
                .map(|path| path.display().to_string())
        })
        .collect();
    Ok(discovery::discover(
        workspace.as_deref(),
        &custom_roots,
        &registered_paths,
    ))
}

pub fn register(path: PathBuf) -> Result<SkillMetadata, String> {
    if is_memory_skill(&path) {
        return Err("memory Skill is temporarily disabled".into());
    }
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    if !path.join("SKILL.md").is_file() {
        return Err("SKILL.md is required".into());
    }
    let (name, _, _, _, _, _, _, _, _) = parse_skill(&path)?;
    let id = name.to_ascii_lowercase().replace(' ', "-");
    let mut store = storage::load().map_err(|e| e.to_string())?;
    store.registrations.retain(|r| r.id != id);
    let registration = SkillRegistration {
        id: id.clone(),
        source_path: path,
        enabled: true,
        permission: SandboxPermission::Isolated,
        network: false,
        execution_mode: SkillExecutionMode::Local,
        allow_execution: true,
    };
    store.registrations.push(registration.clone());
    storage::save(&store).map_err(|e| e.to_string())?;
    metadata(&registration)
}

pub fn register_batch(paths: Vec<PathBuf>) -> Result<Vec<SkillMetadata>, String> {
    let mut store = storage::load().map_err(|e| e.to_string())?;
    let mut registered = Vec::new();
    for p in paths {
        if is_memory_skill(&p) {
            continue;
        }
        let Ok(canon) = p.canonicalize() else {
            continue;
        };
        if !canon.join("SKILL.md").is_file() {
            continue;
        }
        let Ok((name, _, _, _, _, _, _, _, _)) = parse_skill(&canon) else {
            continue;
        };
        let id = name.to_ascii_lowercase().replace(' ', "-");
        store.registrations.retain(|r| r.id != id);
        let registration = SkillRegistration {
            id: id.clone(),
            source_path: canon,
            enabled: true,
            permission: SandboxPermission::Isolated,
            network: false,
            execution_mode: SkillExecutionMode::Local,
            allow_execution: true,
        };
        store.registrations.push(registration.clone());
        if let Ok(meta) = metadata(&registration) {
            registered.push(meta);
        }
    }
    storage::save(&store).map_err(|e| e.to_string())?;
    Ok(registered)
}

pub fn unregister(id: &str) -> Result<(), String> {
    let mut store = storage::load().map_err(|e| e.to_string())?;
    store.registrations.retain(|r| r.id != id);
    storage::save(&store).map_err(|e| e.to_string())
}
pub fn load_content(id: &str, resource: Option<&str>) -> Result<serde_json::Value, String> {
    let store = storage::load().map_err(|e| e.to_string())?;
    let reg = store
        .registrations
        .iter()
        .find(|r| r.id == id)
        .ok_or("skill not found")?;
    if !reg.enabled {
        return Err("skill is disabled".into());
    }
    let meta = metadata(reg)?;
    let root = Path::new(&reg.source_path);
    let path = match resource {
        Some(value) => root.join(value),
        None => root.join("SKILL.md"),
    };
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(root.canonicalize().map_err(|e| e.to_string())?) {
        return Err("resource escapes skill root".into());
    }
    Ok(
        serde_json::json!({"skill_id": id, "content": fs::read_to_string(canonical).map_err(|e| e.to_string())?, "resources": meta.resources, "availability": meta.availability, "can_execute": meta.enabled && meta.allow_execution && meta.availability == SkillAvailability::Ready, "dependencies": meta.dependencies}),
    )
}
