use crate::operator_package_dynamic_smoke::{
    dynamic_smoke_errors, run_operator_package_dynamic_smoke,
};
use crate::qualification_support::{generated_at_unix_ms, read_json, repo_path, write_json};
use crate::{RepoPaths, RunnerResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

mod validation;

use validation::{
    load_and_validate_contract, sha256_file, source_tree_digest, validate_capture_platform,
    validate_report, validator_self_test,
};

const CONTRACT_PATH: &str =
    "config/architecture/operator-sdk-windows-operational-qualification.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.operator-sdk-windows-operational-qualification-contract/v2";
const REPORT_SCHEMA: &str = "kyuubiki.operator-sdk-windows-operational-qualification/v2";
const EXECUTION_ABI: &str = kyuubiki_operator_sdk::OPERATOR_JSON_ABI_SCHEMA_VERSION;
const QUALIFICATION_ID: &str = "operator-sdk-windows-installed-agent-operational";
const WORKFLOW_REPORT_PATH: &str = "tmp/operator-sdk-windows-current/operator-sdk-windows-installed-agent-operational-qualification.json";
const STAGED_DYNAMIC_REPORT: &str = "dynamic-smoke.json";
const STAGED_PREFLIGHT_REPORT: &str = "operator-package-dynamic-preflight.json";
const LIMITATIONS: &[&str] = &[
    "Evidence covers the x86_64-pc-windows-msvc Operator SDK and installed Agent path only.",
    "Desktop shell signing and Windows application distribution remain separate release gates.",
];

#[derive(Debug, Clone, Deserialize)]
struct QualificationContract {
    schema_version: String,
    qualification_id: String,
    execution_abi: String,
    platform: Platform,
    required_stages: Vec<String>,
    source_files: Vec<String>,
    provenance: ProvenancePolicy,
    report: ReportPolicy,
    workflow: WorkflowPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Platform {
    os: String,
    arch: String,
    abi: String,
    dynamic_library_suffix: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ProvenancePolicy {
    allowed_providers: Vec<String>,
    required_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReportPolicy {
    schema_version: String,
    schema_path: String,
    default_path: String,
    dynamic_smoke_file: String,
    preflight_file: String,
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowPolicy {
    path: String,
    artifact_name: String,
    capture_command: String,
    generated_report_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QualificationReport {
    schema_version: String,
    qualification_id: String,
    execution_abi: String,
    generated_at_unix_ms: u128,
    status: String,
    source_tree_sha256: String,
    platform: Platform,
    provenance: Provenance,
    attachments: Attachments,
    summary: Summary,
    limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Provenance {
    provider: String,
    repository: String,
    commit_sha: String,
    run_id: String,
    runner_id: String,
    workflow_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Attachments {
    dynamic_smoke: Attachment,
    preflight: Attachment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Attachment {
    path: String,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Summary {
    stage_count: usize,
    all_stages_passed: bool,
    stable_json_c_abi: bool,
    engine_dynamic_load: bool,
    agent_rpc_dispatch: bool,
    bound_orchestra_rotation: bool,
    tamper_recovery: bool,
    installer_managed_lifecycle: bool,
    residue_cleanup: bool,
    release_complete: bool,
}

pub(crate) fn run_qualify(paths: &RepoPaths, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = QualifyOptions::parse(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner qualify-operator-sdk-windows-operational [--out path]"
        );
        return Ok(0);
    }
    let contract = load_and_validate_contract(&paths.root)?;
    validate_capture_platform(env::consts::OS, env::consts::ARCH, &rust_host_triple()?)?;
    let output = options
        .out
        .unwrap_or_else(|| contract.report.default_path.clone());
    repo_path(&paths.root, &output)?;

    let staging_relative = format!(
        "tmp/operator-sdk-windows-qualification-{}",
        std::process::id()
    );
    let staging_root = repo_path(&paths.root, &staging_relative)?;
    fs::create_dir_all(&staging_root)
        .map_err(|error| format!("failed to create {}: {error}", staging_root.display()))?;
    let result = capture(paths, &contract, &output, &staging_root);
    let cleanup = fs::remove_dir_all(&staging_root)
        .map_err(|error| format!("failed to remove {}: {error}", staging_root.display()));
    match (result, cleanup) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(error), Ok(())) => Err(error),
        (Ok(status), Err(error)) => Err(format!(
            "Windows qualification exited with status {status}; {error}"
        )),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; {cleanup_error}")),
    }
}

pub(crate) fn run_check(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = CheckOptions::parse(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner check-operator-sdk-windows-operational-qualification [--self-test] [--verify-report path]"
        );
        return Ok(0);
    }
    let contract = load_and_validate_contract(root)?;
    if options.self_test {
        validator_self_test(root, &contract)?;
        println!("Operator SDK Windows qualification self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let report_path = options
        .report
        .unwrap_or_else(|| contract.report.default_path.clone());
    let report: QualificationReport = read_json(root, &report_path)?;
    validate_report(root, &contract, &report)?;
    println!("Operator SDK Windows qualification report passed: {report_path}");
    Ok(0)
}

fn capture(
    paths: &RepoPaths,
    contract: &QualificationContract,
    output: &str,
    staging_root: &Path,
) -> RunnerResult<u8> {
    let staged_dynamic = staging_root.join(STAGED_DYNAMIC_REPORT);
    let staged_preflight = staging_root.join(STAGED_PREFLIGHT_REPORT);
    let status = run_operator_package_dynamic_smoke(
        paths,
        vec![
            OsString::from("--out"),
            staged_dynamic.clone().into_os_string(),
        ],
    )?;
    if status != 0 {
        return Ok(status);
    }

    let mut dynamic_report = read_json_path(&staged_dynamic, "staged Windows dynamic smoke")?;
    let errors = dynamic_smoke_errors(&paths.root, &dynamic_report, "Windows dynamic smoke");
    if let Some(error) = errors.first() {
        return Err(error.clone());
    }
    let preflight: Value = read_json_path(&staged_preflight, "staged Windows preflight")?;
    let provenance = capture_provenance()?;
    let source_tree_sha256 = source_tree_digest(&paths.root, &contract.source_files)?;

    let output_parent = Path::new(output).parent().unwrap_or_else(|| Path::new("."));
    let dynamic_path = portable_join(output_parent, &contract.report.dynamic_smoke_file);
    let preflight_path = portable_join(output_parent, &contract.report.preflight_file);
    dynamic_report["preflight_report"] = Value::String(preflight_path.clone());
    write_json(&paths.root, &dynamic_path, &dynamic_report)?;
    write_json(&paths.root, &preflight_path, &preflight)?;

    let report = QualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        execution_abi: EXECUTION_ABI.to_string(),
        generated_at_unix_ms: generated_at_unix_ms()?,
        status: "pass".to_string(),
        source_tree_sha256,
        platform: contract.platform.clone(),
        provenance,
        attachments: Attachments {
            dynamic_smoke: Attachment {
                path: dynamic_path.clone(),
                sha256: sha256_file(&paths.root, &dynamic_path)?,
            },
            preflight: Attachment {
                path: preflight_path.clone(),
                sha256: sha256_file(&paths.root, &preflight_path)?,
            },
        },
        summary: Summary {
            stage_count: contract.required_stages.len(),
            all_stages_passed: true,
            stable_json_c_abi: true,
            engine_dynamic_load: true,
            agent_rpc_dispatch: true,
            bound_orchestra_rotation: true,
            tamper_recovery: true,
            installer_managed_lifecycle: true,
            residue_cleanup: true,
            release_complete: true,
        },
        limitations: LIMITATIONS
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    };
    write_json(&paths.root, output, &report)?;
    validate_report(&paths.root, contract, &report)?;
    println!("Operator SDK Windows operational qualification passed: {output}");
    Ok(0)
}

fn capture_provenance() -> RunnerResult<Provenance> {
    if env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        return Ok(Provenance {
            provider: "github-actions".to_string(),
            repository: required_env("GITHUB_REPOSITORY")?,
            commit_sha: required_env("GITHUB_SHA")?,
            run_id: format!(
                "{}:{}",
                required_env("GITHUB_RUN_ID")?,
                required_env("GITHUB_RUN_ATTEMPT")?
            ),
            runner_id: required_env("RUNNER_NAME")?,
            workflow_ref: required_env("GITHUB_WORKFLOW_REF")?,
        });
    }
    Ok(Provenance {
        provider: "self-hosted-windows".to_string(),
        repository: required_env("KYUUBIKI_QUALIFICATION_REPOSITORY")?,
        commit_sha: required_env("KYUUBIKI_QUALIFICATION_SOURCE_COMMIT")?,
        run_id: required_env("KYUUBIKI_QUALIFICATION_RUN_ID")?,
        runner_id: required_env("KYUUBIKI_QUALIFICATION_RUNNER_ID")?,
        workflow_ref: required_env("KYUUBIKI_QUALIFICATION_WORKFLOW_REF")?,
    })
}

fn rust_host_triple() -> RunnerResult<String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| format!("failed to inspect rustc host triple: {error}"))?;
    if !output.status.success() {
        return Err("rustc -vV failed while inspecting the Windows ABI".to_string());
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(ToString::to_string)
        .ok_or_else(|| "rustc -vV did not report a host triple".to_string())
}

fn required_env(name: &str) -> RunnerResult<String> {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} is required for retained Windows qualification provenance"))
}

fn portable_join(parent: &Path, file_name: &str) -> String {
    parent.join(file_name).to_string_lossy().replace('\\', "/")
}

fn read_json_path(path: &Path, label: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {label} {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid {label}: {error}"))
}

#[derive(Default)]
struct QualifyOptions {
    help: bool,
    out: Option<String>,
}

impl QualifyOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--out" => options.out = Some(required_path(&mut args, "--out")?),
                other => return Err(format!("unknown Windows qualification argument: {other}")),
            }
        }
        if let Some(output) = &options.out {
            validate_relative_path(output, "--out")?;
        }
        Ok(options)
    }
}

#[derive(Default)]
struct CheckOptions {
    help: bool,
    self_test: bool,
    report: Option<String>,
}

impl CheckOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--self-test" => options.self_test = true,
                "--verify-report" => {
                    options.report = Some(required_path(&mut args, "--verify-report")?)
                }
                other => return Err(format!("unknown Windows report argument: {other}")),
            }
        }
        if let Some(report) = &options.report {
            validate_relative_path(report, "--verify-report")?;
        }
        Ok(options)
    }
}

fn required_path(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    args.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a repository-relative path"))
}

fn validate_relative_path(value: &str, flag: &str) -> RunnerResult<()> {
    let path = Path::new(value);
    if path.is_absolute()
        || value.contains('\\')
        || path
            .components()
            .any(|component| component.as_os_str() == "..")
    {
        return Err(format!(
            "{flag} must be a portable repository-relative path"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{portable_join, validate_relative_path};
    use std::path::Path;

    #[test]
    fn evidence_paths_are_portable_and_repository_relative() {
        assert_eq!(
            portable_join(Path::new("releases/evidence"), "report.json"),
            "releases/evidence/report.json"
        );
        assert!(validate_relative_path("releases/evidence/report.json", "--out").is_ok());
        assert!(validate_relative_path("../report.json", "--out").is_err());
        assert!(validate_relative_path("C:\\report.json", "--out").is_err());
    }
}
