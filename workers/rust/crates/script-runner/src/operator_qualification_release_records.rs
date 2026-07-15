use crate::{RunnerResult, run_command};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod review_decision;

use review_decision::{validate_review_decision_path, validate_review_status_transition};

const DEFAULT_INPUT: &str = "releases/qualification-records/1.20.0.json";
const SCHEMA_VERSION: &str = "kyuubiki.operator-qualification-release-records/v1";
const ROADMAP_PATH: &str = "config/operator-qualification-roadmap.json";
const KITS_PATH: &str = "config/operator-qualification-evidence-kits.json";

pub(crate) fn run_check_operator_qualification_release_records(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let input = parse_args(args)?;
    let (absolute, relative) = repo_local_path(root, &input, "--in")?;
    let records = read_json_path(&absolute, &relative)?;
    match validate_records(root, &records) {
        Ok(()) => {
            println!("operator qualification release records ok: {relative}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("operator qualification release records check failed: {issue}");
            Ok(1)
        }
    }
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<String> {
    let mut input = DEFAULT_INPUT.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner check-operator-qualification-release-records [--in releases/qualification-records/1.20.0.json]"
                );
                return Ok(input);
            }
            "--in" => {
                let Some(value) = iter.next() else {
                    return Err("--in requires a repo-local path".to_string());
                };
                input = value.to_string_lossy().to_string();
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(input)
}

fn validate_records(root: &Path, records: &Value) -> RunnerResult<()> {
    assert_eq(
        field(records, "schema_version"),
        SCHEMA_VERSION,
        "schema_version",
    )?;
    let snapshot_path = field(records, "snapshot_path");
    let snapshot = read_json(root, snapshot_path)?;
    assert_eq(
        field(&snapshot, "version"),
        field(records, "release_version"),
        "snapshot version",
    )?;
    let roadmap = read_json(root, ROADMAP_PATH)?;
    let kits = read_json(root, KITS_PATH)?;
    assert_eq(
        field(&roadmap, "version_line"),
        field(records, "version_line"),
        "roadmap version_line",
    )?;
    assert_eq(
        field(&kits, "version_line"),
        field(records, "version_line"),
        "evidence kit version_line",
    )?;
    let candidates = array(&roadmap, "candidates")
        .into_iter()
        .map(|candidate| {
            (
                field(candidate, "candidate_id").to_string(),
                CandidateGate {
                    target_level: field(candidate, "target_level").to_string(),
                    release_gate_impact: field(candidate, "release_gate_impact").to_string(),
                    graduation_gate: field(candidate, "graduation_gate").to_string(),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let requirements = release_requirements_by_candidate(&kits);
    let mut seen = HashSet::new();
    let release_version = field(records, "release_version");
    for record in array(records, "records") {
        validate_record(root, release_version, record, &candidates, &requirements)?;
        if !seen.insert(field(record, "candidate_id").to_string()) {
            return Err(format!(
                "duplicate candidate_id {}",
                field(record, "candidate_id")
            ));
        }
    }
    Ok(())
}

fn validate_record(
    root: &Path,
    release_version: &str,
    record: &Value,
    candidates: &HashMap<String, CandidateGate>,
    requirements: &HashMap<String, ReleaseRequirement>,
) -> RunnerResult<()> {
    let candidate_id = field(record, "candidate_id");
    let Some(candidate) = candidates.get(candidate_id) else {
        return Err(format!(
            "{candidate_id}: release record has no roadmap candidate"
        ));
    };
    if !matches!(
        field(record, "status"),
        "staged_for_review" | "attached_to_release" | "rejected"
    ) {
        return Err(format!("{candidate_id}: unsupported release record status"));
    }
    let review_status = field(record, "review_status");
    if !matches!(
        review_status,
        "pending_signoff" | "approved" | "blocked_scope" | "rejected"
    ) {
        return Err(format!("{candidate_id}: unsupported release review_status"));
    }
    if field(record, "review_gate").is_empty() {
        return Err(format!("{candidate_id}: review_gate must be non-empty"));
    }
    if field(record, "review_gate") != candidate.graduation_gate {
        return Err(format!(
            "{candidate_id}: review_gate must match roadmap graduation_gate"
        ));
    }
    if field(record, "status") == "rejected" && review_status != "rejected" {
        return Err(format!(
            "{candidate_id}: rejected release records must use review_status=rejected"
        ));
    }
    validate_review_status_transition(candidate_id, review_status, candidate)?;
    validate_review_decision_path(root, release_version, record)?;
    let Some(requirement) = requirements.get(candidate_id) else {
        return Err(format!(
            "{candidate_id}: no release-retained evidence requirement"
        ));
    };
    assert_eq(
        field(record, "capture_command"),
        &requirement.capture_command,
        "capture_command",
    )?;
    assert_eq(
        field(record, "check_command"),
        &requirement.check_command,
        "check_command",
    )?;
    let evidence_path = field(record, "evidence_path");
    let (evidence_absolute, _relative) = repo_local_path(root, evidence_path, "evidence_path")?;
    if !evidence_absolute.exists() {
        return Err(format!(
            "{candidate_id}: evidence_path does not exist: {evidence_path}"
        ));
    }
    run_check_command(root, field(record, "check_command"), evidence_path)?;
    Ok(())
}

#[derive(Debug)]
struct ReleaseRequirement {
    capture_command: String,
    check_command: String,
}

#[derive(Debug)]
pub(super) struct CandidateGate {
    pub(super) target_level: String,
    pub(super) release_gate_impact: String,
    graduation_gate: String,
}

fn release_requirements_by_candidate(kits: &Value) -> HashMap<String, ReleaseRequirement> {
    let mut requirements = HashMap::new();
    for kit in array(kits, "kits") {
        for artifact in array(kit, "artifact_requirements") {
            if field(artifact, "kind") == "release_retained_regression_output" {
                requirements.insert(
                    field(kit, "candidate_id").to_string(),
                    ReleaseRequirement {
                        capture_command: field(artifact, "artifact_command").to_string(),
                        check_command: field(artifact, "artifact_check_command").to_string(),
                    },
                );
            }
        }
    }
    requirements
}

fn run_check_command(root: &Path, command: &str, evidence_path: &str) -> RunnerResult<()> {
    let target = command
        .strip_prefix("make ")
        .ok_or_else(|| format!("check_command must be a make target: {command}"))?;
    if target.split_whitespace().count() != 1 {
        return Err(format!(
            "check_command must be a single make target: {command}"
        ));
    }
    let status = run_command(
        root,
        "make",
        [
            OsString::from(target),
            OsString::from(format!("IN={evidence_path}")),
        ],
    )?;
    if status != 0 {
        return Err(format!(
            "check_command failed for {evidence_path}: {command}"
        ));
    }
    Ok(())
}

pub(super) fn assert_eq(actual: &str, expected: &str, context: &str) -> RunnerResult<()> {
    if actual != expected {
        return Err(format!("{context} must be {expected}, got {actual}"));
    }
    Ok(())
}

pub(super) fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let (absolute, relative) = repo_local_path(root, relative_path, "path")?;
    read_json_path(&absolute, &relative)
}

fn read_json_path(path: &Path, label: &str) -> RunnerResult<Value> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {label}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label}: invalid json: {error}"))
}

fn repo_local_path(
    root: &Path,
    relative_path: &str,
    label: &str,
) -> RunnerResult<(PathBuf, String)> {
    let absolute = root.join(relative_path);
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("{label} must stay inside the repository"))?;
    if relative.starts_with("..") || relative.is_absolute() {
        return Err(format!("{label} must stay inside the repository"));
    }
    let relative = relative
        .to_str()
        .ok_or_else(|| format!("{label} must be valid UTF-8"))?
        .replace('\\', "/");
    Ok((absolute, relative))
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

pub(super) fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
