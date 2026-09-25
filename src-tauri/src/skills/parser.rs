use super::model::{DependencyKind, SkillDependency};
use std::fs;
use std::path::Path;

pub fn parse_skill(
    path: &Path,
) -> Result<
    (
        String,
        String,
        String,
        Vec<String>,
        Vec<String>,
        Vec<SkillDependency>,
        Vec<String>,
        Vec<String>,
        bool,
    ),
    String,
> {
    let file = path.join("SKILL.md");
    let text = fs::read_to_string(&file).map_err(|e| format!("cannot read SKILL.md: {e}"))?;
    let mut name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("skill")
        .to_string();
    let mut description = String::new();
    let mut tags = Vec::new();
    let mut triggers = Vec::new();
    let mut capabilities = Vec::new();
    let mut requires_execution = false;
    let mut section = "";
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("name:") {
            name = value.trim().trim_matches('"').to_string();
        }
        if let Some(value) = trimmed.strip_prefix("description:") {
            description = value.trim().trim_matches('"').to_string();
        }
        if let Some(value) = trimmed.strip_prefix("requires_execution:") {
            requires_execution = value.trim().eq_ignore_ascii_case("true");
        }
        if trimmed.ends_with(':') {
            section = trimmed.trim_end_matches(':');
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("- ") {
            let value = value.trim().trim_matches('"').to_string();
            if section.eq_ignore_ascii_case("triggers") && triggers.len() < 16 {
                triggers.push(value.clone());
            } else if section.eq_ignore_ascii_case("capabilities") && capabilities.len() < 16 {
                capabilities.push(value.clone());
            } else if tags.len() < 8 && !value.contains(':') {
                tags.push(value);
            }
        }
        if description.is_empty() && trimmed.starts_with('#') {
            description = trimmed.trim_start_matches('#').trim().to_string();
        }
    }
    if description.is_empty() {
        description = text
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_string();
    }
    let resources = collect_resources(path)?;
    if triggers.is_empty() {
        triggers = tags.clone();
    }
    if capabilities.is_empty() && !description.is_empty() {
        capabilities.push(description.clone());
    }
    let dependencies = detect_dependencies(path);
    Ok((
        name,
        description.chars().take(300).collect(),
        text,
        tags,
        resources,
        dependencies,
        triggers,
        capabilities,
        requires_execution,
    ))
}

fn collect_resources(root: &Path) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    for dir in ["references", "examples", "templates", "assets"] {
        let path = root.join(dir);
        if !path.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            if let Ok(relative) = entry.path().strip_prefix(root) {
                result.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    result.sort();
    Ok(result)
}

fn detect_dependencies(root: &Path) -> Vec<SkillDependency> {
    let mut deps = Vec::new();
    if root.join("package.json").exists() {
        deps.push(SkillDependency {
            name: "node dependencies".into(),
            requirement: Some("package.json".into()),
            kind: DependencyKind::Node,
            installed: false,
            detail: None,
        });
    }
    if root.join("requirements.txt").exists() || root.join("pyproject.toml").exists() {
        deps.push(SkillDependency {
            name: "python dependencies".into(),
            requirement: Some("requirements manifest".into()),
            kind: DependencyKind::Python,
            installed: false,
            detail: None,
        });
    }
    if root.join("Cargo.toml").exists() {
        deps.push(SkillDependency {
            name: "cargo dependencies".into(),
            requirement: Some("Cargo.toml".into()),
            kind: DependencyKind::File,
            installed: false,
            detail: Some("manual build is required".into()),
        });
    }
    if root.join("go.mod").exists() {
        deps.push(SkillDependency {
            name: "go dependencies".into(),
            requirement: Some("go.mod".into()),
            kind: DependencyKind::File,
            installed: false,
            detail: Some("manual build is required".into()),
        });
    }
    deps
}

pub fn is_memory_skill(path: &Path) -> bool {
    path.to_string_lossy()
        .to_ascii_lowercase()
        .contains("memory")
}
