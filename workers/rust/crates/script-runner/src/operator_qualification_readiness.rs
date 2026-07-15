use crate::{RunnerResult, native_time::utc_iso_timestamp};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod args;
mod release_records;
mod self_test;
#[cfg(test)]
mod unit_tests;

use args::{parse_check_args, parse_out};
use release_records::{ReleaseRecord, release_records_by_candidate};

const DEFAULT_OUT: &str = "tmp/operator-qualification-readiness.json";
const SCHEMA_PATH: &str = "schemas/operator-qualification-readiness.schema.json";
const ROADMAP_PATH: &str = "config/operator-qualification-roadmap.json";
const EVIDENCE_KITS_PATH: &str = "config/operator-qualification-evidence-kits.json";
const VALIDATION_PROFILES_PATH: &str = "config/operator-validation-profiles.json";
const SCHEMA_VERSION: &str = "kyuubiki.operator-qualification-readiness/v1";
#[rustfmt::skip]
const ALLOWED_ACTION_KINDS: &[&str] = &["collect_artifact", "restore_or_generate_artifact", "run_command", "review"];
const TARGET_LEVELS: &[&str] = &["baseline", "review", "qualification"];
const EVIDENCE_PHASES: &[&str] = &["planned", "collecting", "ready_for_review", "blocked"];
const RELEASE_GATE_IMPACTS: &[&str] = &["release_blocker", "release_watch", "experimental_only"];
const RELEASE_REVIEW_STATUSES: &[&str] = &[
    "missing",
    "pending_signoff",
    "approved",
    "blocked_scope",
    "rejected",
];

pub(crate) fn run_build_operator_qualification_readiness(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let out = parse_out(args)?;
    let (absolute, relative) = repo_local_path(root, &out, "--out")?;
    let report = build_report(root)?;
    write_json(&absolute, &report)?;
    println!("operator qualification readiness wrote {relative}");
    Ok(0)
}

pub(crate) fn run_check_operator_qualification_readiness(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_check_args(args)?;
    if options.self_test {
        self_test::run_self_test(root)?;
        println!("operator qualification readiness check self-test passed");
        return Ok(0);
    }
    let (absolute, relative) = repo_local_path(root, &options.input, "--in")?;
    let report = read_json_path(&absolute, &relative)?;
    if let Some(issue) = readiness_errors(root, &report, &relative)?
        .into_iter()
        .next()
    {
        eprintln!("operator qualification readiness check failed: {issue}");
        return Ok(1);
    }
    println!("operator qualification readiness check passed: {relative}");
    Ok(0)
}

fn build_report(root: &Path) -> RunnerResult<Value> {
    let roadmap = read_json(root, ROADMAP_PATH)?;
    let kits = read_json(root, EVIDENCE_KITS_PATH)?;
    if field(&roadmap, "version_line") != field(&kits, "version_line") {
        return Err("roadmap and evidence kits version_line must match".to_string());
    }
    let kit_by_candidate = array(&kits, "kits")
        .into_iter()
        .map(|kit| (field(kit, "candidate_id").to_string(), kit))
        .collect::<HashMap<_, _>>();
    let release_records = release_records_by_candidate(root)?;
    let validation_profiles = validation_profiles_by_candidate(root)?;
    let candidates = array(&roadmap, "candidates")
        .into_iter()
        .map(|candidate| {
            readiness_for(
                root,
                candidate,
                kit_by_candidate
                    .get(field(candidate, "candidate_id"))
                    .copied(),
                release_records.get(field(candidate, "candidate_id")),
                validation_profiles
                    .get(field(candidate, "candidate_id"))
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect::<RunnerResult<Vec<_>>>()?;
    let next_actions = build_next_actions(&candidates);
    let profile_count = candidates
        .iter()
        .map(|candidate| array(candidate, "validation_profiles").len())
        .sum::<usize>();
    let release_profiles = candidates
        .iter()
        .flat_map(|candidate| array(candidate, "validation_profiles"))
        .filter(|profile| field(profile, "profile_role") == "release_candidate")
        .count();
    Ok(json!({
        "schema_version": SCHEMA_VERSION,
        "version_line": field(&roadmap, "version_line"),
        "generated_at_utc": utc_iso_timestamp(),
        "summary": {
            "candidates": candidates.len(),
            "collecting": candidates.iter().filter(|candidate| field(candidate, "status") == "collecting").count(),
            "planned": candidates.iter().filter(|candidate| field(candidate, "status") == "planned").count(),
            "with_entries": candidates.iter().filter(|candidate| candidate.pointer("/artifact_counts/present").and_then(Value::as_u64).unwrap_or(0) > 0
                || candidate.pointer("/artifact_counts/command_available").and_then(Value::as_u64).unwrap_or(0) > 0).count(),
            "not_started": candidates.iter().filter(|candidate| candidate.pointer("/artifact_counts/not_started").and_then(Value::as_u64)
                == candidate.pointer("/artifact_counts/total").and_then(Value::as_u64)).count(),
            "broken": candidates.iter().filter(|candidate| field(candidate, "readiness") == "broken").count(),
            "validation_profile_count": profile_count,
            "release_candidate_profiles": release_profiles,
            "component_profiles": profile_count.saturating_sub(release_profiles),
            "candidates_missing_release_profile": candidates.iter().filter(|candidate| !array(candidate, "validation_profiles")
                .iter().any(|profile| field(profile, "profile_role") == "release_candidate")).count(),
            "next_action_count": next_actions.len(),
            "target_levels": count_by(&candidates, "target_level", TARGET_LEVELS),
            "evidence_phases": count_by(&candidates, "evidence_phase", EVIDENCE_PHASES),
            "release_gate_impacts": count_by(&candidates, "release_gate_impact", RELEASE_GATE_IMPACTS),
            "release_review_statuses": count_release_review_statuses(&candidates),
        },
        "next_actions": next_actions,
        "candidates": candidates,
    }))
}

fn readiness_for(
    root: &Path,
    candidate: &Value,
    kit: Option<&Value>,
    release_record: Option<&ReleaseRecord>,
    validation_profiles: Vec<Value>,
) -> RunnerResult<Value> {
    let artifacts = kit
        .map(|kit| array(kit, "artifact_requirements"))
        .unwrap_or_default()
        .into_iter()
        .map(|requirement| artifact_state(root, requirement, release_record))
        .collect::<RunnerResult<Vec<_>>>()?;
    let present = count_state(&artifacts, "present");
    let commands = count_state(&artifacts, "command_available");
    let missing = count_state(&artifacts, "missing");
    let not_started = count_state(&artifacts, "not_started");
    let actionable = artifacts
        .iter()
        .filter(|artifact| field(artifact, "state") != "not_started")
        .count();
    let readiness = if kit.is_some_and(|kit| field(kit, "status") == "blocked") {
        "blocked"
    } else if missing > 0 {
        "broken"
    } else if not_started == 0 && !artifacts.is_empty() {
        "ready_for_review"
    } else if actionable > 0 {
        "partially_collecting"
    } else {
        "planned"
    };
    Ok(json!({
        "candidate_id": field(candidate, "candidate_id"),
        "priority": field(candidate, "priority"),
        "domain": field(candidate, "domain"),
        "target_level": field(candidate, "target_level"),
        "evidence_phase": field(candidate, "evidence_phase"),
        "status": kit.map(|kit| field(kit, "status")).unwrap_or("missing_kit"),
        "readiness": readiness,
        "operator_ids": candidate.get("operator_ids").cloned().unwrap_or(Value::Array(Vec::new())),
        "artifact_counts": {
            "total": artifacts.len(),
            "present": present,
            "command_available": commands,
            "missing": missing,
            "not_started": not_started,
        },
        "artifacts": artifacts,
        "validation_profiles": validation_profiles,
        "primary_blocker": field(candidate, "primary_blocker"),
        "evidence_gaps": candidate.get("evidence_gaps").cloned().unwrap_or(Value::Array(Vec::new())),
        "graduation_gate": field(candidate, "graduation_gate"),
        "preferred_validation_lane": field(candidate, "preferred_validation_lane"),
        "release_gate_impact": field(candidate, "release_gate_impact"),
    }))
}

fn validation_profiles_by_candidate(root: &Path) -> RunnerResult<HashMap<String, Vec<Value>>> {
    let source = read_json(root, VALIDATION_PROFILES_PATH)?;
    let mut grouped: HashMap<String, Vec<Value>> = HashMap::new();
    for profile in array(&source, "profiles") {
        let candidate_id = field(profile, "qualification_candidate_id");
        grouped.entry(candidate_id.to_string()).or_default().push(json!({
            "profile_id": field(profile, "profile_id"),
            "profile_role": field(profile, "profile_role"),
            "trust_goal": field(profile, "trust_goal"),
            "operator_count": array(profile, "operators").len(),
            "command_count": array(profile, "commands").len(),
        }));
    }
    for profiles in grouped.values_mut() {
        profiles.sort_by(|left, right| {
            field(left, "profile_role")
                .cmp(field(right, "profile_role"))
                .then(field(left, "profile_id").cmp(field(right, "profile_id")))
        });
    }
    Ok(grouped)
}

fn artifact_state(
    root: &Path,
    requirement: &Value,
    release_record: Option<&ReleaseRecord>,
) -> RunnerResult<Value> {
    let command = field(requirement, "artifact_command");
    if !command.is_empty() {
        let release_state = release_record
            .filter(|record| record.capture_command == command)
            .map(|record| record.status.as_str());
        return Ok(json!({
            "artifact_id": field(requirement, "artifact_id"),
            "kind": field(requirement, "kind"),
            "state": if release_state.is_some() { "present" } else { "command_available" },
            "path": optional_field(requirement, "artifact_path"),
            "command": command,
            "check_command": optional_field(requirement, "artifact_check_command"),
            "release_record_state": release_state.unwrap_or("missing"),
            "release_record_path": release_record.map(|record| record.evidence_path.as_str()).unwrap_or(""),
            "release_review_status": release_record.map(|record| record.review_status.as_str()).unwrap_or("missing"),
            "release_review_gate": release_record.map(|record| record.review_gate.as_str()).unwrap_or(""),
            "release_review_decision_path": release_record.map(|record| record.review_decision_path.as_str()).unwrap_or(""),
            "gate": field(requirement, "gate"),
        }));
    }
    let artifact_path = field(requirement, "artifact_path");
    if !artifact_path.is_empty() {
        return Ok(json!({
            "artifact_id": field(requirement, "artifact_id"),
            "kind": field(requirement, "kind"),
            "state": if root.join(artifact_path).exists() { "present" } else { "missing" },
            "path": artifact_path,
            "gate": field(requirement, "gate"),
        }));
    }
    Ok(json!({
        "artifact_id": field(requirement, "artifact_id"),
        "kind": field(requirement, "kind"),
        "state": "not_started",
        "gate": field(requirement, "gate"),
    }))
}

fn build_next_actions(candidates: &[Value]) -> Vec<Value> {
    let mut actions = candidates
        .iter()
        .map(|candidate| {
            let artifact = first_actionable_artifact(candidate);
            let validation_profiles = array(candidate, "validation_profiles");
            let release_profiles = validation_profiles
                .iter()
                .filter(|profile| field(profile, "profile_role") == "release_candidate")
                .count();
            json!({
                "candidate_id": field(candidate, "candidate_id"),
                "priority": field(candidate, "priority"),
                "target_level": field(candidate, "target_level"),
                "evidence_phase": field(candidate, "evidence_phase"),
                "readiness": field(candidate, "readiness"),
                "action_kind": action_kind_for_artifact(artifact),
                "artifact_id": artifact.and_then(|artifact| artifact.get("artifact_id")).cloned().unwrap_or(Value::Null),
                "artifact_state": artifact.and_then(|artifact| artifact.get("state")).cloned().unwrap_or(Value::Null),
                "artifact_kind": artifact.and_then(|artifact| artifact.get("kind")).cloned().unwrap_or(Value::Null),
                "command": artifact.and_then(|artifact| artifact.get("command")).cloned().unwrap_or(Value::Null),
                "check_command": artifact.and_then(|artifact| artifact.get("check_command")).cloned().unwrap_or(Value::Null),
                "path": artifact.and_then(|artifact| artifact.get("path")).cloned().unwrap_or(Value::Null),
                "gate": artifact.and_then(|artifact| artifact.get("gate")).and_then(Value::as_str).unwrap_or_else(|| field(candidate, "graduation_gate")),
                "review_reason": if artifact.is_none() { Value::from(field(candidate, "primary_blocker")) } else { Value::Null },
                "validation_profile_count": validation_profiles.len(),
                "release_candidate_profile_count": release_profiles,
                "preferred_validation_lane": field(candidate, "preferred_validation_lane"),
                "release_gate_impact": field(candidate, "release_gate_impact"),
            })
        })
        .filter(|action| {
            action.get("artifact_id") != Some(&Value::Null)
                || field(action, "readiness") != "collecting_with_entries"
        })
        .collect::<Vec<_>>();
    actions.sort_by(compare_actions);
    actions
}

pub(super) fn readiness_errors(
    root: &Path,
    report: &Value,
    relative_input: &str,
) -> RunnerResult<Vec<String>> {
    let mut errors = Vec::new();
    let schema = read_json(root, SCHEMA_PATH)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(SCHEMA_VERSION)
    {
        errors.push(format!("{SCHEMA_PATH}: schema_version const is wrong"));
    }
    if field(report, "schema_version") != SCHEMA_VERSION {
        errors.push(format!("{relative_input}: unexpected schema_version"));
    }
    if field(report, "version_line").is_empty() {
        errors.push(format!("{relative_input}: version_line must be non-empty"));
    }
    if array(report, "candidates").is_empty() {
        errors.push(format!("{relative_input}: candidates must be non-empty"));
    }
    let Some(next_actions) = report.get("next_actions").and_then(Value::as_array) else {
        errors.push(format!("{relative_input}: next_actions must be an array"));
        return Ok(errors);
    };
    let candidates = array(report, "candidates");
    check_summary(
        report,
        &candidates,
        next_actions,
        relative_input,
        &mut errors,
    );
    for (index, action) in next_actions.iter().enumerate() {
        errors.extend(action_errors(action, index));
    }
    for index in 1..next_actions.len() {
        if compare_actions(&next_actions[index - 1], &next_actions[index]) == Ordering::Greater {
            errors.push(format!(
                "{relative_input}: next_actions must stay priority/readiness sorted"
            ));
        }
    }
    Ok(errors)
}

fn check_summary(
    report: &Value,
    candidates: &[&Value],
    next_actions: &[Value],
    relative_input: &str,
    errors: &mut Vec<String>,
) {
    if report
        .pointer("/summary/next_action_count")
        .and_then(Value::as_u64)
        != Some(next_actions.len() as u64)
    {
        errors.push(format!(
            "{relative_input}: summary.next_action_count must match next_actions length"
        ));
    }
    if report
        .pointer("/summary/candidates")
        .and_then(Value::as_u64)
        != Some(candidates.len() as u64)
    {
        errors.push(format!(
            "{relative_input}: summary.candidates must match candidates length"
        ));
    }
    let collecting = candidates
        .iter()
        .filter(|candidate| field(candidate, "status") == "collecting")
        .count() as u64;
    let planned = candidates
        .iter()
        .filter(|candidate| field(candidate, "status") == "planned")
        .count() as u64;
    let with_entries = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .pointer("/artifact_counts/present")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                > 0
                || candidate
                    .pointer("/artifact_counts/command_available")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    > 0
        })
        .count() as u64;
    let broken = candidates
        .iter()
        .filter(|candidate| field(candidate, "readiness") == "broken")
        .count() as u64;
    let profile_count = candidates
        .iter()
        .map(|candidate| array(candidate, "validation_profiles").len() as u64)
        .sum::<u64>();
    let release_profiles = candidates
        .iter()
        .flat_map(|candidate| array(candidate, "validation_profiles"))
        .filter(|profile| field(profile, "profile_role") == "release_candidate")
        .count() as u64;
    let missing_release_profiles = candidates
        .iter()
        .filter(|candidate| {
            !array(candidate, "validation_profiles")
                .iter()
                .any(|profile| field(profile, "profile_role") == "release_candidate")
        })
        .count() as u64;
    for (key, expected, label) in [
        ("collecting", collecting, "summary.collecting is stale"),
        ("planned", planned, "summary.planned is stale"),
        (
            "with_entries",
            with_entries,
            "summary.with_entries is stale",
        ),
        ("broken", broken, "summary.broken is stale"),
        (
            "validation_profile_count",
            profile_count,
            "summary.validation_profile_count is stale",
        ),
        (
            "release_candidate_profiles",
            release_profiles,
            "summary.release_candidate_profiles is stale",
        ),
        (
            "component_profiles",
            profile_count.saturating_sub(release_profiles),
            "summary.component_profiles is stale",
        ),
        (
            "candidates_missing_release_profile",
            missing_release_profiles,
            "summary.candidates_missing_release_profile is stale",
        ),
    ] {
        if report
            .pointer(&format!("/summary/{key}"))
            .and_then(Value::as_u64)
            != Some(expected)
        {
            errors.push(format!("{relative_input}: {label}"));
        }
    }
    check_count_map(
        report,
        "/summary/target_levels",
        &count_by(candidates, "target_level", TARGET_LEVELS),
        relative_input,
        "summary.target_levels",
        errors,
    );
    check_count_map(
        report,
        "/summary/evidence_phases",
        &count_by(candidates, "evidence_phase", EVIDENCE_PHASES),
        relative_input,
        "summary.evidence_phases",
        errors,
    );
    check_count_map(
        report,
        "/summary/release_gate_impacts",
        &count_by(candidates, "release_gate_impact", RELEASE_GATE_IMPACTS),
        relative_input,
        "summary.release_gate_impacts",
        errors,
    );
    check_count_map(
        report,
        "/summary/release_review_statuses",
        &count_release_review_statuses(candidates),
        relative_input,
        "summary.release_review_statuses",
        errors,
    );
}

fn action_errors(action: &Value, index: usize) -> Vec<String> {
    let mut errors = Vec::new();
    let context = format!("next_actions[{index}]");
    for field_name in [
        "candidate_id",
        "priority",
        "target_level",
        "evidence_phase",
        "readiness",
        "action_kind",
    ] {
        require_string(action.get(field_name), field_name, &context, &mut errors);
    }
    let action_kind = field(action, "action_kind");
    if !ALLOWED_ACTION_KINDS.contains(&action_kind) {
        errors.push(format!("{context}: unsupported action_kind {action_kind}"));
    }
    if action_kind == "run_command" {
        require_string(action.get("command"), "command", &context, &mut errors);
    }
    if action_kind == "restore_or_generate_artifact" {
        require_string(action.get("path"), "path", &context, &mut errors);
    }
    if action_kind == "review" {
        require_string(action.get("review_reason"), "review_reason", &context, &mut errors);
    }
    require_u64(
        action.get("validation_profile_count"),
        "validation_profile_count",
        &context,
        &mut errors,
    );
    require_u64(
        action.get("release_candidate_profile_count"),
        "release_candidate_profile_count",
        &context,
        &mut errors,
    );
    require_string(action.get("gate"), "gate", &context, &mut errors);
    require_string(
        action.get("preferred_validation_lane"),
        "preferred_validation_lane",
        &context,
        &mut errors,
    );
    require_string(
        action.get("release_gate_impact"),
        "release_gate_impact",
        &context,
        &mut errors,
    );
    errors
}

fn count_by<T>(candidates: &[T], field_name: &str, values: &[&str]) -> Value
where
    T: std::borrow::Borrow<Value>,
{
    let mut map = serde_json::Map::new();
    for value in values {
        let count = candidates
            .iter()
            .filter(|candidate| field(candidate.borrow(), field_name) == *value)
            .count();
        map.insert((*value).to_string(), Value::from(count));
    }
    Value::Object(map)
}

fn count_release_review_statuses<T>(candidates: &[T]) -> Value
where
    T: std::borrow::Borrow<Value>,
{
    let mut map = serde_json::Map::new();
    for status in RELEASE_REVIEW_STATUSES {
        let count = candidates
            .iter()
            .flat_map(|candidate| array(candidate.borrow(), "artifacts"))
            .filter(|artifact| field(artifact, "kind") == "release_retained_regression_output")
            .filter(|artifact| {
                let review_status = field(artifact, "release_review_status");
                (review_status.is_empty() && *status == "missing") || review_status == *status
            })
            .count();
        map.insert((*status).to_string(), Value::from(count));
    }
    Value::Object(map)
}

fn check_count_map(
    report: &Value,
    pointer: &str,
    expected: &Value,
    relative_input: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    let Some(actual) = report.pointer(pointer).and_then(Value::as_object) else {
        errors.push(format!("{relative_input}: {label} must be an object"));
        return;
    };
    let Some(expected) = expected.as_object() else {
        errors.push(format!(
            "{relative_input}: {label} expected count map is invalid"
        ));
        return;
    };
    for (key, expected_value) in expected {
        if actual.get(key).and_then(Value::as_u64) != expected_value.as_u64() {
            errors.push(format!("{relative_input}: {label}.{key} is stale"));
        }
    }
}

pub(super) fn compare_actions(left: &Value, right: &Value) -> Ordering {
    priority_rank(field(left, "priority"))
        .cmp(&priority_rank(field(right, "priority")))
        .then(
            readiness_rank(field(left, "readiness"))
                .cmp(&readiness_rank(field(right, "readiness"))),
        )
        .then(field(left, "candidate_id").cmp(field(right, "candidate_id")))
}

fn first_actionable_artifact(candidate: &Value) -> Option<&Value> {
    array(candidate, "artifacts")
        .into_iter()
        .find(|artifact| field(artifact, "state") != "present")
}

fn action_kind_for_artifact(artifact: Option<&Value>) -> &'static str {
    match artifact.map(|artifact| field(artifact, "state")) {
        None => "review",
        Some("command_available") => "run_command",
        Some("missing") => "restore_or_generate_artifact",
        _ => "collect_artifact",
    }
}

fn count_state(artifacts: &[Value], state: &str) -> usize {
    artifacts
        .iter()
        .filter(|artifact| field(artifact, "state") == state)
        .count()
}

#[rustfmt::skip]
fn priority_rank(priority: &str) -> u8 { match priority { "p0" => 0, "p1" => 1, "p2" => 2, "p3" => 3, _ => 99 } }

#[rustfmt::skip]
fn readiness_rank(readiness: &str) -> u8 { match readiness { "broken" => 0, "planned" => 1, "partially_collecting" => 2, "collecting_with_entries" => 3, "ready_for_review" => 4, "blocked" => 5, _ => 99 } }

fn require_string(
    value: Option<&Value>,
    field_name: &str,
    context: &str,
    errors: &mut Vec<String>,
) {
    if value.and_then(Value::as_str).is_none_or(str::is_empty) {
        errors.push(format!(
            "{context}: {field_name} must be a non-empty string"
        ));
    }
}

fn require_u64(value: Option<&Value>, field_name: &str, context: &str, errors: &mut Vec<String>) {
    if value.and_then(Value::as_u64).is_none() {
        errors.push(format!(
            "{context}: {field_name} must be a non-negative integer"
        ));
    }
}

fn repo_local_path(root: &Path, path: &str, label: &str) -> RunnerResult<(PathBuf, String)> {
    let absolute = root.join(path);
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("{label} must stay inside the repository"))?
        .to_string_lossy()
        .to_string();
    if relative.starts_with("..") || Path::new(&relative).is_absolute() {
        return Err(format!("{label} must stay inside the repository"));
    }
    Ok((absolute, relative))
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    read_json_path(&root.join(relative_path), relative_path)
}

fn read_json_path(path: &Path, label: &str) -> RunnerResult<Value> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {label}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label}: invalid json: {error}"))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode readiness report: {error}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(super) fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[rustfmt::skip]
fn optional_field(value: &Value, key: &str) -> Value { let text = field(value, key); if text.is_empty() { Value::Null } else { Value::from(text) } }
