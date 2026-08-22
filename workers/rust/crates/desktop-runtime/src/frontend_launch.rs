use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use crate::runtime_layout::{RuntimePaths, resolve_development_command};

pub(crate) struct FrontendLaunchSpec {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub label: &'static str,
}

pub(crate) fn resolve(paths: &RuntimePaths) -> Result<FrontendLaunchSpec, String> {
    if !paths.is_development() {
        let spec = paths.service("frontend", &[])?;
        return Ok(FrontendLaunchSpec {
            command: spec.command,
            args: spec.args,
            cwd: spec.cwd,
            label: "installer-managed native workbench frontend",
        });
    }

    let frontend = paths.root.join("apps/frontend");
    let export = frontend.join("out");
    if static_export_is_stale(&frontend, &export)? {
        build_static_export(paths, &frontend)?;
    }
    if !export.join("index.html").is_file() {
        return Err(format!(
            "frontend build completed without {}",
            export.join("index.html").display()
        ));
    }
    Ok(FrontendLaunchSpec {
        command: resolve_development_command(&paths.root, "cargo")?,
        args: vec![
            "run".to_string(),
            "-p".to_string(),
            "kyuubiki-desktop-runtime".to_string(),
            "--bin".to_string(),
            "kyuubiki-runtime".to_string(),
            "--".to_string(),
            "serve-frontend".to_string(),
            "--root".to_string(),
            export.display().to_string(),
        ],
        cwd: paths.root.join("workers/rust"),
        label: "development native workbench frontend",
    })
}

fn build_static_export(paths: &RuntimePaths, frontend: &Path) -> Result<(), String> {
    let npm = resolve_development_command(&paths.root, "npm")?;
    let status = Command::new(&npm)
        .args(["run", "build"])
        .current_dir(frontend)
        .status()
        .map_err(|error| {
            format!(
                "failed to build frontend export with {}: {error}",
                npm.display()
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "frontend static export failed with status {status}"
        ))
    }
}

fn static_export_is_stale(frontend: &Path, export: &Path) -> Result<bool, String> {
    let output_time = match modified_at(&export.join("index.html")) {
        Ok(time) => time,
        Err(_) => return Ok(true),
    };
    for path in [
        frontend.join("next.config.mjs"),
        frontend.join("package.json"),
        frontend.join("package-lock.json"),
        frontend.join("tsconfig.json"),
    ] {
        if modified_at(&path).is_ok_and(|time| time > output_time) {
            return Ok(true);
        }
    }
    for root in [frontend.join("src"), frontend.join("public")] {
        if tree_has_newer_file(&root, output_time)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn tree_has_newer_file(root: &Path, output_time: SystemTime) -> Result<bool, String> {
    let entries = fs::read_dir(root).map_err(|error| {
        format!(
            "failed to inspect frontend source {}: {error}",
            root.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if metadata.is_dir() {
            if tree_has_newer_file(&path, output_time)? {
                return Ok(true);
            }
        } else if metadata.is_file()
            && metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH) > output_time
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn modified_at(path: &Path) -> Result<SystemTime, String> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;

    use super::static_export_is_stale;

    #[test]
    fn source_changes_invalidate_the_static_export() {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-static-export-stale-{}",
            std::process::id()
        ));
        let frontend = root.join("frontend");
        let export = frontend.join("out");
        fs::create_dir_all(frontend.join("src")).unwrap();
        fs::create_dir_all(frontend.join("public")).unwrap();
        fs::create_dir_all(&export).unwrap();
        for file in [
            "next.config.mjs",
            "package.json",
            "package-lock.json",
            "tsconfig.json",
        ] {
            fs::write(frontend.join(file), "fixture").unwrap();
        }
        fs::write(frontend.join("src/page.tsx"), "old").unwrap();
        std::thread::sleep(Duration::from_millis(10));
        fs::write(export.join("index.html"), "current").unwrap();
        assert!(!static_export_is_stale(&frontend, &export).unwrap());
        std::thread::sleep(Duration::from_millis(10));
        fs::write(frontend.join("src/page.tsx"), "new").unwrap();
        assert!(static_export_is_stale(&frontend, &export).unwrap());
        fs::remove_dir_all(root).unwrap();
    }
}
