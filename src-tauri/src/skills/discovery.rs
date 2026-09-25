use super::model::SkillCandidate;
use super::parser::{is_memory_skill, parse_skill};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillSourceScope {
    Project,
    User,
    System,
    Custom,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SkillSourceRoot {
    pub scope: SkillSourceScope,
    pub label: String,
    pub path: String,
    pub exists: bool,
    pub readable: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DiscoveredSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub scope: SkillSourceScope,
    pub source_label: String,
    pub registered: bool,
    pub conflict_rank: u8,
    pub conflict: bool,
}

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
    "typeshed-fallback",
    "vendor",
];
const MAX_DEPTH: usize = 12;
const MAX_CANDIDATES: usize = 300;

pub fn source_roots(workspace: Option<&Path>, custom_roots: &[PathBuf]) -> Vec<SkillSourceRoot> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut roots = Vec::new();
    let mut add = |scope: SkillSourceScope, label: &str, path: PathBuf| {
        let path_string = path.display().to_string();
        if roots
            .iter()
            .any(|root: &SkillSourceRoot| root.path == path_string)
        {
            return;
        }
        roots.push(SkillSourceRoot {
            scope,
            label: label.to_string(),
            exists: path.is_dir(),
            readable: fs::read_dir(&path).is_ok(),
            path: path_string,
        });
    };
    for (label, path) in [
        ("Trae 用户级", home.join(".trae/skills")),
        ("Trae CN 用户级", home.join(".trae-cn/skills")),
        ("Trae CN 内置 Skill", home.join(".trae-cn/builtin_skills")),
        ("Trae CN 内置运行时", home.join(".trae-cn/builtin")),
        ("Trae CN 插件", home.join(".trae-cn/plugins")),
        ("Trae CN 扩展", home.join(".trae-cn/extensions")),
        ("Trae 用户内置 Skill", home.join(".trae/builtin_skills")),
        ("Trae 用户内置运行时", home.join(".trae/builtin")),
        ("Trae 扩展", home.join(".trae/extensions")),
        ("Codex 用户级", home.join(".codex/skills")),
        ("Agents 用户级", home.join(".agents/skills")),
        ("Claude 用户级", home.join(".claude/skills")),
        ("Cursor 用户级", home.join(".cursor/skills")),
    ] {
        add(SkillSourceScope::User, label, path);
    }
    for (label, path) in [
        (
            "Trae 系统级",
            PathBuf::from("/Library/Application Support/Trae/skills"),
        ),
        (
            "Codex 系统级",
            PathBuf::from("/Library/Application Support/Codex/skills"),
        ),
        (
            "Trae 共享目录",
            PathBuf::from("/usr/local/share/trae/skills"),
        ),
        (
            "Codex 共享目录",
            PathBuf::from("/usr/local/share/codex/skills"),
        ),
        (
            "Homebrew Trae",
            PathBuf::from("/opt/homebrew/share/trae/skills"),
        ),
        (
            "Homebrew Codex",
            PathBuf::from("/opt/homebrew/share/codex/skills"),
        ),
    ] {
        add(SkillSourceScope::System, label, path);
    }
    if let Some(workspace) = workspace {
        for dir in [
            ".agents/skills",
            ".claude/skills",
            ".codex/skills",
            ".cursor/skills",
        ] {
            add(
                SkillSourceScope::Project,
                &format!("项目 {dir}"),
                workspace.join(dir),
            );
        }
    }
    for (index, path) in custom_roots.iter().enumerate() {
        if let Ok(path) = path.canonicalize() {
            add(
                SkillSourceScope::Custom,
                &format!("自定义目录 {}", index + 1),
                path,
            );
        }
    }
    roots
}

pub fn discover(
    workspace: Option<&Path>,
    custom_roots: &[PathBuf],
    registered_paths: &HashSet<String>,
) -> Vec<DiscoveredSkill> {
    let roots = source_roots(workspace, custom_roots);
    let mut found = Vec::new();
    let mut seen_paths = HashSet::new();
    for root in roots.iter().filter(|root| root.exists && root.readable) {
        let root_path = Path::new(&root.path);
        scan(
            root_path,
            root_path,
            0,
            root,
            registered_paths,
            &mut found,
            &mut seen_paths,
        );
        if found.len() >= MAX_CANDIDATES {
            break;
        }
    }
    let mut counts = HashMap::<String, usize>::new();
    for item in &found {
        *counts.entry(item.id.clone()).or_default() += 1;
    }
    for item in &mut found {
        item.conflict = counts.get(&item.id).copied().unwrap_or(0) > 1;
    }
    found.sort_by_key(|item| (item.conflict_rank, item.name.to_ascii_lowercase()));
    found
}

fn scan(
    base: &Path,
    dir: &Path,
    depth: usize,
    root: &SkillSourceRoot,
    registered_paths: &HashSet<String>,
    found: &mut Vec<DiscoveredSkill>,
    seen_paths: &mut HashSet<String>,
) {
    if depth > MAX_DEPTH || found.len() >= MAX_CANDIDATES {
        return;
    }
    let skill_md = dir.join("SKILL.md");
    if skill_md.is_file() && !is_memory_skill(dir) {
        if let Ok((name, description, _, _, _, _, _, _, _)) = parse_skill(dir) {
            let path = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
            let path_string = path.display().to_string();
            if !seen_paths.insert(path_string.clone()) {
                return;
            }
            let id = name.to_ascii_lowercase().replace(' ', "-");
            let scope = root.scope.clone();
            found.push(DiscoveredSkill {
                id,
                name,
                description,
                path: path_string.clone(),
                scope,
                source_label: root.label.clone(),
                registered: registered_paths.contains(&path_string),
                conflict_rank: rank(&root.scope),
                conflict: false,
            });
        }
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_type().map(|kind| !kind.is_dir()).unwrap_or(true) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        scan(
            base,
            &entry.path(),
            depth + 1,
            root,
            registered_paths,
            found,
            seen_paths,
        );
        if found.len() >= MAX_CANDIDATES {
            break;
        }
    }
    let _ = base;
}

fn rank(scope: &SkillSourceScope) -> u8 {
    match scope {
        SkillSourceScope::Project => 0,
        SkillSourceScope::User => 1,
        SkillSourceScope::System => 2,
        SkillSourceScope::Custom => 3,
    }
}

pub fn candidate_as_skill(candidate: &DiscoveredSkill) -> SkillCandidate {
    SkillCandidate {
        path: candidate.path.clone(),
        name: candidate.name.clone(),
        description: candidate.description.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_has_higher_rank_than_system() {
        assert!(rank(&SkillSourceScope::Project) < rank(&SkillSourceScope::System));
    }
}
