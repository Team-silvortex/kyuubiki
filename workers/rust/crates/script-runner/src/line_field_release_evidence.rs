use crate::{line_field_provenance, native_time::utc_iso_timestamp};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const DEFAULT_INPUT: &str = "tmp/line-field-qualification-release-evidence.json";
const DEFAULT_OUT: &str = "tmp/line-field-qualification-release-evidence.json";
const REQUIRED_COMMAND_IDS: &[&str] = &["evidence_check", "solver_baseline"];
const REQUIRED_PROMOTED_OPERATOR_IDS: &[&str] = &[
    "solve.bar_1d",
    "solve.thermal_bar_1d",
    "solve.heat_bar_1d",
    "solve.electrostatic_bar_1d",
];
const RETAINED_EVIDENCE_PATH: &str =
    "releases/qualification-evidence/2.0.0/line-field-closed-form-release-evidence.json";
const RELEASE_RECORD_PATH: &str = "releases/qualification-records/1.20.0.json";
const REVIEW_DECISION_PATH: &str =
    "releases/qualification-review-decisions/2.0.0/line-field-closed-form-review-decision.json";
const REQUIRED_TRACKED_INPUTS: &[&str] = &[
    "evidence/operator-qualification/line-field-closed-form-baseline.json",
    "evidence/operator-qualification/line-field-closed-form-derivation.md",
    "evidence/operator-qualification/line-field-tolerance-policy.json",
    "workers/rust/crates/solver/tests/accuracy_baselines/line_1d.rs",
    "scripts/check-line-field-closed-form-baseline.mjs",
];

type RunnerResult<T> = Result<T, String>;

struct CaptureOptions {
    out: String,
    allow_failure: bool,
}

struct EvidenceCommand {
    id: &'static str,
    cwd: &'static str,
    command: &'static str,
    args: &'static [&'static str],
}

const EVIDENCE_COMMANDS: &[EvidenceCommand] = &[
    EvidenceCommand {
        id: "evidence_check",
        cwd: ".",
        command: "./scripts/kyuubiki",
        args: &["check-line-field-closed-form-baseline"],
    },
    EvidenceCommand {
        id: "solver_baseline",
        cwd: "workers/rust",
        command: "cargo",
        args: &[
            "test",
            "-p",
            "kyuubiki-solver",
            "--test",
            "accuracy_baselines",
            "line_1d",
        ],
    },
];

pub(crate) fn run_capture_line_field_qualification_release_evidence(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_capture_args(args)?;
    let (absolute, relative) = repo_local_path(root, &options.out, "--out")?;
    let evidence = build_release_evidence(root)?;
    write_json(&absolute, &evidence)?;
    println!("line-field qualification release evidence wrote {relative}");
    let ok = evidence.pointer("/summary/ok").and_then(Value::as_bool) == Some(true);
    Ok(if ok || options.allow_failure { 0 } else { 1 })
}

pub(crate) fn run_check_line_field_qualification_release_evidence(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let input = parse_input(args)?;
    let (absolute, relative) = repo_local_path(root, &input, "--in")?;
    if !absolute.exists() {
        eprintln!(
            "line-field qualification release evidence check failed: input does not exist: {relative}"
        );
        return Ok(1);
    }
    let evidence = read_json_path(&absolute, &relative)?;
    if let Err(issue) = validate_evidence(root, &evidence) {
        eprintln!("line-field qualification release evidence check failed: {issue}");
        return Ok(1);
    }
    println!("line-field qualification release evidence ok: {input}");
    Ok(0)
}

fn parse_capture_args(args: Vec<OsString>) -> RunnerResult<CaptureOptions> {
    let mut options = CaptureOptions {
        out: DEFAULT_OUT.to_string(),
        allow_failure: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner capture-line-field-qualification-release-evidence [--out tmp/file.json] [--allow-failure]"
                );
                return Ok(options);
            }
            "--out" => {
                let Some(value) = iter.next() else {
                    return Err("--out requires a repo-local path".to_string());
                };
                options.out = value.to_string_lossy().to_string();
            }
            "--allow-failure" => options.allow_failure = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if options.out.is_empty() {
        return Err("--out requires a repo-local path".to_string());
    }
    Ok(options)
}

fn parse_input(args: Vec<OsString>) -> RunnerResult<String> {
    let mut input = DEFAULT_INPUT.to_string();
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner check-line-field-qualification-release-evidence [--in tmp/file.json]"
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
    if input.is_empty() {
        return Err("--in requires a repo-local path".to_string());
    }
    Ok(input)
}

fn build_release_evidence(root: &Path) -> RunnerResult<Value> {
    let commands = EVIDENCE_COMMANDS
        .iter()
        .map(|command| run_evidence_command(root, command))
        .collect::<Vec<_>>();
    let passed = commands
        .iter()
        .filter(|command| command.get("ok").and_then(Value::as_bool) == Some(true))
        .count();
    let failed = commands.len().saturating_sub(passed);
    Ok(json!({
        "schema_version": "kyuubiki.operator-qualification-release-evidence/v1",
        "version_line": "moxi 2.0.x",
        "candidate_id": "line-field-closed-form",
        "generated_at_utc": utc_iso_timestamp(),
        "release_retention": {
            "intended_release_artifact": true,
            "repo_relative_paths_only": true,
            "generated_output_should_not_be_committed_directly": true,
        },
        "promotion_summary": {
            "candidate_id": "line-field-closed-form",
            "release_version": "2.0.0",
            "approved_coverage_level": "qualification",
            "retained_evidence_path": RETAINED_EVIDENCE_PATH,
            "release_record_path": RELEASE_RECORD_PATH,
            "review_decision_path": REVIEW_DECISION_PATH,
            "promoted_operator_ids": REQUIRED_PROMOTED_OPERATOR_IDS,
        },
        "provenance": line_field_provenance::build_provenance(root)?,
        "commands": commands,
        "summary": {
            "command_count": commands.len(),
            "passed": passed,
            "failed": failed,
            "ok": failed == 0,
        },
    }))
}

fn run_evidence_command(root: &Path, spec: &EvidenceCommand) -> Value {
    let started = Instant::now();
    let output = Command::new(spec.command)
        .args(spec.args)
        .current_dir(root.join(spec.cwd))
        .output();
    match output {
        Ok(output) => json!({
            "id": spec.id,
            "cwd": spec.cwd,
            "argv": std::iter::once(spec.command).chain(spec.args.iter().copied()).collect::<Vec<_>>(),
            "status": output.status.code().unwrap_or(1),
            "signal": Value::Null,
            "duration_ms": started.elapsed().as_millis(),
            "stdout": sanitize_output(root, &String::from_utf8_lossy(&output.stdout)),
            "stderr": sanitize_output(root, &String::from_utf8_lossy(&output.stderr)),
            "ok": output.status.success(),
        }),
        Err(error) => json!({
            "id": spec.id,
            "cwd": spec.cwd,
            "argv": std::iter::once(spec.command).chain(spec.args.iter().copied()).collect::<Vec<_>>(),
            "status": 1,
            "signal": Value::Null,
            "duration_ms": started.elapsed().as_millis(),
            "stdout": "",
            "stderr": sanitize_output(root, &error.to_string()),
            "ok": false,
        }),
    }
}

fn sanitize_output(root: &Path, text: &str) -> String {
    let mut sanitized = text.replace(root.to_string_lossy().as_ref(), "$REPO_ROOT");
    if let Ok(canonical) = root.canonicalize() {
        sanitized = sanitized.replace(canonical.to_string_lossy().as_ref(), "$REPO_ROOT");
    }
    sanitized
}

fn validate_evidence(root: &Path, evidence: &Value) -> RunnerResult<()> {
    require_eq(
        field(evidence, "schema_version"),
        "kyuubiki.operator-qualification-release-evidence/v1",
        "unexpected schema_version",
    )?;
    require_eq(
        field(evidence, "version_line"),
        "moxi 2.0.x",
        "version_line must match moxi 2.0.x",
    )?;
    require_eq(
        field(evidence, "candidate_id"),
        "line-field-closed-form",
        "candidate_id must be line-field-closed-form",
    )?;
    require_true(
        evidence.pointer("/release_retention/intended_release_artifact"),
        "release_retention.intended_release_artifact must be true",
    )?;
    require_true(
        evidence.pointer("/release_retention/repo_relative_paths_only"),
        "release_retention.repo_relative_paths_only must be true",
    )?;
    require_true(
        evidence.pointer("/release_retention/generated_output_should_not_be_committed_directly"),
        "release_retention.generated_output_should_not_be_committed_directly must be true",
    )?;
    if evidence.pointer("/summary/ok").and_then(Value::as_bool) != Some(true)
        || evidence.pointer("/summary/failed").and_then(Value::as_u64) != Some(0)
    {
        return Err("summary must report a passing release evidence run".to_string());
    }
    let commands = array(evidence, "commands");
    if commands.len() != REQUIRED_COMMAND_IDS.len() {
        return Err(format!(
            "commands must contain exactly {} entries",
            REQUIRED_COMMAND_IDS.len()
        ));
    }
    let mut seen_commands = BTreeSet::new();
    for command in commands {
        validate_command(command)?;
        seen_commands.insert(field(command, "id").to_string());
    }
    for expected in REQUIRED_COMMAND_IDS {
        if !seen_commands.contains(*expected) {
            return Err(format!("missing command {expected}"));
        }
    }
    validate_promotion_summary(root, evidence.get("promotion_summary").unwrap_or(&Value::Null))?;
    validate_provenance(evidence.get("provenance").unwrap_or(&Value::Null))?;
    assert_no_absolute_repo_path(root, evidence, "evidence")
}

fn validate_promotion_summary(root: &Path, summary: &Value) -> RunnerResult<()> {
    if !summary.is_object() {
        return Err("promotion_summary: must be present".to_string());
    }
    require_eq(
        field(summary, "candidate_id"),
        "line-field-closed-form",
        "promotion_summary: candidate_id must be line-field-closed-form",
    )?;
    require_eq(
        field(summary, "release_version"),
        "2.0.0",
        "promotion_summary: release_version must be 2.0.0",
    )?;
    require_eq(
        field(summary, "approved_coverage_level"),
        "qualification",
        "promotion_summary: approved_coverage_level must be qualification",
    )?;
    require_eq(
        field(summary, "retained_evidence_path"),
        RETAINED_EVIDENCE_PATH,
        "promotion_summary: retained_evidence_path must point to the retained moxi 2.0.0 evidence",
    )?;
    require_eq(
        field(summary, "release_record_path"),
        RELEASE_RECORD_PATH,
        "promotion_summary: release_record_path mismatch",
    )?;
    require_eq(
        field(summary, "review_decision_path"),
        REVIEW_DECISION_PATH,
        "promotion_summary: review_decision_path mismatch",
    )?;

    let promoted = array(summary, "promoted_operator_ids");
    if promoted.len() != REQUIRED_PROMOTED_OPERATOR_IDS.len() {
        return Err(format!(
            "promotion_summary: expected {} promoted operators",
            REQUIRED_PROMOTED_OPERATOR_IDS.len()
        ));
    }
    let promoted_ids = promoted
        .iter()
        .map(|operator| operator.as_str().unwrap_or_default())
        .collect::<BTreeSet<_>>();
    for expected in REQUIRED_PROMOTED_OPERATOR_IDS {
        if !promoted_ids.contains(expected) {
            return Err(format!(
                "promotion_summary: missing promoted operator {expected}"
            ));
        }
    }

    let release_records = read_repo_json(root, field(summary, "release_record_path"))?;
    let release_record = release_records
        .get("records")
        .and_then(Value::as_array)
        .and_then(|records| {
            records
                .iter()
                .find(|record| field(record, "candidate_id") == field(summary, "candidate_id"))
        })
        .ok_or_else(|| {
            "promotion_summary: release record is missing line-field-closed-form".to_string()
        })?;
    require_eq(
        field(release_record, "review_status"),
        "approved",
        "promotion_summary: release record review_status must be approved",
    )?;
    require_eq(
        field(release_record, "evidence_path"),
        field(summary, "retained_evidence_path"),
        "promotion_summary: release record evidence_path must match retained evidence",
    )?;
    require_eq(
        field(release_record, "review_decision_path"),
        field(summary, "review_decision_path"),
        "promotion_summary: release record review_decision_path mismatch",
    )?;

    let review_decision = read_repo_json(root, field(summary, "review_decision_path"))?;
    require_eq(
        field(&review_decision, "candidate_id"),
        field(summary, "candidate_id"),
        "promotion_summary: review decision candidate_id mismatch",
    )?;
    require_eq(
        field(&review_decision, "release_version"),
        field(summary, "release_version"),
        "promotion_summary: review decision release_version mismatch",
    )?;
    require_eq(
        field(&review_decision, "decision"),
        "approve_promotion",
        "promotion_summary: review decision must approve promotion",
    )?;
    require_eq(
        field(&review_decision, "evidence_path"),
        field(summary, "retained_evidence_path"),
        "promotion_summary: review decision evidence_path must match retained evidence",
    )
}

fn validate_command(command: &Value) -> RunnerResult<()> {
    let context = if field(command, "id").is_empty() {
        "unknown command"
    } else {
        field(command, "id")
    };
    if !REQUIRED_COMMAND_IDS.contains(&context) {
        return Err(format!("{context}: unexpected command id"));
    }
    if command.get("ok").and_then(Value::as_bool) != Some(true)
        || command.get("status").and_then(Value::as_i64) != Some(0)
    {
        return Err(format!("{context}: command must pass with status 0"));
    }
    if array(command, "argv").is_empty() {
        return Err(format!("{context}: argv must be non-empty"));
    }
    let cwd = field(command, "cwd");
    if cwd.is_empty() {
        return Err(format!("{context}: cwd must be repo-relative"));
    }
    if Path::new(cwd).is_absolute() || cwd.contains("..") {
        return Err(format!("{context}: cwd must not escape the repository"));
    }
    let duration = command.get("duration_ms").and_then(Value::as_f64);
    if !duration.is_some_and(|number| number.is_finite() && number >= 0.0) {
        return Err(format!(
            "{context}: duration_ms must be finite and non-negative"
        ));
    }
    Ok(())
}

fn validate_provenance(provenance: &Value) -> RunnerResult<()> {
    require_eq(
        field(provenance, "schema_version"),
        "kyuubiki.operator-qualification-provenance/v1",
        "provenance: unexpected schema_version",
    )?;
    require_eq(
        field(provenance, "candidate_id"),
        "line-field-closed-form",
        "provenance: candidate_id must be line-field-closed-form",
    )?;
    require_true(
        provenance.pointer("/retention_policy/no_local_absolute_paths"),
        "provenance: no_local_absolute_paths must be true",
    )?;
    let tracked = array(provenance, "tracked_inputs");
    if tracked.len() != REQUIRED_TRACKED_INPUTS.len() {
        return Err(format!(
            "provenance: expected {} tracked inputs",
            REQUIRED_TRACKED_INPUTS.len()
        ));
    }
    let mut seen = BTreeSet::new();
    for input in tracked {
        let path = field(input, "path");
        if !REQUIRED_TRACKED_INPUTS.contains(&path) {
            return Err(format!("provenance: unexpected tracked input {path}"));
        }
        let sha = field(input, "sha256");
        if !is_lower_hex_sha256(sha) {
            return Err(format!("provenance: {path} sha256 must be lowercase hex"));
        }
        seen.insert(path.to_string());
    }
    for expected in REQUIRED_TRACKED_INPUTS {
        if !seen.contains(*expected) {
            return Err(format!("provenance: missing tracked input {expected}"));
        }
    }
    Ok(())
}

fn assert_no_absolute_repo_path(root: &Path, value: &Value, context: &str) -> RunnerResult<()> {
    if let Some(text) = value.as_str() {
        let root_text = root.to_string_lossy();
        if text.contains(root_text.as_ref()) {
            return Err(format!(
                "{context}: contains local absolute repository path"
            ));
        }
    }
    if let Some(items) = value.as_array() {
        for (index, item) in items.iter().enumerate() {
            assert_no_absolute_repo_path(root, item, &format!("{context}[{index}]"))?;
        }
    }
    if let Some(object) = value.as_object() {
        for (key, nested) in object {
            assert_no_absolute_repo_path(root, nested, &format!("{context}.{key}"))?;
        }
    }
    Ok(())
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

fn require_true(value: Option<&Value>, message: &str) -> RunnerResult<()> {
    if value.and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn require_eq(actual: &str, expected: &str, message: &str) -> RunnerResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(message.to_string())
    }
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

fn read_json_path(path: &Path, label: &str) -> RunnerResult<Value> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {label}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label}: invalid json: {error}"))
}

fn read_repo_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let (absolute, normalized) = repo_local_path(root, relative_path, "promotion_summary path")?;
    if !absolute.exists() {
        return Err(format!("promotion_summary: missing {normalized}"));
    }
    read_json_path(&absolute, &normalized)
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode release evidence: {error}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
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
    use super::is_lower_hex_sha256;

    #[test]
    fn sha_shape_requires_lowercase_hex() {
        assert!(is_lower_hex_sha256(&"a".repeat(64)));
        assert!(!is_lower_hex_sha256(&"A".repeat(64)));
        assert!(!is_lower_hex_sha256("abc"));
    }
}
