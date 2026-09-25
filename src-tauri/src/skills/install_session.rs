use super::model::{InstallSession, SkillCandidate};
use super::parser::parse_skill;
use crate::error::AppResult;
use crate::platform::platform;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// 创建待安装 Skill 的临时沙箱
pub fn create_session(network: bool) -> AppResult<InstallSession> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let session_id = format!("pending-{ts}");
    let root = platform()
        .app_config_dir()?
        .join("skill-sandboxes")
        .join(&session_id);
    let workspace = root.join("workspace");
    for dir in [
        &root,
        &workspace,
        &root.join("runtime"),
        &root.join("cache"),
        &root.join("tmp"),
        &root.join("logs"),
        &root.join("state"),
    ] {
        fs::create_dir_all(dir)?;
    }
    Ok(InstallSession {
        session_id: session_id.clone(),
        sandbox_root: root.display().to_string(),
        workspace: workspace.display().to_string(),
        network,
        created_at: session_id.clone(),
    })
}

/// 获取安装会话的沙箱根目录
pub fn session_root(session_id: &str) -> AppResult<PathBuf> {
    let root = platform()
        .app_config_dir()?
        .join("skill-sandboxes")
        .join(session_id);
    if !root.is_dir() {
        return Err(crate::error::AppError::Message(
            "install session not found".into(),
        ));
    }
    Ok(root)
}

/// 在安装会话沙箱内执行终端命令
pub fn run_terminal(
    session_id: &str,
    command: &str,
) -> Result<super::terminal::TerminalResult, String> {
    let root = session_root(session_id).map_err(|e| e.to_string())?;
    let workspace = root.join("workspace");
    let state = root.join("state");
    let runtime = root.join("runtime");
    fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
    fs::create_dir_all(&state).map_err(|e| e.to_string())?;
    if command.trim().is_empty() {
        return Err("command is required".into());
    }
    if command.len() > 4000 {
        return Err("command too long".into());
    }
    let output = std::process::Command::new("/bin/zsh")
        .args(["-lc", command])
        .current_dir(&workspace)
        .env("HOME", &state)
        .env("XDG_CONFIG_HOME", state.join("config"))
        .env("TMPDIR", root.join("tmp"))
        .env(
            "PATH",
            format!(
                "{}:{}",
                runtime.join("bin").display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .output()
        .map_err(|e| format!("failed to start shell: {e}"))?;
    Ok(super::terminal::TerminalResult {
        command: command.to_string(),
        cwd: workspace.display().to_string(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code(),
    })
}

/// 在安装会话沙箱内递归探测 SKILL.md
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".cache",
    "target",
];
const MAX_DEPTH: usize = 5;
const MAX_CANDIDATES: usize = 20;

pub fn detect_candidates(session_id: &str) -> Result<Vec<SkillCandidate>, String> {
    let root = session_root(session_id).map_err(|e| e.to_string())?;
    let workspace = root.join("workspace");
    if !workspace.is_dir() {
        return Ok(Vec::new());
    }
    let mut candidates = Vec::new();
    scan_dir(&workspace, &workspace, 0, &mut candidates);
    // 过滤 memory skill
    candidates.retain(|c| !super::parser::is_memory_skill(Path::new(&c.path)));
    Ok(candidates)
}

fn scan_dir(base: &Path, dir: &Path, depth: usize, candidates: &mut Vec<SkillCandidate>) {
    if depth > MAX_DEPTH || candidates.len() >= MAX_CANDIDATES {
        return;
    }
    let skill_md = dir.join("SKILL.md");
    if skill_md.is_file() {
        if let Ok((name, description, _, _, _, _, _, _, _)) = parse_skill(dir) {
            let rel = dir.strip_prefix(base).unwrap_or(dir).display().to_string();
            candidates.push(SkillCandidate {
                path: rel,
                name,
                description,
            });
            return; // 不再深入已识别为 Skill 的目录
        }
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if SKIP_DIRS.contains(&name_str.as_ref()) {
            continue;
        }
        scan_dir(base, &path, depth + 1, candidates);
        if candidates.len() >= MAX_CANDIDATES {
            return;
        }
    }
}

/// 将探测到的 Skill 从 pending 沙箱迁移到正式沙箱并注册
pub fn register_from_session(
    session_id: &str,
    relative_path: &str,
) -> Result<super::model::SkillMetadata, String> {
    let pending_root = session_root(session_id).map_err(|e| e.to_string())?;
    let skill_src = pending_root.join("workspace").join(relative_path);
    let canonical = skill_src.canonicalize().map_err(|e| e.to_string())?;
    // 路径安全：必须在 pending 沙箱内
    let pending_canonical = pending_root.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(&pending_canonical) {
        return Err("skill path escapes install session sandbox".into());
    }
    if !canonical.join("SKILL.md").is_file() {
        return Err("SKILL.md not found in selected path".into());
    }
    // 解析 Skill 元数据
    let (name, _, _, _, _, _, _, _, _) = parse_skill(&canonical)?;
    let id = name.to_ascii_lowercase().replace(' ', "-");
    // 创建正式沙箱
    let sandbox = super::sandbox::SkillSandbox::create(&id, &canonical, Default::default(), false)
        .map_err(|e| e.to_string())?;
    // 复制 Skill 文件到正式沙箱 source 目录
    copy_dir(&canonical, &sandbox.source).map_err(|e| e.to_string())?;
    // 注册
    let mut store = super::storage::load().map_err(|e| e.to_string())?;
    store.registrations.retain(|r| r.id != id);
    let registration = super::model::SkillRegistration {
        id: id.clone(),
        source_path: sandbox.source.clone(),
        enabled: true,
        permission: Default::default(),
        network: false,
        execution_mode: super::model::SkillExecutionMode::Local,
        allow_execution: true,
    };
    store.registrations.push(registration.clone());
    super::storage::save(&store).map_err(|e| e.to_string())?;
    // 清理 pending 沙箱
    let _ = fs::remove_dir_all(&pending_root);
    super::metadata(&registration)
}

/// 清理安装会话
pub fn cleanup_session(session_id: &str) -> Result<(), String> {
    let root = session_root(session_id).map_err(|e| e.to_string())?;
    fs::remove_dir_all(&root).map_err(|e| e.to_string())
}

/// 递归复制目录
fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            let name = entry.file_name();
            if SKIP_DIRS.contains(&name.to_string_lossy().as_ref()) {
                continue;
            }
            copy_dir(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
