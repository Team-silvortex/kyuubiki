use super::{
    Attachment, Attachments, CONTRACT_PATH, CONTRACT_SCHEMA, LIMITATIONS, Platform, Provenance,
    QUALIFICATION_ID, QualificationContract, QualificationReport, REPORT_SCHEMA, Summary,
};
use crate::RunnerResult;
use crate::operator_package_dynamic_smoke::dynamic_smoke_errors;
use crate::qualification_support::{read_json, repo_path, write_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const CONTRACT_SCHEMA_PATH: &str =
    "schemas/operator-sdk-windows-operational-qualification-contract.schema.json";
const REPORT_SCHEMA_PATH: &str =
    "schemas/operator-sdk-windows-operational-qualification-report.schema.json";
const DYNAMIC_EXAMPLE_PATH: &str = "schemas/examples.operator-package-dynamic-smoke.json";
const REQUIRED_STAGES: &[&str] = &[
    "template_tests",
    "strict_preflight",
    "template_cdylib_build",
    "engine_dynamic_host_load",
    "agent_dynamic_host_dispatch",
    "installer_managed_agent_lifecycle",
];
const REQUIRED_SOURCES: &[&str] = &[
    ".github/workflows/operator-sdk-windows-qualification.yml",
    "config/architecture/operator-sdk-windows-operational-qualification.json",
    "schemas/operator-package-dynamic-smoke.schema.json",
    "schemas/operator-sdk-windows-operational-qualification-contract.schema.json",
    "schemas/operator-sdk-windows-operational-qualification-report.schema.json",
    "workers/rust/crates/cli/tests/operator_package_installer_live.rs",
    "workers/rust/crates/cli/tests/operator_package_live.rs",
    "workers/rust/crates/cli/tests/operator_package_orchestra_fetch_live.rs",
    "workers/rust/crates/script-runner/src/operator_package_dynamic_smoke.rs",
    "workers/rust/crates/script-runner/src/operator_package_dynamic_smoke/check.rs",
    "workers/rust/crates/script-runner/src/operator_sdk_windows_qualification.rs",
    "workers/rust/crates/script-runner/src/operator_sdk_windows_qualification/validation.rs",
];
const REQUIRED_PROVENANCE_FIELDS: &[&str] = &[
    "repository",
    "commit_sha",
    "run_id",
    "runner_id",
    "workflow_ref",
];

pub(super) fn load_and_validate_contract(root: &Path) -> RunnerResult<QualificationContract> {
    let contract: QualificationContract = read_json(root, CONTRACT_PATH)?;
    if contract.schema_version != CONTRACT_SCHEMA {
        return Err("Windows qualification contract schema_version mismatch".to_string());
    }
    if contract.qualification_id != QUALIFICATION_ID {
        return Err("Windows qualification_id mismatch".to_string());
    }
    let expected_platform = Platform {
        os: "windows".to_string(),
        arch: "x86_64".to_string(),
        abi: "msvc".to_string(),
        dynamic_library_suffix: ".dll".to_string(),
    };
    if contract.platform != expected_platform {
        return Err("Windows qualification platform must be x86_64-pc-windows-msvc".to_string());
    }
    require_exact_strings(
        &contract.required_stages,
        REQUIRED_STAGES,
        "required_stages",
    )?;
    require_exact_strings(&contract.source_files, REQUIRED_SOURCES, "source_files")?;
    require_exact_strings(
        &contract.provenance.required_fields,
        REQUIRED_PROVENANCE_FIELDS,
        "provenance.required_fields",
    )?;
    require_exact_strings(
        &contract.provenance.allowed_providers,
        &["github-actions", "self-hosted-windows"],
        "provenance.allowed_providers",
    )?;
    if contract.report.schema_version != REPORT_SCHEMA
        || contract.report.schema_path != REPORT_SCHEMA_PATH
        || contract.report.dynamic_smoke_file != "operator-sdk-windows-dynamic-smoke.json"
        || contract.report.preflight_file != "operator-sdk-windows-package-preflight.json"
    {
        return Err("Windows qualification report policy drifted".to_string());
    }
    for path in contract
        .source_files
        .iter()
        .chain([&contract.report.schema_path, &contract.workflow.path])
    {
        if !repo_path(root, path)?.is_file() {
            return Err(format!("Windows qualification source is missing: {path}"));
        }
    }
    validate_schema_const(root, CONTRACT_SCHEMA_PATH, CONTRACT_SCHEMA)?;
    validate_schema_const(root, REPORT_SCHEMA_PATH, REPORT_SCHEMA)?;
    validate_workflow(root, &contract)?;
    Ok(contract)
}

pub(super) fn validate_capture_platform(os: &str, arch: &str, rust_host: &str) -> RunnerResult<()> {
    if os != "windows" || arch != "x86_64" || rust_host != "x86_64-pc-windows-msvc" {
        return Err(format!(
            "Windows operational qualification requires x86_64-pc-windows-msvc, got {os}/{arch}/{rust_host}"
        ));
    }
    Ok(())
}

pub(super) fn validate_report(
    root: &Path,
    contract: &QualificationContract,
    report: &QualificationReport,
) -> RunnerResult<()> {
    if report.schema_version != REPORT_SCHEMA
        || report.qualification_id != QUALIFICATION_ID
        || report.status != "pass"
        || report.generated_at_unix_ms == 0
    {
        return Err("Windows qualification report identity or status is invalid".to_string());
    }
    if report.platform != contract.platform {
        return Err("Windows qualification report platform mismatch".to_string());
    }
    if report.source_tree_sha256 != source_tree_digest(root, &contract.source_files)? {
        return Err("Windows qualification source tree digest mismatch".to_string());
    }
    validate_provenance(contract, &report.provenance)?;
    validate_summary(contract, &report.summary)?;
    if report.limitations != LIMITATIONS {
        return Err("Windows qualification limitations drifted".to_string());
    }

    validate_attachment(root, &report.attachments.dynamic_smoke)?;
    validate_attachment(root, &report.attachments.preflight)?;
    let dynamic: Value = read_json(root, &report.attachments.dynamic_smoke.path)?;
    let preflight: Value = read_json(root, &report.attachments.preflight.path)?;
    validate_dynamic_report(root, contract, report, &dynamic)?;
    validate_preflight(contract, report, &dynamic, &preflight)
}

fn validate_provenance(
    contract: &QualificationContract,
    provenance: &Provenance,
) -> RunnerResult<()> {
    if !contract
        .provenance
        .allowed_providers
        .contains(&provenance.provider)
    {
        return Err("Windows qualification provenance provider is not allowed".to_string());
    }
    for (name, value) in [
        ("repository", provenance.repository.as_str()),
        ("run_id", provenance.run_id.as_str()),
        ("runner_id", provenance.runner_id.as_str()),
        ("workflow_ref", provenance.workflow_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("Windows qualification provenance.{name} is empty"));
        }
    }
    if !is_commit_digest(&provenance.commit_sha) {
        return Err("Windows qualification commit_sha is not canonical lowercase hex".to_string());
    }
    Ok(())
}

fn validate_summary(contract: &QualificationContract, summary: &Summary) -> RunnerResult<()> {
    let complete = summary.stage_count == contract.required_stages.len()
        && summary.all_stages_passed
        && summary.engine_dynamic_load
        && summary.agent_rpc_dispatch
        && summary.bound_orchestra_rotation
        && summary.tamper_recovery
        && summary.installer_managed_lifecycle
        && summary.residue_cleanup
        && summary.release_complete;
    if !complete {
        return Err("Windows qualification summary is not release-complete".to_string());
    }
    Ok(())
}

fn validate_attachment(root: &Path, attachment: &Attachment) -> RunnerResult<()> {
    validate_repo_relative(&attachment.path, "attachment.path")?;
    if !is_sha256(&attachment.sha256) {
        return Err("Windows qualification attachment digest is invalid".to_string());
    }
    if attachment.sha256 != sha256_file(root, &attachment.path)? {
        return Err(format!(
            "Windows qualification attachment digest mismatch: {}",
            attachment.path
        ));
    }
    Ok(())
}

fn validate_dynamic_report(
    root: &Path,
    contract: &QualificationContract,
    report: &QualificationReport,
    dynamic: &Value,
) -> RunnerResult<()> {
    if let Some(error) = dynamic_smoke_errors(root, dynamic, "Windows dynamic smoke").first() {
        return Err(error.clone());
    }
    let dynamic_library = field(dynamic, "dynamic_library");
    validate_repo_relative(dynamic_library, "dynamic_library")?;
    if !dynamic_library.ends_with(&contract.platform.dynamic_library_suffix) {
        return Err("Windows dynamic smoke did not retain a DLL entrypoint".to_string());
    }
    if field(dynamic, "preflight_report") != report.attachments.preflight.path {
        return Err("Windows dynamic smoke preflight attachment mismatch".to_string());
    }
    let stages = dynamic
        .get("stages")
        .and_then(Value::as_array)
        .ok_or_else(|| "Windows dynamic smoke stages are missing".to_string())?;
    let stage_ids = stages
        .iter()
        .map(|stage| field(stage, "id").to_string())
        .collect::<Vec<_>>();
    if stage_ids != contract.required_stages {
        return Err("Windows dynamic smoke stage order drifted".to_string());
    }
    let agent_description = field(&stages[4], "description").to_ascii_lowercase();
    for required in ["orchestra", "tamper", "recovery"] {
        if !agent_description.contains(required) {
            return Err(format!(
                "Windows Agent stage does not assert {required} behavior"
            ));
        }
    }
    let installer_description = field(&stages[5], "description").to_ascii_lowercase();
    for required in ["install", "recover", "uninstall", "residue"] {
        if !installer_description.contains(required) {
            return Err(format!(
                "Windows Installer stage does not assert {required} behavior"
            ));
        }
    }
    Ok(())
}

fn validate_preflight(
    contract: &QualificationContract,
    report: &QualificationReport,
    dynamic: &Value,
    preflight: &Value,
) -> RunnerResult<()> {
    for (name, expected) in [
        ("accepted_package_count", 1),
        ("rejected_package_count", 0),
        ("readiness_error_count", 0),
        ("readiness_warning_count", 0),
    ] {
        if preflight.get(name).and_then(Value::as_u64) != Some(expected) {
            return Err(format!("Windows preflight {name} must be {expected}"));
        }
    }
    let package = preflight
        .get("accepted_packages")
        .and_then(Value::as_array)
        .and_then(|packages| packages.first())
        .ok_or_else(|| "Windows preflight accepted package is missing".to_string())?;
    if field(package, "package_id") != field(dynamic, "package_id")
        || field(package, "sdk_api_version") != field(dynamic, "sdk_api_version")
        || field(preflight, "host_version") != field(dynamic, "host_version")
        || field(package, "runtime") != "rust_crate"
    {
        return Err("Windows preflight package identity drifted".to_string());
    }
    let entrypoint = field(package, "entrypoint_path");
    validate_repo_relative(entrypoint, "accepted_packages[0].entrypoint_path")?;
    if !entrypoint.ends_with(&contract.platform.dynamic_library_suffix) {
        return Err("Windows preflight entrypoint is not a DLL".to_string());
    }
    if field(dynamic, "preflight_report") != report.attachments.preflight.path {
        return Err("Windows preflight attachment path drifted".to_string());
    }
    Ok(())
}

fn validate_workflow(root: &Path, contract: &QualificationContract) -> RunnerResult<()> {
    let text = fs::read_to_string(repo_path(root, &contract.workflow.path)?)
        .map_err(|error| format!("failed to read Windows qualification workflow: {error}"))?;
    for expected in [
        "runs-on: windows-latest",
        "qualify-operator-sdk-windows-operational",
        "check-operator-sdk-windows-operational-qualification",
        "actions/upload-artifact@v4",
        contract.workflow.artifact_name.as_str(),
    ] {
        if !text.contains(expected) {
            return Err(format!(
                "Windows qualification workflow is missing {expected}"
            ));
        }
    }
    for token in contract.workflow.capture_command.split_ascii_whitespace() {
        if !text.contains(token) {
            return Err(format!(
                "Windows qualification workflow is missing token {token}"
            ));
        }
    }
    Ok(())
}

fn validate_schema_const(root: &Path, path: &str, expected: &str) -> RunnerResult<()> {
    let schema: Value = read_json(root, path)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(expected)
    {
        return Err(format!("{path} schema_version const mismatch"));
    }
    Ok(())
}

pub(super) fn source_tree_digest(root: &Path, paths: &[String]) -> RunnerResult<String> {
    let mut ordered = paths.to_vec();
    ordered.sort();
    let mut hasher = Sha256::new();
    for path in ordered {
        let bytes = fs::read(repo_path(root, &path)?).map_err(|error| {
            format!("failed to read Windows qualification source {path}: {error}")
        })?;
        hasher.update(path.len().to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update(bytes.len().to_le_bytes());
        hasher.update(bytes);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) fn sha256_file(root: &Path, relative: &str) -> RunnerResult<String> {
    let bytes = fs::read(repo_path(root, relative)?)
        .map_err(|error| format!("failed to read Windows evidence {relative}: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(super) fn validator_self_test(
    root: &Path,
    contract: &QualificationContract,
) -> RunnerResult<()> {
    validate_capture_platform("windows", "x86_64", "x86_64-pc-windows-msvc")?;
    if validate_capture_platform("linux", "x86_64", "x86_64-unknown-linux-gnu").is_ok() {
        return Err("Windows validator accepted a Linux capture host".to_string());
    }
    let relative_root = format!("tmp/operator-sdk-windows-validator-{}", std::process::id());
    let absolute_root = repo_path(root, &relative_root)?;
    if absolute_root.exists() {
        fs::remove_dir_all(&absolute_root)
            .map_err(|error| format!("failed to reset validator fixture: {error}"))?;
    }
    fs::create_dir_all(&absolute_root)
        .map_err(|error| format!("failed to create validator fixture: {error}"))?;
    let result = run_validator_self_test(root, contract, &relative_root);
    let cleanup = fs::remove_dir_all(&absolute_root)
        .map_err(|error| format!("failed to clean validator fixture: {error}"));
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; {cleanup_error}")),
    }
}

fn run_validator_self_test(
    root: &Path,
    contract: &QualificationContract,
    relative_root: &str,
) -> RunnerResult<()> {
    let dynamic_path = format!("{relative_root}/operator-sdk-windows-dynamic-smoke.json");
    let preflight_path = format!("{relative_root}/operator-sdk-windows-package-preflight.json");
    let mut dynamic: Value = read_json(root, DYNAMIC_EXAMPLE_PATH)?;
    dynamic["dynamic_library"] = Value::String(
        "workers/rust/templates/operator-crate-template/target/debug/kyuubiki_operator_template.dll"
            .to_string(),
    );
    dynamic["preflight_report"] = Value::String(preflight_path.clone());
    write_json(root, &dynamic_path, &dynamic)?;
    let preflight = json!({
        "host_version": field(&dynamic, "host_version"),
        "accepted_package_count": 1,
        "rejected_package_count": 0,
        "readiness_error_count": 0,
        "readiness_warning_count": 0,
        "accepted_packages": [{
            "package_id": field(&dynamic, "package_id"),
            "sdk_api_version": field(&dynamic, "sdk_api_version"),
            "runtime": "rust_crate",
            "entrypoint_path": "workers/rust/templates/operator-crate-template/target/debug/kyuubiki_operator_template.dll"
        }]
    });
    write_json(root, &preflight_path, &preflight)?;
    let report = QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        generated_at_unix_ms: 1,
        status: "pass".to_string(),
        source_tree_sha256: source_tree_digest(root, &contract.source_files)?,
        platform: contract.platform.clone(),
        provenance: Provenance {
            provider: "github-actions".to_string(),
            repository: "Team-silvortex/kyuubiki".to_string(),
            commit_sha: "a".repeat(40),
            run_id: "1:1".to_string(),
            runner_id: "GitHub Actions 1".to_string(),
            workflow_ref: "Team-silvortex/kyuubiki/.github/workflows/operator-sdk-windows-qualification.yml@refs/heads/main".to_string(),
        },
        attachments: Attachments {
            dynamic_smoke: Attachment {
                path: dynamic_path.clone(),
                sha256: sha256_file(root, &dynamic_path)?,
            },
            preflight: Attachment {
                path: preflight_path.clone(),
                sha256: sha256_file(root, &preflight_path)?,
            },
        },
        summary: Summary {
            stage_count: REQUIRED_STAGES.len(),
            all_stages_passed: true,
            engine_dynamic_load: true,
            agent_rpc_dispatch: true,
            bound_orchestra_rotation: true,
            tamper_recovery: true,
            installer_managed_lifecycle: true,
            residue_cleanup: true,
            release_complete: true,
        },
        limitations: LIMITATIONS.iter().map(|value| (*value).to_string()).collect(),
    };
    validate_report(root, contract, &report)?;
    let mut wrong_platform = report.clone();
    wrong_platform.platform.os = "linux".to_string();
    if validate_report(root, contract, &wrong_platform).is_ok() {
        return Err("Windows validator accepted a Linux report".to_string());
    }
    let mut incomplete = report.clone();
    incomplete.summary.release_complete = false;
    if validate_report(root, contract, &incomplete).is_ok() {
        return Err("Windows validator accepted an incomplete report".to_string());
    }
    fs::write(repo_path(root, &dynamic_path)?, b"{}")
        .map_err(|error| format!("failed to tamper validator attachment: {error}"))?;
    if validate_report(root, contract, &report).is_ok() {
        return Err("Windows validator accepted a tampered attachment".to_string());
    }
    Ok(())
}

fn require_exact_strings(actual: &[String], expected: &[&str], label: &str) -> RunnerResult<()> {
    let expected = expected
        .iter()
        .map(|value| (*value).to_string())
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(format!("Windows qualification {label} drifted"));
    }
    Ok(())
}

fn validate_repo_relative(value: &str, label: &str) -> RunnerResult<()> {
    let path = PathBuf::from(value);
    if value.is_empty()
        || value.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        return Err(format!(
            "{label} must be a portable repository-relative path"
        ));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_commit_digest(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn field<'a>(value: &'a Value, name: &str) -> &'a str {
    value.get(name).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{is_commit_digest, is_sha256, validate_capture_platform};

    #[test]
    fn digests_and_capture_platform_are_strict() {
        assert!(is_sha256(&"a".repeat(64)));
        assert!(!is_sha256(&"A".repeat(64)));
        assert!(is_commit_digest(&"b".repeat(40)));
        assert!(!is_commit_digest(&"g".repeat(40)));
        assert!(validate_capture_platform("windows", "x86_64", "x86_64-pc-windows-msvc").is_ok());
        assert!(validate_capture_platform("windows", "x86_64", "x86_64-pc-windows-gnu").is_err());
    }
}
