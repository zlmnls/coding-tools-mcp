use super::model::{SandboxPermission, SkillSandboxDto};
use crate::error::AppResult;
use crate::platform::platform;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SkillSandbox {
    pub root: PathBuf,
    pub source: PathBuf,
    pub runtime: PathBuf,
    pub workspace: PathBuf,
    pub cache: PathBuf,
    pub tmp: PathBuf,
    pub logs: PathBuf,
    pub state: PathBuf,
    pub permission: SandboxPermission,
    pub network: bool,
}

impl SkillSandbox {
    pub fn create(
        id: &str,
        source: &Path,
        permission: SandboxPermission,
        network: bool,
    ) -> AppResult<Self> {
        let root = platform()
            .app_config_dir()?
            .join("skill-sandboxes")
            .join(id);
        let sandbox = Self {
            source: root.join("source"),
            runtime: root.join("runtime"),
            workspace: root.join("workspace"),
            cache: root.join("cache"),
            tmp: root.join("tmp"),
            logs: root.join("logs"),
            state: root.join("state"),
            root,
            permission,
            network,
        };
        for path in [
            &sandbox.root,
            &sandbox.source,
            &sandbox.runtime,
            &sandbox.workspace,
            &sandbox.cache,
            &sandbox.tmp,
            &sandbox.logs,
            &sandbox.state,
        ] {
            fs::create_dir_all(path)?;
        }
        let _ = source;
        Ok(sandbox)
    }
    pub fn contains(&self, path: &Path) -> bool {
        path.canonicalize()
            .ok()
            .map(|p| {
                p.starts_with(
                    self.root
                        .canonicalize()
                        .unwrap_or_else(|_| self.root.clone()),
                )
            })
            .unwrap_or(false)
    }
    pub fn dto(&self) -> SkillSandboxDto {
        SkillSandboxDto {
            root: self.root.display().to_string(),
            source: self.source.display().to_string(),
            runtime: self.runtime.display().to_string(),
            workspace: self.workspace.display().to_string(),
            cache: self.cache.display().to_string(),
            tmp: self.tmp.display().to_string(),
            logs: self.logs.display().to_string(),
            state: self.state.display().to_string(),
            permission: self.permission.clone(),
            network: self.network,
        }
    }
}
