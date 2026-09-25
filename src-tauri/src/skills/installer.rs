use super::{list, model::DependencyKind, sandbox::SkillSandbox};
use std::process::Command;

pub fn install(id: &str) -> Result<(), String> {
    let meta = list()?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or("skill not found")?;
    let sandbox = SkillSandbox::create(
        id,
        std::path::Path::new(&meta.source_path),
        Default::default(),
        meta.sandbox.network,
    )
    .map_err(|e| e.to_string())?;
    for dependency in meta.dependencies {
        let output = match dependency.kind {
            DependencyKind::Node => Command::new("npm")
                .args([
                    "install",
                    "--prefix",
                    &sandbox.runtime.display().to_string(),
                ])
                .current_dir(&meta.source_path)
                .output(),
            DependencyKind::Python => Command::new("python3")
                .args([
                    "-m",
                    "venv",
                    &sandbox.runtime.join("python").display().to_string(),
                ])
                .current_dir(&meta.source_path)
                .output(),
            _ => continue,
        }
        .map_err(|e| format!("failed to start dependency installer: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "dependency install failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    Ok(())
}
