use crate::{RunnerResult, native_time::utc_iso_timestamp};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod self_test;

const DEFAULT_INPUT: &str = "tmp/operator-qualification-readiness.json";
const DEFAULT_OUT: &str = "tmp/operator-qualification-readiness.json";
const SCHEMA_PATH: &str = "schemas/operator-qualification-readiness.schema.json";
const ROADMAP_PATH: &str = "config/operator-qualification-roadmap.json";
const EVIDENCE_KITS_PATH: &str = "config/operator-qualification-evidence-kits.json";
const SCHEMA_VERSION: &str = "kyuubiki.operator-qualification-readiness/v1";
const ALLOWED_ACTION_KINDS: &[&str] = &[
    "collect_artifact",
    "restore_or_generate_artifact",
    "run_command",
    "review",
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

fn parse_out(args: Vec<OsString>) -> RunnerResult<String> {
    let mut out = DEFAULT_OUT.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--out" => {
                let Some(value) = iter.next() else {
                    return Err("--out requires a repo-local path".to_string());
                };
                out = value.to_string_lossy().to_string();
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if out.is_empty() {
        return Err("--out requires a repo-local path".to_string());
    }
    Ok(out)
}

struct CheckOptions {
    input: String,
    self_test: bool,
}

fn parse_check_args(args: Vec<OsString>) -> RunnerResult<CheckOptions> {
    let mut input = DEFAULT_INPUT.to_string();
    let mut self_test = false;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--self-test" => self_test = true,
            "--in" => {
                let Some(value) = iter.next() else {
                    return Err("--in requires a repo-local path".to_string());
                };
                input = value.to_string_lossy().to_string();
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !self_test && input.is_empty() {
        return Err("--in requires a repo-local path".to_string());
    }
    Ok(CheckOptions { input, self_test })
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
        .collect::<std::collections::HashMap<_, _>>();
    let candidates = array(&roadmap, "candidates")
        .into_iter()
        .map(|candidate| {
            readiness_for(
                root,
                candidate,
                kit_by_candidate
                    .get(field(candidate, "candidate_id"))
                    .copied(),
            )
        })
        .collect::<RunnerResult<Vec<_>>>()?;
    let next_actions = build_next_actions(&candidates);
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
            "next_action_count": next_actions.len(),
        },
        "next_actions": next_actions,
        "candidates": candidates,
    }))
}

fn readiness_for(root: &Path, candidate: &Value, kit: Option<&Value>) -> RunnerResult<Value> {
    let artifacts = kit
        .map(|kit| array(kit, "artifact_requirements"))
        .unwrap_or_default()
        .into_iter()
        .map(|requirement| artifact_state(root, requirement))
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
        "collecting_with_entries"
    } else if actionable > 0 {
        "partially_collecting"
    } else {
        "planned"
    };
    Ok(json!({
        "candidate_id": field(candidate, "candidate_id"),
        "priority": field(candidate, "priority"),
        "domain": field(candidate, "domain"),
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
        "evidence_gaps": candidate.get("evidence_gaps").cloned().unwrap_or(Value::Array(Vec::new())),
        "graduation_gate": field(candidate, "graduation_gate"),
    }))
}

fn artifact_state(root: &Path, requirement: &Value) -> RunnerResult<Value> {
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
    let command = field(requirement, "artifact_command");
    if !command.is_empty() {
        return Ok(json!({
            "artifact_id": field(requirement, "artifact_id"),
            "kind": field(requirement, "kind"),
            "state": "command_available",
            "command": command,
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
            json!({
                "candidate_id": field(candidate, "candidate_id"),
                "priority": field(candidate, "priority"),
                "readiness": field(candidate, "readiness"),
                "action_kind": action_kind_for_artifact(artifact),
                "artifact_id": artifact.and_then(|artifact| artifact.get("artifact_id")).cloned().unwrap_or(Value::Null),
                "artifact_state": artifact.and_then(|artifact| artifact.get("state")).cloned().unwrap_or(Value::Null),
                "artifact_kind": artifact.and_then(|artifact| artifact.get("kind")).cloned().unwrap_or(Value::Null),
                "command": artifact.and_then(|artifact| artifact.get("command")).cloned().unwrap_or(Value::Null),
                "path": artifact.and_then(|artifact| artifact.get("path")).cloned().unwrap_or(Value::Null),
                "gate": artifact.and_then(|artifact| artifact.get("gate")).and_then(Value::as_str).unwrap_or_else(|| field(candidate, "graduation_gate")),
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
    for (key, expected, label) in [
        ("collecting", collecting, "summary.collecting is stale"),
        ("planned", planned, "summary.planned is stale"),
        (
            "with_entries",
            with_entries,
            "summary.with_entries is stale",
        ),
        ("broken", broken, "summary.broken is stale"),
    ] {
        if report
            .pointer(&format!("/summary/{key}"))
            .and_then(Value::as_u64)
            != Some(expected)
        {
            errors.push(format!("{relative_input}: {label}"));
        }
    }
}

fn action_errors(action: &Value, index: usize) -> Vec<String> {
    let mut errors = Vec::new();
    let context = format!("next_actions[{index}]");
    for field_name in ["candidate_id", "priority", "readiness", "action_kind"] {
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
    require_string(action.get("gate"), "gate", &context, &mut errors);
    errors
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

fn priority_rank(priority: &str) -> u8 {
    match priority {
        "p0" => 0,
        "p1" => 1,
        "p2" => 2,
        "p3" => 3,
        _ => 99,
    }
}

fn readiness_rank(readiness: &str) -> u8 {
    match readiness {
        "broken" => 0,
        "planned" => 1,
        "partially_collecting" => 2,
        "collecting_with_entries" => 3,
        "blocked" => 4,
        _ => 99,
    }
}

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

#[cfg(test)]
mod tests {
    use super::{compare_actions, priority_rank, readiness_rank};

    #[test]
    fn sort_ranks_match_contract_order() {
        assert!(priority_rank("p0") < priority_rank("p1"));
        assert!(readiness_rank("broken") < readiness_rank("planned"));
    }

    #[test]
    fn compare_actions_orders_priority_first() {
        let left =
            serde_json::json!({"priority": "p0", "readiness": "blocked", "candidate_id": "z"});
        let right =
            serde_json::json!({"priority": "p1", "readiness": "broken", "candidate_id": "a"});
        assert!(compare_actions(&left, &right).is_lt());
    }
}
