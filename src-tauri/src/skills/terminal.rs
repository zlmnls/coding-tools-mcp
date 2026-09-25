use super::model::{SkillExecutionMode, SkillMetadata};
use super::{list, sandbox::SkillSandbox};
use std::process::Command;

fn host_path() -> String {
    #[cfg(target_os = "macos")]
    if let Some(path) = crate::tools::exec::macos_interactive_shell_path() {
        return path.to_string();
    }

    std::env::var("PATH").unwrap_or_default()
}

#[derive(Debug, serde::Serialize)]
pub struct TerminalResult {
    pub command: String,
    pub cwd: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

pub fn run(id: &str, command: &str) -> Result<TerminalResult, String> {
    let meta: SkillMetadata = list()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or("skill not found")?;
    if !meta.enabled || !meta.allow_execution {
        return Err("skill execution is disabled".into());
    }
    if meta.execution_mode == SkillExecutionMode::ReadOnly {
        return Err("skill is registered as read-only".into());
    }
    if command.trim().is_empty() {
        return Err("command is required".into());
    }
    if command.len() > 4000 {
        return Err("command too long".into());
    }
    let cwd = std::path::Path::new(&meta.source_path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let host_path = host_path();
    let output = match meta.execution_mode {
        SkillExecutionMode::Local => Command::new("/bin/zsh")
            .args(["-lc", command])
            .current_dir(&cwd)
            .env("PATH", &host_path)
            .output(),
        SkillExecutionMode::Sandbox => {
            let sandbox = SkillSandbox::create(id, &cwd, Default::default(), meta.sandbox.network)
                .map_err(|e| e.to_string())?;
            Command::new("/bin/zsh")
                .args(["-lc", command])
                .current_dir(&sandbox.workspace)
                .env("HOME", &sandbox.state)
                .env("XDG_CONFIG_HOME", sandbox.state.join("config"))
                .env("TMPDIR", sandbox.tmp)
                .env(
                    "PATH",
                    format!("{}:{}", sandbox.runtime.join("bin").display(), host_path),
                )
                .output()
        }
        SkillExecutionMode::ReadOnly => unreachable!(),
    }
    .map_err(|e| format!("failed to start shell: {e}"))?;
    Ok(TerminalResult {
        command: command.to_string(),
        cwd: cwd.display().to_string(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code(),
    })
}

pub fn run_local_for_mcp(id: &str, command: &str) -> Result<TerminalResult, String> {
    run(id, command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_path_is_available() {
        assert!(!host_path().trim().is_empty());
    }
}
