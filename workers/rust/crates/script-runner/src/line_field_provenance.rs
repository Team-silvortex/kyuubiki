use crate::native_time::utc_iso_timestamp;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TRACKED_INPUTS: &[&str] = &[
    "evidence/operator-qualification/line-field-closed-form-baseline.json",
    "evidence/operator-qualification/line-field-closed-form-derivation.md",
    "evidence/operator-qualification/line-field-tolerance-policy.json",
    "workers/rust/crates/solver/tests/accuracy_baselines/line_1d.rs",
    "scripts/check-line-field-closed-form-baseline.mjs",
];

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_capture_line_field_qualification_provenance(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!(
            "usage: kyuubiki-script-runner capture-line-field-qualification-provenance [--out tmp/file.json]"
        );
        return Ok(0);
    }
    let out = parse_args(args)?;
    let payload = build_provenance(root)?;
    if let Some(out) = out {
        let (absolute, relative) = repo_local_path(root, &out, "--out")?;
        write_json(&absolute, &payload)?;
        println!("line-field qualification provenance wrote {relative}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload)
                .map_err(|error| format!("failed to encode provenance: {error}"))?
        );
    }
    Ok(0)
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Option<String>> {
    let mut out = None;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => {
                let Some(value) = iter.next() else {
                    return Err("--out requires a repo-local path".to_string());
                };
                out = Some(value.to_string_lossy().to_string());
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(out)
}

pub(crate) fn build_provenance(root: &Path) -> RunnerResult<Value> {
    let git_status = run(root, "git", &["status", "--short"]).unwrap_or_default();
    let status_entry_count = git_status
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    Ok(json!({
        "schema_version": "kyuubiki.operator-qualification-provenance/v1",
        "version_line": "tamamono 1.20.x",
        "candidate_id": "line-field-closed-form",
        "generated_at_utc": utc_iso_timestamp(),
        "commands": {
            "evidence_check": "./scripts/kyuubiki check-line-field-closed-form-baseline",
            "solver_baseline": "cargo test -p kyuubiki-solver --test accuracy_baselines line_1d",
        },
        "source_revision": {
            "git_commit": run(root, "git", &["rev-parse", "HEAD"]),
            "git_branch": run(root, "git", &["rev-parse", "--abbrev-ref", "HEAD"]),
            "working_tree_clean": git_status.is_empty(),
            "status_entry_count": status_entry_count,
        },
        "toolchain": {
            "node": run(root, "node", &["--version"]),
            "rustc": run(root, "rustc", &["--version"]),
            "cargo": run(root, "cargo", &["--version"]),
        },
        "platform": {
            "os": env::consts::OS,
            "arch": env::consts::ARCH,
            "release": run(root, "uname", &["-r"]),
        },
        "tracked_inputs": tracked_inputs(root)?,
        "retention_policy": {
            "release_artifact": true,
            "repo_relative_paths_only": true,
            "no_local_absolute_paths": true,
        },
    }))
}

fn tracked_inputs(root: &Path) -> RunnerResult<Vec<Value>> {
    TRACKED_INPUTS
        .iter()
        .map(|relative_path| {
            Ok(json!({
                "path": relative_path,
                "sha256": sha256_file(root, relative_path)?,
            }))
        })
        .collect()
}

fn sha256_file(root: &Path, relative_path: &str) -> RunnerResult<String> {
    let path = root.join(relative_path);
    if !path.exists() {
        return Err(format!("tracked input missing: {relative_path}"));
    }
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn run(root: &Path, command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command)
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn repo_local_path(root: &Path, path: &str, label: &str) -> RunnerResult<(PathBuf, String)> {
    let absolute = root.join(path);
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("{label} must stay inside the repository"))?
        .to_string_lossy()
        .replace('\\', "/");
    if relative.starts_with("..") || Path::new(&relative).is_absolute() {
        return Err(format!("{label} must stay inside the repository"));
    }
    Ok((absolute, relative))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode provenance: {error}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::TRACKED_INPUTS;

    #[test]
    fn tracked_inputs_keep_release_evidence_surface() {
        assert!(
            TRACKED_INPUTS
                .contains(&"evidence/operator-qualification/line-field-closed-form-baseline.json")
        );
        assert!(TRACKED_INPUTS.contains(&"scripts/check-line-field-closed-form-baseline.mjs"));
    }
}
