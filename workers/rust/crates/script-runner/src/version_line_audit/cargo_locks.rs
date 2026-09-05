use std::fs;
use std::path::Path;

const LOCK_FILES: &[&str] = &[
    "workers/rust/Cargo.lock",
    "sdks/rust/Cargo.lock",
    "apps/hub-gui/src-tauri/Cargo.lock",
    "apps/workbench-gui/src-tauri/Cargo.lock",
    "apps/installer-gui/src-tauri/Cargo.lock",
    "workers/rust/templates/operator-crate-template/Cargo.lock",
];

pub(super) struct CargoLockVersion {
    pub(super) file: &'static str,
    pub(super) name: String,
    pub(super) version: String,
}

pub(super) fn cargo_lock_versions(root: &Path) -> Result<Vec<CargoLockVersion>, String> {
    let mut versions = Vec::new();
    for file in LOCK_FILES {
        let text = fs::read_to_string(root.join(file))
            .map_err(|error| format!("failed to read {file}: {error}"))?;
        for block in text.split("[[package]]").skip(1) {
            let Some(name) = quoted_assignment(block, "name") else {
                continue;
            };
            if !name.starts_with("kyuubiki-") || name == "kyuubiki-operator-template" {
                continue;
            }
            let version = quoted_assignment(block, "version")
                .ok_or_else(|| format!("{file}: package {name} has no version"))?;
            versions.push(CargoLockVersion {
                file,
                name: name.to_string(),
                version: version.to_string(),
            });
        }
    }
    Ok(versions)
}

fn quoted_assignment<'a>(block: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key} = \"");
    block.lines().find_map(|line| {
        line.trim()
            .strip_prefix(&prefix)
            .and_then(|value| value.strip_suffix('"'))
    })
}

#[cfg(test)]
mod tests {
    use super::quoted_assignment;

    #[test]
    fn reads_package_fields_without_matching_dependencies() {
        let block = r#"
name = "kyuubiki-engine"
version = "2.20.1"
dependencies = ["kyuubiki-protocol"]
"#;
        assert_eq!(quoted_assignment(block, "name"), Some("kyuubiki-engine"));
        assert_eq!(quoted_assignment(block, "version"), Some("2.20.1"));
    }
}
