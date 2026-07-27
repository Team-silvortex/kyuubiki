use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use crate::desktop::{Platform, host_platform};
use crate::{RepoPaths, RunnerResult};

pub(crate) fn distribution_preflight(paths: &RepoPaths, platform: Platform) -> RunnerResult<()> {
    if platform != host_platform() {
        return Err(format!(
            "distribution release for {} must run on a {} host",
            platform.as_str(),
            platform.as_str()
        ));
    }
    match platform {
        Platform::Macos => macos_distribution_preflight(paths),
        Platform::Windows => windows_distribution_preflight(),
        Platform::Linux => Ok(()),
    }
}

pub(crate) fn verify_distribution_artifacts(
    paths: &RepoPaths,
    platform: Platform,
) -> RunnerResult<()> {
    match platform {
        Platform::Macos => verify_macos_artifacts(paths),
        Platform::Windows | Platform::Linux => Ok(()),
    }
}

pub(crate) fn verify_runtime_payload(paths: &RepoPaths, platform: Platform) -> RunnerResult<()> {
    let root = paths.root.join("dist").join(platform.as_str());
    let manifest_path = root.join("manifests/service-launch.json");
    let bytes = std::fs::read(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    let manifest: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;
    if manifest["schema_version"] != "kyuubiki.service-launch/v1" {
        return Err(format!(
            "invalid service launch schema in {}",
            manifest_path.display()
        ));
    }
    let services = manifest["services"]
        .as_array()
        .ok_or_else(|| format!("{} has no services array", manifest_path.display()))?;
    let mut missing = Vec::new();
    for service in services {
        let id = service["id"].as_str().unwrap_or("unknown");
        for field in ["command", "cwd"] {
            let Some(relative) = service[field].as_str() else {
                missing.push(format!("{id}:{field}=undeclared"));
                continue;
            };
            let relative = relative.replace("{port}", "5001");
            let path = checked_payload_path(&root, &relative)?;
            let present = if field == "command" {
                path.is_file()
            } else {
                path.is_dir()
            };
            if !present {
                missing.push(format!("{id}:{field}={relative}"));
            }
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "installer runtime payload is incomplete for {}: {}; acquire/stage the declared payloads before desktop-release",
            platform.as_str(),
            missing.join(", ")
        ));
    }
    println!(
        "installer runtime payload verification passed for {}",
        platform.as_str()
    );
    Ok(())
}

pub(crate) fn runtime_payload_readiness(paths: &RepoPaths, platform: Platform) -> String {
    match verify_runtime_payload(paths, platform) {
        Ok(()) => "ready".to_string(),
        Err(error) => format!("blocked ({error})"),
    }
}

pub(crate) fn distribution_readiness(platform: Platform) -> String {
    match platform {
        Platform::Macos => match macos_signing_inputs() {
            Ok(_) => "ready (Developer ID + notarization credentials)".to_string(),
            Err(error) => format!("blocked ({error})"),
        },
        Platform::Windows => {
            if nonempty_env("WINDOWS_CERTIFICATE_THUMBPRINT").is_some() {
                "ready (certificate thumbprint configured)".to_string()
            } else {
                "blocked (WINDOWS_CERTIFICATE_THUMBPRINT is missing)".to_string()
            }
        }
        Platform::Linux => "ready (artifact integrity gate; repository signing is external)".into(),
    }
}

fn macos_distribution_preflight(paths: &RepoPaths) -> RunnerResult<()> {
    macos_signing_inputs()?;
    for app in ["hub-gui", "installer-gui", "workbench-gui"] {
        let config = paths
            .root
            .join("apps")
            .join(app)
            .join("src-tauri")
            .join("tauri.conf.json");
        let text = std::fs::read_to_string(&config)
            .map_err(|error| format!("failed to read {}: {error}", config.display()))?;
        if text.contains("\"signingIdentity\": \"-\"") {
            return Err(format!(
                "{} still forces ad-hoc signing; remove signingIdentity `-` before release",
                config.display()
            ));
        }
    }
    Ok(())
}

fn windows_distribution_preflight() -> RunnerResult<()> {
    nonempty_env("WINDOWS_CERTIFICATE_THUMBPRINT")
        .map(|_| ())
        .ok_or_else(|| {
            "WINDOWS_CERTIFICATE_THUMBPRINT is required for a Windows distribution release"
                .to_string()
        })
}

fn macos_signing_inputs() -> RunnerResult<MacSigningInputs> {
    let identity = nonempty_env("APPLE_SIGNING_IDENTITY")
        .filter(|value| value != "-")
        .ok_or_else(|| {
            "APPLE_SIGNING_IDENTITY must name a Developer ID Application identity".to_string()
        })?;
    let has_apple_id = ["APPLE_ID", "APPLE_PASSWORD", "APPLE_TEAM_ID"]
        .iter()
        .all(|name| nonempty_env(name).is_some());
    let has_api_key = nonempty_env("APPLE_API_KEY").is_some()
        && nonempty_env("APPLE_API_ISSUER").is_some()
        && (nonempty_env("APPLE_API_KEY_PATH").is_some()
            || nonempty_env("APPLE_API_KEY_CONTENT").is_some());
    if !has_apple_id && !has_api_key {
        return Err(
            "Apple notarization credentials are missing (Apple ID triplet or App Store Connect API key)"
                .to_string(),
        );
    }
    Ok(MacSigningInputs {
        identity,
        notarization_mode: if has_api_key { "api-key" } else { "apple-id" },
    })
}

fn verify_macos_artifacts(paths: &RepoPaths) -> RunnerResult<()> {
    let root = paths.root.join("target/desktop-cache/macos/release/bundle");
    let apps = collect_extension(&root.join("macos"), "app")?;
    if apps.len() != 3 {
        return Err(format!(
            "expected three signed macOS app bundles, found {} under {}",
            apps.len(),
            root.display()
        ));
    }
    for app in apps {
        run_checked("codesign", ["--verify", "--deep", "--strict"], &app)?;
        let detail = command_output("codesign", ["-dv", "--verbose=4"], &app)?;
        if detail.contains("Signature=adhoc")
            || !detail.contains("Authority=Developer ID Application")
        {
            return Err(format!(
                "{} is not signed with Developer ID Application",
                app.display()
            ));
        }
        run_checked("xcrun", ["stapler", "validate"], &app)?;
        run_checked("spctl", ["--assess", "--type", "execute"], &app)?;
    }
    for dmg in collect_extension(&root.join("dmg"), "dmg")? {
        run_checked("xcrun", ["stapler", "validate"], &dmg)?;
        run_checked("spctl", ["--assess", "--type", "open"], &dmg)?;
    }
    println!("macOS distribution signature and notarization verification passed");
    Ok(())
}

fn collect_extension(root: &Path, extension: &str) -> RunnerResult<Vec<PathBuf>> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    Ok(std::fs::read_dir(root)
        .map_err(|error| format!("failed to read {}: {error}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some(extension))
        .collect())
}

fn checked_payload_path(root: &Path, relative: &str) -> RunnerResult<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(format!(
            "runtime payload path escapes the platform stage: {}",
            relative.display()
        ));
    }
    Ok(root.join(relative))
}

fn run_checked<const N: usize>(program: &str, args: [&str; N], path: &Path) -> RunnerResult<()> {
    let output = Command::new(program)
        .args(args)
        .arg(path)
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} rejected {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn command_output<const N: usize>(
    program: &str,
    args: [&str; N],
    path: &Path,
) -> RunnerResult<String> {
    let output = Command::new(program)
        .args(args)
        .arg(path)
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(text)
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

struct MacSigningInputs {
    #[allow(dead_code)]
    identity: String,
    #[allow(dead_code)]
    notarization_mode: &'static str,
}

#[cfg(test)]
mod tests {
    use super::{checked_payload_path, collect_extension};
    use std::fs;

    #[test]
    fn artifact_collection_is_extension_scoped() {
        let root = std::env::temp_dir().join("kyuubiki-distribution-artifacts");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("Kyuubiki Hub.app")).unwrap();
        fs::write(root.join("notes.txt"), "not an app").unwrap();
        let apps = collect_extension(&root, "app").unwrap();
        fs::remove_dir_all(root).unwrap();
        assert_eq!(apps.len(), 1);
    }

    #[test]
    fn runtime_payload_paths_cannot_escape_the_stage() {
        let root = std::path::Path::new("/tmp/dist/macos");
        assert!(checked_payload_path(root, "../secrets").is_err());
        assert_eq!(
            checked_payload_path(root, "bin/kyuubiki-cli").unwrap(),
            root.join("bin/kyuubiki-cli")
        );
    }
}
