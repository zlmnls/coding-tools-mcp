use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillAvailability {
    Registered,
    DependenciesPending,
    Ready,
    Broken,
}

impl Default for SkillAvailability {
    fn default() -> Self {
        Self::Registered
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SandboxPermission {
    Isolated,
    WorkspaceRead,
    WorkspaceWrite,
}

impl Default for SandboxPermission {
    fn default() -> Self {
        Self::Isolated
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillExecutionMode {
    Local,
    ReadOnly,
    Sandbox,
}

impl Default for SkillExecutionMode {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DependencyKind {
    Node,
    Python,
    System,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub requirement: Option<String>,
    pub kind: DependencyKind,
    pub installed: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSandboxDto {
    pub root: String,
    pub source: String,
    pub runtime: String,
    pub workspace: String,
    pub cache: String,
    pub tmp: String,
    pub logs: String,
    pub state: String,
    pub permission: SandboxPermission,
    pub network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub triggers: Vec<String>,
    pub capabilities: Vec<String>,
    pub requires_execution: bool,
    pub source_path: String,
    pub enabled: bool,
    pub availability: SkillAvailability,
    pub resources: Vec<String>,
    pub dependencies: Vec<SkillDependency>,
    pub sandbox: SkillSandboxDto,
    pub execution_mode: SkillExecutionMode,
    pub allow_execution: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRegistration {
    pub id: String,
    pub source_path: PathBuf,
    pub enabled: bool,
    pub permission: SandboxPermission,
    pub network: bool,
    #[serde(default)]
    pub execution_mode: SkillExecutionMode,
    #[serde(default = "default_true")]
    pub allow_execution: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillStore {
    pub registrations: Vec<SkillRegistration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPlan {
    pub skill_id: String,
    pub commands: Vec<String>,
    pub network: bool,
    pub target: String,
    pub missing_system_dependencies: Vec<String>,
}

/// 终端安装会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSession {
    pub session_id: String,
    pub sandbox_root: String,
    pub workspace: String,
    pub network: bool,
    pub created_at: String,
}

/// 探测到的 Skill 候选
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCandidate {
    pub path: String,
    pub name: String,
    pub description: String,
}
