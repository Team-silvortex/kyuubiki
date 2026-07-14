use crate::RunnerResult;
use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_INPUT: &str = "tmp/beam-frame-classic-qualification-release-evidence.json";
const REPORT_SCHEMA_VERSION: &str = "kyuubiki.operator-validation-report/v1";
const PROFILE_ID: &str = "beam-frame-classic";
const SOURCE: &str = "config/operator-validation-profiles.json";
const EXPECTED_OPERATORS: &[&str] = &["solve.beam_1d", "solve.torsion_1d", "solve.frame_2d"];
const EXPECTED_COMMANDS: &[&str] = &[
    "beam-frame-classic-regression",
    "beam-review-fixture",
    "torsion-review-fixture",
    "frame-review-fixture",
];
const EXPECTED_EVIDENCE_PATHS: &[&str] = &[
    "evidence/operator-qualification/beam-frame-classic-reference-note.md",
    "evidence/operator-qualification/beam-frame-force-sign-convention.md",
    "workers/rust/crates/solver/tests/beam_frame_classic_regression.rs",
    "workers/rust/crates/solver/tests/beam_1d_review.rs",
    "workers/rust/crates/solver/tests/torsion_1d_review.rs",
    "workers/rust/crates/solver/tests/frame_2d_review.rs",
];

pub(crate) fn run_check_beam_frame_qualification_release_evidence(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let input = parse_args(args)?;
    let (absolute, relative) = repo_local_path(root, &input, "--in")?;
    if !absolute.exists() {
        eprintln!(
            "beam-frame qualification release evidence check failed: input does not exist: {relative}"
        );
        return Ok(1);
    }
    let report = read_json(&absolute, &relative)?;
    match validate_report(root, &report) {
        Ok(()) => {
            println!("beam-frame qualification release evidence ok: {relative}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("beam-frame qualification release evidence check failed: {issue}");
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
                    "usage: kyuubiki-script-runner check-beam-frame-qualification-release-evidence [--in tmp/file.json]"
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

fn validate_report(root: &Path, report: &Value) -> RunnerResult<()> {
    assert_eq(
        field(report, "schema_version"),
        REPORT_SCHEMA_VERSION,
        "schema_version",
    )?;
    assert_eq(field(report, "source"), SOURCE, "source")?;
    assert_bool(report.get("executed"), true, "executed")?;
    assert_bool(report.get("ok"), true, "ok")?;
    assert_number(report.get("profile_count"), 1, "profile_count")?;
    let profiles = array(report, "profiles")?;
    if profiles.len() != 1 {
        return Err("profiles must contain only beam-frame-classic".to_string());
    }
    validate_profile(root, &profiles[0])
}

fn validate_profile(root: &Path, profile: &Value) -> RunnerResult<()> {
    assert_eq(field(profile, "profile_id"), PROFILE_ID, "profile_id")?;
    assert_eq(
        field(profile, "trust_goal"),
        "qualification_input",
        "trust_goal",
    )?;
    assert_bool(profile.get("ok"), true, "profile.ok")?;
    assert_string_set(profile.get("operators"), EXPECTED_OPERATORS, "operators")?;
    assert_string_set(
        profile.get("evidence_paths"),
        EXPECTED_EVIDENCE_PATHS,
        "evidence_paths",
    )?;
    for path in EXPECTED_EVIDENCE_PATHS {
        if !root.join(path).exists() {
            return Err(format!("evidence path missing from repository: {path}"));
        }
    }
    let commands = array(profile, "commands")?;
    let command_ids = commands
        .iter()
        .map(|command| field(command, "id").to_string())
        .collect::<BTreeSet<_>>();
    if command_ids != expected_set(EXPECTED_COMMANDS) {
        return Err(format!("commands mismatch: {command_ids:?}"));
    }
    for command in commands {
        validate_command(command)?;
    }
    Ok(())
}

fn validate_command(command: &Value) -> RunnerResult<()> {
    let id = field(command, "id");
    if id.is_empty() {
        return Err("command.id must be non-empty".to_string());
    }
    let text = field(command, "command");
    if !text.starts_with("cd workers/rust && cargo test -p kyuubiki-solver --test ") {
        return Err(format!("command {id} must use the solver cargo-test lane"));
    }
    assert_bool(
        command.pointer("/result/ok"),
        true,
        &format!("command {id} ok"),
    )?;
    assert_number(
        command.pointer("/result/status"),
        0,
        &format!("command {id} status"),
    )?;
    if command
        .pointer("/result/duration_ms")
        .and_then(Value::as_i64)
        .is_none()
    {
        return Err(format!("command {id} duration_ms must be present"));
    }
    Ok(())
}

fn array<'a>(value: &'a Value, key: &str) -> RunnerResult<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))
}

fn assert_eq(actual: &str, expected: &str, context: &str) -> RunnerResult<()> {
    if actual != expected {
        return Err(format!("{context} must be {expected}, got {actual}"));
    }
    Ok(())
}

fn assert_bool(value: Option<&Value>, expected: bool, context: &str) -> RunnerResult<()> {
    if value.and_then(Value::as_bool) != Some(expected) {
        return Err(format!("{context} must be {expected}"));
    }
    Ok(())
}

fn assert_number(value: Option<&Value>, expected: i64, context: &str) -> RunnerResult<()> {
    if value.and_then(Value::as_i64) != Some(expected) {
        return Err(format!("{context} must be {expected}"));
    }
    Ok(())
}

fn assert_string_set(value: Option<&Value>, expected: &[&str], context: &str) -> RunnerResult<()> {
    let actual = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context} must be an array"))?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if actual != expected_set(expected) {
        return Err(format!("{context} mismatch: {actual:?}"));
    }
    Ok(())
}

fn expected_set(expected: &[&str]) -> BTreeSet<String> {
    expected.iter().map(|value| value.to_string()).collect()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn read_json(path: &Path, label: &str) -> RunnerResult<Value> {
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
