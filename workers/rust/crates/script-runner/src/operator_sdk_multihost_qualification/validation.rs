use crate::operator_package_dynamic_smoke::dynamic_smoke_errors;
use crate::qualification_support::{read_json, repo_path};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(super) const CONTRACT_PATH: &str =
    "config/architecture/operator-sdk-multihost-operational-qualification.json";
pub(super) const DEFAULT_REPORT: &str =
    "releases/usability-evidence/2.16.4/operator-sdk-multihost-operational-qualification.json";
pub(super) const CONTRACT_SCHEMA: &str =
    "kyuubiki.operator-sdk-multihost-operational-qualification-contract/v1";
pub(super) const REPORT_SCHEMA: &str =
    "kyuubiki.operator-sdk-multihost-operational-qualification/v1";
pub(super) const QUALIFICATION_ID: &str = "operator-sdk-macos-linux-multihost-operational";
pub(super) const REQUIRED_STAGES: &[&str] = &[
    "template_tests",
    "strict_preflight",
    "template_cdylib_build",
    "engine_dynamic_host_load",
    "agent_dynamic_host_dispatch",
    "installer_managed_agent_lifecycle",
];
pub(super) const REQUIRED_CHECKS: &[&str] = &[
    "local_macos_capture",
    "remote_linux_capture",
    "package_identity_match",
    "sdk_api_match",
    "canonical_stage_match",
    "engine_dynamic_host_both",
    "agent_dispatch_both",
    "installer_lifecycle_both",
    "tamper_recovery_both",
    "remote_cleanup_complete",
    "sensitive_metadata_absent",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageIdentity {
    package_id: String,
    operator_ids: Vec<String>,
    host_version: String,
    sdk_api_version: String,
}

pub(super) fn validate_contract(root: &Path, require_report: bool) -> RunnerResult<()> {
    let contract: Value = read_json(root, CONTRACT_PATH)?;
    expect_string(&contract, "/schema_version", CONTRACT_SCHEMA)?;
    expect_string(&contract, "/qualification_id", QUALIFICATION_ID)?;
    expect_string(
        &contract,
        "/capture/dynamic_smoke_schema",
        "kyuubiki.operator-package-dynamic-smoke/v3",
    )?;
    expect_host(
        &contract,
        "/capture/local_host",
        "local-macos-qualification-host",
        "macos",
        "aarch64",
    )?;
    expect_host(
        &contract,
        "/capture/remote_host",
        "remote-linux-qualification-host",
        "linux",
        "x86_64",
    )?;
    expect_sequence(&contract, "/capture/required_stage_ids", REQUIRED_STAGES)?;
    expect_sequence(
        &contract,
        "/capture/required_completed_platforms",
        &["macos", "linux"],
    )?;
    expect_sequence(&contract, "/capture/deferred_platforms", &["windows"])?;
    if contract
        .pointer("/capture/release_complete")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err("operator SDK multihost contract must keep Windows deferred".to_string());
    }
    expect_sequence(&contract, "/required_checks", REQUIRED_CHECKS)?;
    validate_coordinates(&contract)?;

    let report_path = string_at(&contract, "/retention/report_path")?;
    if report_path != DEFAULT_REPORT {
        return Err(format!("retained report path must be {DEFAULT_REPORT}"));
    }
    expect_string(&contract, "/retention/report_schema", REPORT_SCHEMA)?;
    validate_schema_const(
        root,
        "schemas/operator-sdk-multihost-operational-qualification-contract.schema.json",
        CONTRACT_SCHEMA,
    )?;
    validate_schema_const(
        root,
        "schemas/operator-sdk-multihost-operational-qualification-report.schema.json",
        REPORT_SCHEMA,
    )?;
    if require_report && !repo_path(root, report_path)?.is_file() {
        return Err(format!(
            "retained qualification report is missing: {report_path}"
        ));
    }
    Ok(())
}

pub(super) fn validate_report(root: &Path, report: &Value) -> RunnerResult<()> {
    expect_string(report, "/schema_version", REPORT_SCHEMA)?;
    expect_string(report, "/status", "pass")?;
    expect_string(report, "/qualification_id", QUALIFICATION_ID)?;
    expect_sequence(report, "/scope/completed_platforms", &["macos", "linux"])?;
    expect_sequence(report, "/scope/deferred_platforms", &["windows"])?;
    if report
        .pointer("/scope/release_complete")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err("multihost report cannot claim the deferred Windows lane".to_string());
    }
    let hosts = report
        .get("hosts")
        .and_then(Value::as_array)
        .filter(|hosts| hosts.len() == 2)
        .ok_or("multihost report must contain exactly two host captures")?;
    let local = validate_host(
        root,
        &hosts[0],
        "local-macos-qualification-host",
        "macos",
        "aarch64",
        "local-native",
    )?;
    let remote = validate_host(
        root,
        &hosts[1],
        "remote-linux-qualification-host",
        "linux",
        "x86_64",
        "remote-native",
    )?;
    if local != remote {
        return Err("operator package identity differs across macOS and Linux".to_string());
    }
    validate_package_summary(report, &local)?;
    validate_cleanup(report)?;
    expect_check_set(report)?;
    reject_sensitive_content(report, "report")?;
    Ok(())
}

fn validate_host(
    root: &Path,
    host: &Value,
    role: &str,
    platform: &str,
    architecture: &str,
    capture_kind: &str,
) -> RunnerResult<PackageIdentity> {
    for (field, expected) in [
        ("role", role),
        ("platform", platform),
        ("architecture", architecture),
        ("capture_kind", capture_kind),
    ] {
        if host.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!("{role} must declare {field}={expected}"));
        }
    }
    let report_path = host_string(host, "report_path", role)?;
    let report_bytes = fs::read(repo_path(root, report_path)?)
        .map_err(|error| format!("failed to read {report_path}: {error}"))?;
    expect_digest(host, "report_sha256", &digest(&report_bytes), role)?;
    let dynamic_report: Value = serde_json::from_slice(&report_bytes)
        .map_err(|error| format!("invalid dynamic smoke report {report_path}: {error}"))?;
    if let Some(error) = dynamic_smoke_errors(root, &dynamic_report, report_path).first() {
        return Err(error.clone());
    }

    let preflight_path = host_string(host, "preflight_path", role)?;
    let preflight_bytes = fs::read(repo_path(root, preflight_path)?)
        .map_err(|error| format!("failed to read {preflight_path}: {error}"))?;
    expect_digest(host, "preflight_sha256", &digest(&preflight_bytes), role)?;
    if dynamic_report
        .get("preflight_report")
        .and_then(Value::as_str)
        != Some(preflight_path)
    {
        return Err(format!(
            "{role} dynamic report must bind its retained preflight"
        ));
    }
    let stage_ids = dynamic_report
        .get("stages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|stage| stage.get("id").and_then(Value::as_str))
        .collect::<Vec<_>>();
    if stage_ids != REQUIRED_STAGES {
        return Err(format!(
            "{role} dynamic report has a non-canonical stage sequence"
        ));
    }
    if host.get("stage_count").and_then(Value::as_u64) != Some(6)
        || host.get("all_stages_passed").and_then(Value::as_bool) != Some(true)
        || host
            .get("stage_ids")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
            .as_deref()
            != Some(REQUIRED_STAGES)
    {
        return Err(format!(
            "{role} host summary does not match its stage report"
        ));
    }
    validate_tamper_recovery_text(&dynamic_report, role)?;
    Ok(PackageIdentity {
        package_id: dynamic_string(&dynamic_report, "package_id", role)?,
        operator_ids: dynamic_report
            .get("operator_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
        host_version: dynamic_string(&dynamic_report, "host_version", role)?,
        sdk_api_version: dynamic_string(&dynamic_report, "sdk_api_version", role)?,
    })
}

fn validate_tamper_recovery_text(report: &Value, role: &str) -> RunnerResult<()> {
    let stages = report["stages"]
        .as_array()
        .ok_or("stages must be an array")?;
    for id in [
        "agent_dynamic_host_dispatch",
        "installer_managed_agent_lifecycle",
    ] {
        let description = stages
            .iter()
            .find(|stage| stage.get("id").and_then(Value::as_str) == Some(id))
            .and_then(|stage| stage.get("description"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !description.contains("tamper") || !description.contains("recover") {
            return Err(format!(
                "{role} {id} must prove tamper rejection and recovery"
            ));
        }
    }
    Ok(())
}

fn validate_package_summary(report: &Value, identity: &PackageIdentity) -> RunnerResult<()> {
    let package = report
        .get("package")
        .ok_or("multihost report misses package summary")?;
    if package.get("package_id").and_then(Value::as_str) != Some(&identity.package_id)
        || package.get("host_version").and_then(Value::as_str) != Some(&identity.host_version)
        || package.get("sdk_api_version").and_then(Value::as_str) != Some(&identity.sdk_api_version)
        || package
            .get("operator_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            != identity
                .operator_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
    {
        return Err("multihost package summary does not match host captures".to_string());
    }
    Ok(())
}

fn validate_cleanup(report: &Value) -> RunnerResult<()> {
    if report
        .pointer("/cleanup/remote_run_root_removed")
        .and_then(Value::as_bool)
        != Some(true)
        || report
            .pointer("/cleanup/local_staging_removed")
            .and_then(Value::as_bool)
            != Some(true)
        || report
            .pointer("/cleanup/residue_count")
            .and_then(Value::as_u64)
            != Some(0)
    {
        return Err("multihost qualification cleanup is incomplete".to_string());
    }
    Ok(())
}

fn expect_check_set(report: &Value) -> RunnerResult<()> {
    let checks = report
        .get("checks")
        .and_then(Value::as_array)
        .ok_or("multihost report checks must be an array")?;
    let mut actual = BTreeSet::new();
    for check in checks {
        let id = check
            .get("id")
            .and_then(Value::as_str)
            .ok_or("multihost report check misses id")?;
        if check.get("ok").and_then(Value::as_bool) != Some(true) {
            return Err(format!("multihost report check {id} did not pass"));
        }
        actual.insert(id);
    }
    let expected = REQUIRED_CHECKS.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err("multihost report check set is incomplete".to_string());
    }
    Ok(())
}

pub(super) fn validator_self_test(root: &Path) -> RunnerResult<()> {
    let dir = format!(
        "tmp/operator-sdk-multihost-validator-{}",
        std::process::id()
    );
    let local_report_path = format!("{dir}/local.json");
    let remote_report_path = format!("{dir}/remote.json");
    let local_preflight_path = format!("{dir}/local-preflight.json");
    let remote_preflight_path = format!("{dir}/remote-preflight.json");
    fs::create_dir_all(repo_path(root, &dir)?)
        .map_err(|error| format!("failed to create self-test directory: {error}"))?;
    write_fixture(root, &local_report_path, &local_preflight_path, "dylib")?;
    write_fixture(root, &remote_report_path, &remote_preflight_path, "so")?;
    let report = fixture_qualification(
        root,
        &local_report_path,
        &local_preflight_path,
        &remote_report_path,
        &remote_preflight_path,
    )?;
    validate_report(root, &report)?;
    let mut false_claim = report.clone();
    false_claim["scope"]["release_complete"] = Value::Bool(true);
    if validate_report(root, &false_claim).is_ok() {
        return Err("validator self-test accepted a false Windows completion claim".to_string());
    }
    let mut tampered = report;
    tampered["hosts"][1]["report_sha256"] = Value::String("0".repeat(64));
    if validate_report(root, &tampered).is_ok() {
        return Err("validator self-test accepted a tampered host report".to_string());
    }
    fs::remove_dir_all(repo_path(root, &dir)?)
        .map_err(|error| format!("failed to remove self-test directory: {error}"))?;
    Ok(())
}

fn write_fixture(
    root: &Path,
    report_path: &str,
    preflight_path: &str,
    extension: &str,
) -> RunnerResult<()> {
    let stages = REQUIRED_STAGES
        .iter()
        .map(|id| {
            let description = if matches!(
                *id,
                "agent_dynamic_host_dispatch" | "installer_managed_agent_lifecycle"
            ) {
                format!("{id} rejects tamper and recovers")
            } else {
                format!("{id} qualification stage")
            };
            json!({
                "id": id,
                "description": description,
                "cwd": ".",
                "command": ["cargo", "test", id],
                "status": 0,
                "ok": true
            })
        })
        .collect::<Vec<_>>();
    let report = json!({
        "schema_version": "kyuubiki.operator-package-dynamic-smoke/v3",
        "generated_at": "2026-08-24T00:00:00Z",
        "ok": true,
        "package_id": "operator.template.summary",
        "operator_ids": ["extract.template_summary"],
        "host_version": "2.16.4",
        "sdk_api_version": "kyuubiki.operator-sdk/v1",
        "template_manifest": "workers/rust/templates/operator-crate-template/Cargo.toml",
        "package_manifest": "workers/rust/templates/operator-crate-template/kyuubiki-operator.json",
        "preflight_report": preflight_path,
        "dynamic_library": format!("workers/rust/templates/operator-crate-template/target/debug/libkyuubiki_operator_template.{extension}"),
        "stages": stages
    });
    write_value(root, report_path, &report)?;
    write_value(root, preflight_path, &json!({"status": "pass"}))
}

fn fixture_qualification(
    root: &Path,
    local_report: &str,
    local_preflight: &str,
    remote_report: &str,
    remote_preflight: &str,
) -> RunnerResult<Value> {
    let host = |role: &str,
                platform: &str,
                architecture: &str,
                kind: &str,
                report_path: &str,
                preflight_path: &str|
     -> RunnerResult<Value> {
        Ok(json!({
            "role": role,
            "platform": platform,
            "architecture": architecture,
            "capture_kind": kind,
            "report_path": report_path,
            "report_sha256": digest(&fs::read(repo_path(root, report_path)?).map_err(|error| error.to_string())?),
            "preflight_path": preflight_path,
            "preflight_sha256": digest(&fs::read(repo_path(root, preflight_path)?).map_err(|error| error.to_string())?),
            "stage_ids": REQUIRED_STAGES,
            "stage_count": 6,
            "all_stages_passed": true
        }))
    };
    Ok(json!({
        "schema_version": REPORT_SCHEMA,
        "generated_at_unix_ms": 1,
        "status": "pass",
        "qualification_id": QUALIFICATION_ID,
        "scope": {
            "completed_platforms": ["macos", "linux"],
            "deferred_platforms": ["windows"],
            "release_complete": false
        },
        "package": {
            "package_id": "operator.template.summary",
            "operator_ids": ["extract.template_summary"],
            "host_version": "2.16.4",
            "sdk_api_version": "kyuubiki.operator-sdk/v1"
        },
        "hosts": [
            host("local-macos-qualification-host", "macos", "aarch64", "local-native", local_report, local_preflight)?,
            host("remote-linux-qualification-host", "linux", "x86_64", "remote-native", remote_report, remote_preflight)?
        ],
        "cleanup": {
            "remote_run_root_removed": true,
            "local_staging_removed": true,
            "residue_count": 0
        },
        "checks": REQUIRED_CHECKS.iter().map(|id| json!({"id": id, "ok": true})).collect::<Vec<_>>()
    }))
}

fn validate_coordinates(contract: &Value) -> RunnerResult<()> {
    let coordinates = contract
        .get("target_coordinates")
        .and_then(Value::as_array)
        .ok_or("target_coordinates must be an array")?;
    let actual = coordinates
        .iter()
        .map(|coordinate| {
            (
                coordinate.get("module_id").and_then(Value::as_str),
                coordinate.get("paradigm").and_then(Value::as_str),
                coordinate.get("target_grade").and_then(Value::as_str),
            )
        })
        .collect::<BTreeSet<_>>();
    let expected = [
        "sdk-operator",
        "runtime-engine-solver",
        "runtime-agent-cli",
        "runtime-installer",
    ]
    .into_iter()
    .map(|module| (Some(module), Some("sdk_operator"), Some("operational")))
    .collect::<BTreeSet<_>>();
    if actual != expected {
        return Err("operator SDK multihost target coordinates are incomplete".to_string());
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
        return Err(format!("{path} does not enforce {expected}"));
    }
    Ok(())
}

fn expect_host(
    value: &Value,
    pointer: &str,
    role: &str,
    platform: &str,
    architecture: &str,
) -> RunnerResult<()> {
    for (field, expected) in [
        ("role", role),
        ("platform", platform),
        ("architecture", architecture),
    ] {
        expect_string(value, &format!("{pointer}/{field}"), expected)?;
    }
    Ok(())
}

fn expect_string(value: &Value, pointer: &str, expected: &str) -> RunnerResult<()> {
    if value.pointer(pointer).and_then(Value::as_str) != Some(expected) {
        return Err(format!("{pointer} must be {expected}"));
    }
    Ok(())
}

fn expect_sequence(value: &Value, pointer: &str, expected: &[&str]) -> RunnerResult<()> {
    let actual = value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(format!("{pointer} does not match the required sequence"));
    }
    Ok(())
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> RunnerResult<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{pointer} must be a non-empty string"))
}

fn host_string<'a>(host: &'a Value, field: &str, role: &str) -> RunnerResult<&'a str> {
    host.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{role} misses {field}"))
}

fn dynamic_string(report: &Value, field: &str, role: &str) -> RunnerResult<String> {
    host_string(report, field, role).map(ToString::to_string)
}

fn expect_digest(host: &Value, field: &str, expected: &str, role: &str) -> RunnerResult<()> {
    let actual = host_string(host, field, role)?;
    if actual != expected || !valid_digest(actual) {
        return Err(format!("{role} {field} does not match retained content"));
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_value(root: &Path, path: &str, value: &Value) -> RunnerResult<()> {
    let rendered = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {path}: {error}"))?;
    fs::write(repo_path(root, path)?, format!("{rendered}\n"))
        .map_err(|error| format!("failed to write {path}: {error}"))
}

fn reject_sensitive_content(value: &Value, location: &str) -> RunnerResult<()> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if matches!(
                    key.as_str(),
                    "host_address" | "hostname" | "username" | "credential" | "absolute_host_path"
                ) {
                    return Err(format!("{location} retains forbidden field {key}"));
                }
                reject_sensitive_content(child, &format!("{location}.{key}"))?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_sensitive_content(child, &format!("{location}[{index}]"))?;
            }
        }
        Value::String(text)
            if text.starts_with('/')
                || text.starts_with("~/")
                || text.contains("/Users/")
                || text.contains("/home/")
                || text.contains("192.168.")
                || text.contains("kyuubiki-lab") =>
        {
            return Err(format!("{location} retains host-specific content"));
        }
        _ => {}
    }
    Ok(())
}
