use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "deploy/install-update-disk-hygiene.json";
const INTEGRITY_PATH: &str = "deploy/installation-integrity-contract.json";
const CHANNELS_PATH: &str = "deploy/update-channels.json";
const PACKAGING_DOCS_PATH: &str = "docs/packaging-and-deployment.md";
const UPLOAD_RUNNER_PATH: &str =
    "workers/rust/crates/script-runner/src/desktop_release_upload_remote.rs";
const EXPECTED_SCHEMA: &str = "kyuubiki.install-update-disk-hygiene/v1";

pub(crate) fn run_check_install_update_disk_hygiene(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test()?;
        println!("install/update disk hygiene self-test passed");
        return Ok(0);
    }
    if wants_help(&args) {
        println!("usage: kyuubiki-script-runner check-install-update-disk-hygiene [--self-test]");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-install-update-disk-hygiene only accepts --self-test".to_string());
    }

    let contract = read_json(root, CONTRACT_PATH)?;
    let integrity = read_json(root, INTEGRITY_PATH)?;
    let channels = read_json(root, CHANNELS_PATH)?;
    let packaging = read_text(root, PACKAGING_DOCS_PATH)?;
    let upload_runner = read_text(root, UPLOAD_RUNNER_PATH)?;
    let issues = validate_contract(&contract, &integrity, &channels, &packaging, &upload_runner);

    if !issues.is_empty() {
        eprintln!("install/update disk hygiene validation failed:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        return Ok(1);
    }

    println!(
        "install/update disk hygiene ok: {} removable roots",
        array_len(&contract, "/local_retention_policy/removable_roots")
    );
    Ok(0)
}

fn validate_contract(
    contract: &Value,
    integrity: &Value,
    channels: &Value,
    docs: &str,
    runner: &str,
) -> Vec<String> {
    let mut issues = Vec::new();

    if string_at(contract, "/schema_version") != Some(EXPECTED_SCHEMA) {
        issues.push(format!("{CONTRACT_PATH}: unexpected schema_version"));
    }

    if string_at(contract, "/shipping_version") != string_at(integrity, "/shipping_version") {
        issues.push(format!(
            "{CONTRACT_PATH}: shipping_version does not match installation integrity"
        ));
    }

    if string_at(contract, "/shipping_version") != string_at(channels, "/shipping_version") {
        issues.push(format!(
            "{CONTRACT_PATH}: shipping_version does not match update channels"
        ));
    }

    require_source_contract(
        &mut issues,
        string_at(contract, "/source_contracts/installation_integrity"),
        INTEGRITY_PATH,
    );
    require_source_contract(
        &mut issues,
        string_at(contract, "/source_contracts/update_channels"),
        CHANNELS_PATH,
    );
    require_source_contract(
        &mut issues,
        string_at(contract, "/source_contracts/packaging_docs"),
        PACKAGING_DOCS_PATH,
    );

    validate_remote_policy(&mut issues, contract, docs, runner);
    validate_local_retention(&mut issues, contract, integrity, docs, runner);
    validate_update_visibility(&mut issues, contract, channels);

    if array_len(contract, "/operator_visible_rules") == 0 {
        issues.push("operator_visible_rules: missing non-empty array".to_string());
    }

    issues
}

fn validate_remote_policy(issues: &mut Vec<String>, contract: &Value, docs: &str, runner: &str) {
    let authority = string_at(contract, "/remote_artifact_policy/authority");
    require_text(issues, authority, "remote_artifact_policy.authority");
    if authority != Some("remote_download_server") {
        issues.push("remote_artifact_policy.authority must be remote_download_server".to_string());
    }

    let command = string_at(contract, "/remote_artifact_policy/required_command");
    require_text(issues, command, "remote_artifact_policy.required_command");
    if let Some(command) = command
        && !docs.contains(command)
    {
        issues.push("packaging docs must mention the remote upload command".to_string());
    }

    for key in [
        "remote_root_env",
        "remote_host_env",
        "version_env",
        "password_env",
    ] {
        let pointer = format!("/remote_artifact_policy/{key}");
        let value = string_at(contract, &pointer);
        require_text(issues, value, &format!("remote_artifact_policy.{key}"));
        if let Some(value) = value
            && (!docs.contains(value) || !runner.contains(value))
        {
            issues.push(format!(
                "{key} must be documented and implemented by the native upload runner"
            ));
        }
    }

    if string_at(contract, "/remote_artifact_policy/password_policy")
        != Some("temporary_compatibility_only")
    {
        issues.push("password_policy must keep password uploads temporary-only".to_string());
    }
}

fn validate_local_retention(
    issues: &mut Vec<String>,
    contract: &Value,
    integrity: &Value,
    docs: &str,
    runner: &str,
) {
    if string_at(contract, "/local_retention_policy/default") != Some("metadata_and_source_only") {
        issues.push("local_retention_policy.default must be metadata_and_source_only".to_string());
    }
    if string_at(
        contract,
        "/local_retention_policy/purge_after_remote_upload_env",
    ) != Some("PURGE_LOCAL")
    {
        issues.push("local_retention_policy must use PURGE_LOCAL as the purge switch".to_string());
    }
    if string_at(
        contract,
        "/local_retention_policy/purge_after_remote_upload_value",
    ) != Some("1")
    {
        issues.push("local_retention_policy purge value must be 1".to_string());
    }
    if !docs.contains("PURGE_LOCAL=1") || !runner.contains("PURGE_LOCAL") {
        issues.push(
            "PURGE_LOCAL=1 must be documented and implemented by the native upload runner"
                .to_string(),
        );
    }

    for root in string_array_at(contract, "/local_retention_policy/removable_roots") {
        validate_relative_path(issues, &root, "removable_roots");
    }

    let protected_paths = string_array_at(integrity, "/protected_paths");
    for root in string_array_at(contract, "/local_retention_policy/protected_roots") {
        validate_relative_path(issues, &root, "protected_roots");
        if !protected_paths.contains(&root) {
            issues.push(format!(
                "protected root is not protected by installation integrity: {root}"
            ));
        }
    }

    if string_array_at(contract, "/local_retention_policy/removable_roots")
        .iter()
        .any(|root| root == "tmp/data")
    {
        issues.push("tmp/data must never be a release-bundle purge target".to_string());
    }
}

fn validate_update_visibility(issues: &mut Vec<String>, contract: &Value, channels: &Value) {
    let required_channel = string_at(contract, "/update_visibility_policy/required_channel");
    let Some(channel) = find_channel(channels, required_channel) else {
        issues.push(format!(
            "missing update channel: {}",
            required_channel.unwrap_or("")
        ));
        return;
    };

    if string_at(channel, "/rollout/cleanup_policy")
        != string_at(
            contract,
            "/update_visibility_policy/required_cleanup_policy",
        )
    {
        issues.push("update channel cleanup policy drifted from disk hygiene contract".to_string());
    }
    if string_at(channel, "/rollout/rollback")
        != string_at(contract, "/update_visibility_policy/required_rollback")
    {
        issues
            .push("update channel rollback policy drifted from disk hygiene contract".to_string());
    }
    if string_at(channel, "/rollout/requires_integrity_contract")
        != string_at(
            contract,
            "/update_visibility_policy/requires_integrity_contract",
        )
    {
        issues.push(
            "update channel integrity contract link drifted from disk hygiene contract".to_string(),
        );
    }
}

fn find_channel<'a>(channels: &'a Value, required_channel: Option<&str>) -> Option<&'a Value> {
    let required_channel = required_channel?;
    channels
        .get("channels")
        .and_then(Value::as_array)?
        .iter()
        .find(|entry| string_at(entry, "/id") == Some(required_channel))
}

fn validate_relative_path(issues: &mut Vec<String>, value: &str, field: &str) {
    if value.trim().is_empty() {
        issues.push(format!("{field}: empty path"));
        return;
    }
    if value.starts_with('/') || value.split('/').any(|part| part == "..") {
        issues.push(format!(
            "{field}: path must be repo-relative and non-traversing: {value}"
        ));
    }
}

fn require_source_contract(issues: &mut Vec<String>, actual: Option<&str>, expected: &str) {
    if actual != Some(expected) {
        issues.push(format!("source contract must point at {expected}"));
    }
}

fn require_text(issues: &mut Vec<String>, value: Option<&str>, label: &str) {
    if value.is_none_or(|value| value.trim().is_empty()) {
        issues.push(format!("{label}: missing text"));
    }
}

fn run_self_test() -> RunnerResult<()> {
    let mut sample = json!({
        "schema_version": EXPECTED_SCHEMA,
        "shipping_version": "1.20.0",
        "source_contracts": {
            "installation_integrity": INTEGRITY_PATH,
            "update_channels": CHANNELS_PATH,
            "packaging_docs": PACKAGING_DOCS_PATH
        },
        "remote_artifact_policy": {
            "authority": "remote_download_server",
            "required_command": "./scripts/kyuubiki desktop-upload-remote",
            "remote_root_env": "KYUUBIKI_RELEASE_REMOTE_DIR",
            "remote_host_env": "KYUUBIKI_RELEASE_REMOTE_HOST",
            "version_env": "KYUUBIKI_RELEASE_VERSION",
            "password_env": "KYUUBIKI_RELEASE_REMOTE_PASSWORD",
            "password_policy": "temporary_compatibility_only"
        },
        "local_retention_policy": {
            "default": "metadata_and_source_only",
            "purge_after_remote_upload_env": "PURGE_LOCAL",
            "purge_after_remote_upload_value": "1",
            "removable_roots": ["dist/macos"],
            "protected_roots": ["tmp/data"]
        },
        "update_visibility_policy": {
            "required_channel": "stable",
            "required_cleanup_policy": "allowlisted",
            "required_rollback": "same-channel reinstall",
            "requires_integrity_contract": INTEGRITY_PATH
        },
        "operator_visible_rules": ["visible cleanup"]
    });
    let integrity = json!({
        "shipping_version": "1.20.0",
        "protected_paths": ["tmp/data"]
    });
    let channels = json!({
        "shipping_version": "1.20.0",
        "channels": [{
            "id": "stable",
            "rollout": {
                "cleanup_policy": "allowlisted",
                "rollback": "same-channel reinstall",
                "requires_integrity_contract": INTEGRITY_PATH
            }
        }]
    });
    let docs = "PURGE_LOCAL=1 ./scripts/kyuubiki desktop-upload-remote KYUUBIKI_RELEASE_REMOTE_DIR KYUUBIKI_RELEASE_REMOTE_HOST KYUUBIKI_RELEASE_VERSION KYUUBIKI_RELEASE_REMOTE_PASSWORD";
    let runner = "PURGE_LOCAL KYUUBIKI_RELEASE_REMOTE_DIR KYUUBIKI_RELEASE_REMOTE_HOST KYUUBIKI_RELEASE_VERSION KYUUBIKI_RELEASE_REMOTE_PASSWORD";
    let issues = validate_contract(&sample, &integrity, &channels, docs, runner);
    if !issues.is_empty() {
        return Err(format!(
            "self-test unexpectedly failed: {}",
            issues.join("; ")
        ));
    }

    sample["local_retention_policy"]["removable_roots"] = json!(["/tmp/unsafe"]);
    let issues = validate_contract(
        &sample,
        &integrity,
        &json!({ "shipping_version": "1.20.0", "channels": [] }),
        "",
        "",
    );
    if !issues.iter().any(|issue| issue.contains("repo-relative")) {
        return Err("self-test did not reject absolute removable root".to_string());
    }
    Ok(())
}

fn array_len(value: &Value, pointer: &str) -> usize {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn string_array_at(value: &Value, pointer: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    let path = root.join(relative_path);
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}

fn wants_help(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}

#[cfg(test)]
mod tests {
    use super::{run_self_test, validate_relative_path};

    #[test]
    fn self_test_contract_fixture_is_valid() {
        run_self_test().unwrap();
    }

    #[test]
    fn relative_path_validation_rejects_absolute_paths() {
        let mut issues = Vec::new();
        validate_relative_path(&mut issues, "/tmp/unsafe", "removable_roots");
        assert!(issues.iter().any(|issue| issue.contains("repo-relative")));
    }
}
