use crate::{RunnerResult, run_command};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const DEFAULT_CONFIG: &str = "config/operator-validation-profiles.json";
const DEFAULT_OUT: &str = "tmp/operator-validation-report.json";
const SCHEMA_VERSION: &str = "kyuubiki.operator-validation-profiles/v1";
const REPORT_SCHEMA_VERSION: &str = "kyuubiki.operator-validation-report/v1";
const PROFILE_SCHEMA: &str = "schemas/operator-validation-profiles.schema.json";
const REPORT_SCHEMA: &str = "schemas/operator-validation-report.schema.json";
const ALLOWED_COMMAND_PREFIXES: &[&str] = &["make ", "cd workers/rust && cargo "];
const ALLOWED_KINDS: &[&str] = &[
    "analytic",
    "boundary_regression",
    "contract",
    "cross_check",
    "invariant",
];
const ALLOWED_PROFILE_ROLES: &[&str] = &["release_candidate", "component_profile"];

pub(crate) fn run_check_operator_validation(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    match run(root, args) {
        Ok(code) => Ok(code),
        Err(message) => {
            eprintln!("operator validation failed: {message}");
            Ok(1)
        }
    }
}

fn run(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = parse_args(args)?;
    if options.self_test {
        run_self_test(root)?;
        println!("operator validation self-test passed");
        return Ok(0);
    }
    if let Some(input_report) = &options.input_report {
        let report = read_json(root, input_report)?;
        validate_report(&report, &options)?;
        validate_input_report(&report, &options)?;
        let profile_count = report
            .get("profile_count")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        println!("operator validation report ok: {input_report} ({profile_count} profile(s))");
        return Ok(0);
    }
    let config = load_config(root, &options.config)?;
    validate_config(root, &config)?;
    let report = build_report(root, &config, &options)?;
    validate_report(&report, &options)?;
    write_report(root, &report, &options.out)?;
    let ok = report.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let profile_count = report
        .get("profile_count")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    println!(
        "operator validation {}: {profile_count} profile(s), executed={}",
        if ok { "passed" } else { "failed" },
        options.execute
    );
    Ok(if ok { 0 } else { 1 })
}

#[derive(Debug, Clone)]
struct Options {
    config: String,
    input_report: Option<String>,
    out: String,
    profile: Option<String>,
    execute: bool,
    self_test: bool,
}

fn parse_args(args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        config: DEFAULT_CONFIG.to_string(),
        input_report: None,
        out: DEFAULT_OUT.to_string(),
        profile: None,
        execute: false,
        self_test: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--config" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --config".to_string());
                };
                options.config = value.to_string_lossy().to_string();
            }
            "--in" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --in".to_string());
                };
                options.input_report = Some(value.to_string_lossy().to_string());
            }
            "--out" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --out".to_string());
                };
                options.out = value.to_string_lossy().to_string();
            }
            "--profile" => {
                let Some(value) = iter.next() else {
                    return Err("missing value for --profile".to_string());
                };
                options.profile = Some(value.to_string_lossy().to_string());
            }
            "--execute" => options.execute = true,
            "--self-test" => options.self_test = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(options)
}

fn validate_config(root: &Path, config: &Value) -> RunnerResult<()> {
    if field(config, "schema_version") != SCHEMA_VERSION {
        return Err(format!("schema_version must be {SCHEMA_VERSION}"));
    }
    require_string(config.get("version_line"), "version_line", "config")?;
    if config.get("profile_shards").is_some() {
        require_string_list(config.get("profile_shards"), "profile_shards", "config")?;
    }
    let profiles = config
        .get("profiles")
        .and_then(Value::as_array)
        .ok_or_else(|| "profiles must be non-empty".to_string())?;
    if profiles.is_empty() {
        return Err("profiles must be non-empty".to_string());
    }
    let mut seen = HashSet::new();
    for (index, profile) in profiles.iter().enumerate() {
        validate_profile(root, profile, &format!("profiles/{index}"))?;
        let profile_id = field(profile, "profile_id");
        if !seen.insert(profile_id.to_string()) {
            return Err(format!("duplicate profile_id {profile_id}"));
        }
    }
    Ok(())
}

fn load_config(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let mut config = read_json(root, relative_path)?;
    let version_line = field(&config, "version_line").to_string();
    let mut profiles = config
        .get("profiles")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "profiles must be an array".to_string())?;
    for shard_path in string_array(config.get("profile_shards")) {
        let shard = read_json(root, shard_path)?;
        if field(&shard, "schema_version") != SCHEMA_VERSION {
            return Err(format!(
                "{shard_path}: schema_version must be {SCHEMA_VERSION}"
            ));
        }
        if field(&shard, "version_line") != version_line {
            return Err(format!(
                "{shard_path}: version_line must match {relative_path}"
            ));
        }
        let shard_profiles = shard
            .get("profiles")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{shard_path}: profiles must be an array"))?;
        profiles.extend(shard_profiles.iter().cloned());
    }
    config["profiles"] = Value::Array(profiles);
    Ok(config)
}

fn validate_profile(root: &Path, profile: &Value, context: &str) -> RunnerResult<()> {
    require_string(profile.get("profile_id"), "profile_id", context)?;
    require_string(profile.get("profile_role"), "profile_role", context)?;
    require_string(
        profile.get("qualification_candidate_id"),
        "qualification_candidate_id",
        context,
    )?;
    require_string(profile.get("trust_goal"), "trust_goal", context)?;
    let profile_role = field(profile, "profile_role");
    if !ALLOWED_PROFILE_ROLES.contains(&profile_role) {
        return Err(format!(
            "{context}: unsupported profile_role {profile_role}"
        ));
    }
    for field_name in [
        "operators",
        "validation_methods",
        "formal_invariants",
        "evidence_paths",
    ] {
        require_string_array(profile.get(field_name), field_name, context)?;
    }
    let commands = profile
        .get("commands")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}: commands must be non-empty"))?;
    if commands.is_empty() {
        return Err(format!("{context}: commands must be non-empty"));
    }
    for evidence_path in string_array(profile.get("evidence_paths")) {
        if !repo_path(root, evidence_path)?.exists() {
            return Err(format!(
                "{context}: evidence path does not exist: {evidence_path}"
            ));
        }
    }
    for (index, command) in commands.iter().enumerate() {
        validate_command(command, &format!("{context}#commands/{index}"))?;
    }
    Ok(())
}

fn validate_command(command: &Value, context: &str) -> RunnerResult<()> {
    require_string(command.get("id"), "command.id", context)?;
    require_string(command.get("kind"), "command.kind", context)?;
    require_string(command.get("command"), "command.command", context)?;
    let kind = field(command, "kind");
    if !ALLOWED_KINDS.contains(&kind) {
        return Err(format!("{context}: unsupported command kind {kind}"));
    }
    let command_text = field(command, "command");
    if !ALLOWED_COMMAND_PREFIXES
        .iter()
        .any(|prefix| command_text.starts_with(prefix))
    {
        return Err(format!(
            "{context}: unsupported command prefix {command_text}"
        ));
    }
    if command_text.contains("..") || command_text.contains(';') || command_text.contains("&& rm ")
    {
        return Err(format!(
            "{context}: command contains unsafe shell structure"
        ));
    }
    Ok(())
}

fn build_report(root: &Path, config: &Value, options: &Options) -> RunnerResult<Value> {
    let source_profiles = config
        .get("profiles")
        .and_then(Value::as_array)
        .ok_or_else(|| "profiles must be an array".to_string())?;
    let profiles = source_profiles
        .iter()
        .filter(|profile| {
            options
                .profile
                .as_deref()
                .is_none_or(|wanted| field(profile, "profile_id") == wanted)
        })
        .map(|profile| build_profile_report(root, profile, options))
        .collect::<RunnerResult<Vec<_>>>()?;
    if profiles.is_empty() {
        let wanted = options.profile.as_deref().unwrap_or("<all>");
        return Err(format!("no operator validation profiles matched {wanted}"));
    }
    let ok = profiles
        .iter()
        .all(|profile| profile.get("ok").and_then(Value::as_bool) == Some(true));
    Ok(json!({
        "schema_version": REPORT_SCHEMA_VERSION,
        "source": options.config,
        "executed": options.execute,
        "profile_count": profiles.len(),
        "ok": ok,
        "profiles": profiles,
    }))
}

fn build_profile_report(root: &Path, profile: &Value, options: &Options) -> RunnerResult<Value> {
    let commands = profile
        .get("commands")
        .and_then(Value::as_array)
        .unwrap_or(&Vec::new())
        .iter()
        .map(|command| build_command_report(root, command, options.execute))
        .collect::<RunnerResult<Vec<_>>>()?;
    let ok = commands
        .iter()
        .all(|command| command.pointer("/result/ok").and_then(Value::as_bool) != Some(false));
    Ok(json!({
        "profile_id": field(profile, "profile_id"),
        "profile_role": field(profile, "profile_role"),
        "qualification_candidate_id": field(profile, "qualification_candidate_id"),
        "trust_goal": field(profile, "trust_goal"),
        "operators": profile.get("operators").cloned().unwrap_or(Value::Array(Vec::new())),
        "validation_methods": profile.get("validation_methods").cloned().unwrap_or(Value::Array(Vec::new())),
        "formal_invariants": profile.get("formal_invariants").cloned().unwrap_or(Value::Array(Vec::new())),
        "evidence_paths": profile.get("evidence_paths").cloned().unwrap_or(Value::Array(Vec::new())),
        "commands": commands,
        "ok": ok,
    }))
}

fn build_command_report(root: &Path, command: &Value, execute: bool) -> RunnerResult<Value> {
    let result = if execute {
        run_validation_command(root, field(command, "command"))?
    } else {
        json!({ "ok": null, "status": "not_run" })
    };
    Ok(json!({
        "id": field(command, "id"),
        "kind": field(command, "kind"),
        "command": field(command, "command"),
        "result": result,
    }))
}

fn run_validation_command(root: &Path, command: &str) -> RunnerResult<Value> {
    let started = Instant::now();
    let status = if let Some(rest) = command.strip_prefix("make ") {
        run_command(root, "make", rest.split_whitespace().map(OsString::from))?
    } else if let Some(rest) = command.strip_prefix("cd workers/rust && cargo ") {
        run_command(
            &root.join("workers/rust"),
            "cargo",
            rest.split_whitespace().map(OsString::from),
        )?
    } else {
        return Err(format!("unsupported command {command}"));
    };
    Ok(json!({
        "ok": status == 0,
        "status": status,
        "duration_ms": started.elapsed().as_millis() as u64,
        "stdout_tail": [],
        "stderr_tail": [],
    }))
}

fn validate_report(report: &Value, options: &Options) -> RunnerResult<()> {
    if field(report, "schema_version") != REPORT_SCHEMA_VERSION {
        return Err(format!(
            "report schema_version must be {REPORT_SCHEMA_VERSION}"
        ));
    }
    if field(report, "source") != options.config {
        return Err(format!("report source must be {}", options.config));
    }
    require_boolean(report.get("executed"), "executed", "report")?;
    require_number(report.get("profile_count"), "profile_count", "report")?;
    require_boolean(report.get("ok"), "ok", "report")?;
    let profiles = report
        .get("profiles")
        .and_then(Value::as_array)
        .ok_or_else(|| "report profiles must be an array".to_string())?;
    if report.get("profile_count").and_then(Value::as_u64) != Some(profiles.len() as u64) {
        return Err("report profile_count must match profiles length".to_string());
    }
    let mut expected_ok = true;
    for (index, profile) in profiles.iter().enumerate() {
        validate_report_profile(
            profile,
            report.get("executed").and_then(Value::as_bool) == Some(true),
            index,
        )?;
        expected_ok &= profile.get("ok").and_then(Value::as_bool) == Some(true);
    }
    if report.get("ok").and_then(Value::as_bool) != Some(expected_ok) {
        return Err("report ok must equal the profile status rollup".to_string());
    }
    Ok(())
}

fn validate_input_report(report: &Value, options: &Options) -> RunnerResult<()> {
    if report.get("executed").and_then(Value::as_bool) != Some(true) {
        return Err("input report must be executed=true".to_string());
    }
    if report.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err("input report must be ok=true".to_string());
    }
    if let Some(expected_profile) = options.profile.as_deref() {
        let profiles = report
            .get("profiles")
            .and_then(Value::as_array)
            .ok_or_else(|| "report profiles must be an array".to_string())?;
        if profiles.len() != 1 || field(&profiles[0], "profile_id") != expected_profile {
            return Err(format!(
                "input report must contain only profile {expected_profile}"
            ));
        }
    }
    Ok(())
}

fn validate_report_profile(profile: &Value, executed: bool, index: usize) -> RunnerResult<()> {
    let context = format!("report.profiles/{index}");
    require_string(profile.get("profile_id"), "profile_id", &context)?;
    require_string(profile.get("trust_goal"), "trust_goal", &context)?;
    for field_name in [
        "operators",
        "validation_methods",
        "formal_invariants",
        "evidence_paths",
    ] {
        require_string_array(profile.get(field_name), field_name, &context)?;
    }
    require_boolean(profile.get("ok"), "ok", &context)?;
    let commands = profile
        .get("commands")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}: commands must be non-empty"))?;
    if commands.is_empty() {
        return Err(format!("{context}: commands must be non-empty"));
    }
    for (command_index, command) in commands.iter().enumerate() {
        validate_report_command(
            command,
            executed,
            &format!("{context}.commands/{command_index}"),
        )?;
    }
    Ok(())
}

fn validate_report_command(command: &Value, executed: bool, context: &str) -> RunnerResult<()> {
    require_string(command.get("id"), "id", context)?;
    require_string(command.get("kind"), "kind", context)?;
    require_string(command.get("command"), "command", context)?;
    let kind = field(command, "kind");
    if !ALLOWED_KINDS.contains(&kind) {
        return Err(format!("{context}: unsupported command kind {kind}"));
    }
    let result = command
        .get("result")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{context}: result must be an object"))?;
    if executed {
        require_boolean(result.get("ok"), "result.ok", context)?;
        let status = result.get("status");
        if !(status.and_then(Value::as_i64).is_some() || status == Some(&Value::Null)) {
            return Err(format!(
                "{context}: result.status must be an integer or null"
            ));
        }
        require_number(result.get("duration_ms"), "result.duration_ms", context)?;
        require_string_list(result.get("stdout_tail"), "result.stdout_tail", context)?;
        require_string_list(result.get("stderr_tail"), "result.stderr_tail", context)?;
    } else if result.get("ok") != Some(&Value::Null)
        || result.get("status").and_then(Value::as_str) != Some("not_run")
    {
        return Err(format!(
            "{context}: skipped command result must be ok=null,status=not_run"
        ));
    }
    Ok(())
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    assert_schema_command_kinds(root, PROFILE_SCHEMA)?;
    assert_schema_command_kinds(root, REPORT_SCHEMA)?;
    let mut sample = serde_json::json!({
        "schema_version": SCHEMA_VERSION,
        "version_line": "moxi test",
        "profiles": [{
            "profile_id": "sample",
            "profile_role": "release_candidate",
            "qualification_candidate_id": "sample",
            "trust_goal": "review",
            "operators": ["solve.sample"],
            "validation_methods": ["analytic"],
            "formal_invariants": ["finite"],
            "evidence_paths": ["docs/operator-reliability.md"],
            "commands": [
                { "id": "smoke", "kind": "contract", "command": "make check-make-modules" },
                {
                    "id": "boundary",
                    "kind": "boundary_regression",
                    "command": "cd workers/rust && cargo test -p kyuubiki-solver --test stokes_flow_triangle_reliability"
                }
            ]
        }]
    });
    validate_config(root, &sample)?;
    sample["profiles"][0]["commands"][0]["command"] = Value::from("python -c 'print(1)'");
    expect_error(validate_config(root, &sample), "unsupported command prefix")?;
    sample["profiles"][0]["commands"][0]["command"] = Value::from("make check-make-modules");
    sample["profiles"][0]["commands"][1]["kind"] = Value::from("ad_hoc");
    expect_error(validate_config(root, &sample), "unsupported command kind")?;
    sample["profiles"][0]["commands"][1]["kind"] = Value::from("boundary_regression");
    let mut report = build_report(
        root,
        &sample,
        &Options {
            config: "config/sample.json".to_string(),
            input_report: None,
            out: DEFAULT_OUT.to_string(),
            profile: None,
            execute: false,
            self_test: false,
        },
    )?;
    validate_report(
        &report,
        &Options {
            config: "config/sample.json".to_string(),
            input_report: None,
            out: DEFAULT_OUT.to_string(),
            profile: None,
            execute: false,
            self_test: false,
        },
    )?;
    let mut executed_report = report.clone();
    executed_report["executed"] = Value::from(true);
    for command in executed_report["profiles"][0]["commands"]
        .as_array_mut()
        .ok_or_else(|| "self-test report commands must be an array".to_string())?
    {
        command["result"] = json!({
            "ok": true,
            "status": 0,
            "duration_ms": 1,
            "stdout_tail": [],
            "stderr_tail": [],
        });
    }
    validate_report(
        &executed_report,
        &Options {
            config: "config/sample.json".to_string(),
            input_report: None,
            out: DEFAULT_OUT.to_string(),
            profile: Some("sample".to_string()),
            execute: false,
            self_test: false,
        },
    )?;
    validate_input_report(
        &executed_report,
        &Options {
            config: "config/sample.json".to_string(),
            input_report: None,
            out: DEFAULT_OUT.to_string(),
            profile: Some("sample".to_string()),
            execute: false,
            self_test: false,
        },
    )?;
    expect_error(
        validate_input_report(
            &report,
            &Options {
                config: "config/sample.json".to_string(),
                input_report: None,
                out: DEFAULT_OUT.to_string(),
                profile: Some("sample".to_string()),
                execute: false,
                self_test: false,
            },
        ),
        "executed=true",
    )?;
    expect_error(
        build_report(
            root,
            &sample,
            &Options {
                config: "config/sample.json".to_string(),
                input_report: None,
                out: DEFAULT_OUT.to_string(),
                profile: Some("missing".to_string()),
                execute: false,
                self_test: false,
            },
        )
        .map(|_| ()),
        "no operator validation profiles matched",
    )?;
    report["profile_count"] = Value::from(2);
    expect_error(
        validate_report(
            &report,
            &Options {
                config: "config/sample.json".to_string(),
                input_report: None,
                out: DEFAULT_OUT.to_string(),
                profile: None,
                execute: false,
                self_test: false,
            },
        ),
        "profile_count",
    )
}

fn assert_schema_command_kinds(root: &Path, relative_path: &str) -> RunnerResult<()> {
    let schema = read_json(root, relative_path)?;
    let values = schema
        .pointer("/$defs/commandKind/enum")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{relative_path}: missing $defs.commandKind.enum"))?
        .iter()
        .filter_map(Value::as_str)
        .collect::<HashSet<_>>();
    let allowed = ALLOWED_KINDS.iter().copied().collect::<HashSet<_>>();
    if values != allowed {
        return Err(format!(
            "{relative_path}: command kinds do not match allowlist"
        ));
    }
    Ok(())
}

fn expect_error(result: RunnerResult<()>, expected: &str) -> RunnerResult<()> {
    match result {
        Ok(()) => Err(format!("self-test expected error containing {expected}")),
        Err(message) if message.contains(expected) => Ok(()),
        Err(message) => Err(format!("self-test expected {expected}, got {message}")),
    }
}

fn write_report(root: &Path, report: &Value, out_path: &str) -> RunnerResult<()> {
    let absolute = repo_path(root, out_path)?;
    if let Some(parent) = absolute.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to encode report: {error}"))?;
    fs::write(&absolute, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", absolute.display()))
}

fn repo_path(root: &Path, relative_path: &str) -> RunnerResult<PathBuf> {
    let absolute = root.join(relative_path);
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("path must stay inside repository: {relative_path}"))?;
    if relative.starts_with("..") || relative.is_absolute() {
        return Err(format!("path must stay inside repository: {relative_path}"));
    }
    Ok(absolute)
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(repo_path(root, relative_path)?)
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn require_string(value: Option<&Value>, field: &str, context: &str) -> RunnerResult<()> {
    if value
        .and_then(Value::as_str)
        .is_none_or(|text| text.trim().is_empty())
    {
        return Err(format!("{context}: {field} must be a non-empty string"));
    }
    Ok(())
}

fn require_string_array(value: Option<&Value>, field: &str, context: &str) -> RunnerResult<()> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Err(format!("{context}: {field} must be a non-empty array"));
    };
    if items.is_empty() {
        return Err(format!("{context}: {field} must be a non-empty array"));
    }
    for (index, item) in items.iter().enumerate() {
        require_string(Some(item), &format!("{field}[{index}]"), context)?;
    }
    Ok(())
}

fn require_string_list(value: Option<&Value>, field: &str, context: &str) -> RunnerResult<()> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Err(format!("{context}: {field} must be an array"));
    };
    for (index, item) in items.iter().enumerate() {
        require_string(Some(item), &format!("{field}[{index}]"), context)?;
    }
    Ok(())
}

fn require_boolean(value: Option<&Value>, field: &str, context: &str) -> RunnerResult<()> {
    if !value.is_some_and(Value::is_boolean) {
        return Err(format!("{context}: {field} must be a boolean"));
    }
    Ok(())
}

fn require_number(value: Option<&Value>, field: &str, context: &str) -> RunnerResult<()> {
    if !value
        .and_then(Value::as_i64)
        .is_some_and(|number| number >= 0)
    {
        return Err(format!("{context}: {field} must be a non-negative integer"));
    }
    Ok(())
}

fn string_array(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{parse_args, validate_command};
    use std::ffi::OsString;

    #[test]
    fn parse_execute_flag() {
        let options = parse_args(vec![OsString::from("--execute")]).expect("options");
        assert!(options.execute);
    }

    #[test]
    fn rejects_unsupported_command_prefix() {
        let command = serde_json::json!({
            "id": "bad",
            "kind": "contract",
            "command": "python -c 'print(1)'"
        });
        assert!(validate_command(&command, "self").is_err());
    }
}
