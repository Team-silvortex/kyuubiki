use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

const CONTRACT_PATH: &str = "config/test-coverage-posture.json";
const CONTRACT_SCHEMA: &str = "kyuubiki.test-coverage-posture/v1";
const REPORT_SCHEMA: &str = "kyuubiki.test-coverage-posture-report/v1";
const REQUIRED_CLAIM: &str = "not-100-percent-until-artifacts-and-thresholds-exist";
const STATUS_ORDER: &[&str] = &["declared", "instrumented", "thresholded", "enforced"];

#[derive(Debug, Eq, PartialEq)]
struct Options {
    out: Option<String>,
    markdown_out: Option<String>,
    strict: bool,
    self_test: bool,
}

#[derive(Debug, Eq, PartialEq)]
struct RustCoverageOptions {
    out: String,
    package: Option<String>,
    test_filter: Option<String>,
    self_test: bool,
}

pub(crate) fn run_check_test_coverage_posture(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    if options.self_test {
        run_self_test()?;
        println!("test coverage posture self-test passed");
        return Ok(0);
    }

    let contract = read_json(root, CONTRACT_PATH)?;
    let mut issues = validate_contract(root, &contract)?;
    let report = build_report(root, &contract)?;

    if options.strict && !all_code_surfaces_enforced(&report) {
        issues.push("strict mode requires every code surface to be status=enforced".to_string());
    }
    if let Some(out) = &options.out {
        write_text(root, out, &format_json(&report)?)?;
    }
    if let Some(out) = &options.markdown_out {
        write_text(root, out, &render_markdown(&report))?;
    }

    print_summary(&report, &options);
    if issues.is_empty() {
        Ok(0)
    } else {
        eprintln!("test coverage posture issues:");
        for issue in issues {
            eprintln!("- {issue}");
        }
        Ok(1)
    }
}

pub(crate) fn run_rust_coverage(root: &Path, rust: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = RustCoverageOptions::parse(args)?;
    if options.self_test {
        run_rust_coverage_self_test()?;
        println!("rust coverage command self-test passed");
        return Ok(0);
    }
    validate_output_path(&options.out)?;
    let out_path = root.join(&options.out);
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let mut cargo_args = vec!["llvm-cov".to_string()];
    if let Some(package) = &options.package {
        cargo_args.extend(["-p".to_string(), package.clone()]);
    } else {
        cargo_args.push("--workspace".to_string());
    }
    cargo_args.extend([
        "--lcov".to_string(),
        "--summary-only".to_string(),
        "--output-path".to_string(),
        out_path.to_string_lossy().into_owned(),
    ]);
    if let Some(test_filter) = &options.test_filter {
        cargo_args.extend(["--".to_string(), test_filter.clone()]);
    }

    let status = Command::new("cargo")
        .args(&cargo_args)
        .current_dir(rust)
        .status()
        .map_err(|error| format!("failed to run cargo llvm-cov: {error}"))?;
    let code = status.code().unwrap_or(1) as u8;
    if code == 0 {
        println!("rust coverage lcov report: {}", options.out);
    }
    Ok(code)
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Options {
            out: None,
            markdown_out: None,
            strict: false,
            self_test: false,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            let arg = arg
                .into_string()
                .map_err(|_| "coverage posture argument is not valid utf-8".to_string())?;
            match arg.as_str() {
                "--help" | "-h" => {
                    println!(
                        "usage: kyuubiki-script-runner check-test-coverage-posture [--out <path>] [--markdown-out <path>] [--strict] [--self-test]"
                    );
                    options.self_test = true;
                }
                "--self-test" => options.self_test = true,
                "--strict" => options.strict = true,
                "--out" => options.out = Some(next_arg(&mut iter, "--out")?),
                "--markdown-out" => {
                    options.markdown_out = Some(next_arg(&mut iter, "--markdown-out")?)
                }
                other => {
                    return Err(format!(
                        "unknown check-test-coverage-posture argument: {other}"
                    ));
                }
            }
        }
        Ok(options)
    }
}

impl RustCoverageOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = RustCoverageOptions {
            out: "tmp/coverage/rust/lcov.info".to_string(),
            package: None,
            test_filter: None,
            self_test: false,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            let arg = arg
                .into_string()
                .map_err(|_| "rust coverage argument is not valid utf-8".to_string())?;
            match arg.as_str() {
                "--help" | "-h" => {
                    println!(
                        "usage: kyuubiki-script-runner rust-coverage [--out <path>] [--package <name>] [--test-filter <filter>] [--self-test]"
                    );
                    options.self_test = true;
                }
                "--self-test" => options.self_test = true,
                "--out" => options.out = next_arg(&mut iter, "--out")?,
                "--package" => options.package = Some(next_arg(&mut iter, "--package")?),
                "--test-filter" => {
                    options.test_filter = Some(next_arg(&mut iter, "--test-filter")?)
                }
                other => return Err(format!("unknown rust-coverage argument: {other}")),
            }
        }
        Ok(options)
    }
}

fn next_arg(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .ok_or_else(|| format!("{flag} requires a value"))?
        .into_string()
        .map_err(|_| format!("{flag} value is not valid utf-8"))
}

fn validate_contract(root: &Path, contract: &Value) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    if string_at(contract, "schema_version") != CONTRACT_SCHEMA {
        issues.push(format!(
            "{CONTRACT_PATH}: schema_version must be {CONTRACT_SCHEMA}"
        ));
    }
    if string_at(
        pointer(contract, "/truthfulness_policy"),
        "code_coverage_claim",
    ) != REQUIRED_CLAIM
    {
        issues.push(format!(
            "{CONTRACT_PATH}: truthfulness_policy.code_coverage_claim must be {REQUIRED_CLAIM}"
        ));
    }

    let mut ids = BTreeSet::new();
    let surfaces = array_at(contract, "code_surfaces", &mut issues);
    for (index, surface) in surfaces.iter().enumerate() {
        validate_code_surface(root, &mut issues, &mut ids, surface, index);
    }
    let gates = array_at(contract, "non_code_coverage_gates", &mut issues);
    for (index, gate) in gates.iter().enumerate() {
        validate_gate(&mut issues, gate, index);
    }
    Ok(issues)
}

fn validate_code_surface(
    root: &Path,
    issues: &mut Vec<String>,
    ids: &mut BTreeSet<String>,
    surface: &Value,
    index: usize,
) {
    let context = format!("code_surfaces[{index}]");
    let id = required_string(issues, surface, "id", &context);
    if let Some(id) = id {
        if !id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
            issues.push(format!("{context}.id must be kebab-case ascii"));
        }
        if !ids.insert(id.to_string()) {
            issues.push(format!("{context}.id duplicates {id}"));
        }
    }
    for field in [
        "owner",
        "language",
        "test_command",
        "planned_tool",
        "planned_command",
    ] {
        required_string(issues, surface, field, &context);
    }
    for field in ["root", "planned_artifact"] {
        if let Some(value) = required_string(issues, surface, field, &context) {
            validate_relative_path(issues, value, &format!("{context}.{field}"));
        }
    }
    if let Some(value) = string_value(surface, "root") {
        if !root.join(value).exists() {
            issues.push(format!("{context}.root does not exist: {value}"));
        }
    }
    let status = string_value(surface, "status").unwrap_or_default();
    if !STATUS_ORDER.contains(&status) {
        issues.push(format!("{context}.status has unknown value: {status}"));
    }
    validate_thresholds(issues, surface.get("minimum_next_threshold"), &context);
}

fn validate_gate(issues: &mut Vec<String>, gate: &Value, index: usize) {
    let context = format!("non_code_coverage_gates[{index}]");
    for field in ["id", "kind", "command"] {
        required_string(issues, gate, field, &context);
    }
}

fn validate_thresholds(issues: &mut Vec<String>, thresholds: Option<&Value>, context: &str) {
    let Some(thresholds) = thresholds else {
        issues.push(format!("{context}.minimum_next_threshold is required"));
        return;
    };
    let Some(object) = thresholds.as_object() else {
        issues.push(format!(
            "{context}.minimum_next_threshold must be an object"
        ));
        return;
    };
    for field in ["lines", "branches", "functions"] {
        match object.get(field).and_then(Value::as_u64) {
            Some(value) if value <= 100 => {}
            _ => issues.push(format!(
                "{context}.minimum_next_threshold.{field} must be 0..100"
            )),
        }
    }
}

fn validate_relative_path(issues: &mut Vec<String>, value: &str, context: &str) {
    if value.is_empty() {
        issues.push(format!("{context} must not be empty"));
    }
    if Path::new(value).is_absolute() {
        issues.push(format!(
            "{context} must be repository-relative, not absolute"
        ));
    }
    if value.split('/').any(|part| part == "..") {
        issues.push(format!("{context} must not contain '..'"));
    }
}

fn build_report(root: &Path, contract: &Value) -> RunnerResult<Value> {
    let surfaces = contract
        .get("code_surfaces")
        .and_then(Value::as_array)
        .ok_or_else(|| "code_surfaces must be an array".to_string())?;
    let gates = contract
        .get("non_code_coverage_gates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut status_counts = BTreeMap::new();
    let surface_reports = surfaces
        .iter()
        .map(|surface| {
            let status = string_value(surface, "status").unwrap_or("unknown");
            *status_counts.entry(status.to_string()).or_insert(0usize) += 1;
            let artifact = string_value(surface, "planned_artifact").unwrap_or("");
            json!({
                "id": string_value(surface, "id").unwrap_or("unknown"),
                "owner": string_value(surface, "owner").unwrap_or("unknown"),
                "language": string_value(surface, "language").unwrap_or("unknown"),
                "root": string_value(surface, "root").unwrap_or(""),
                "test_command": string_value(surface, "test_command").unwrap_or(""),
                "planned_tool": string_value(surface, "planned_tool").unwrap_or(""),
                "planned_command": string_value(surface, "planned_command").unwrap_or(""),
                "planned_artifact": artifact,
                "artifact_exists": !artifact.is_empty() && root.join(artifact).exists(),
                "status": status,
                "minimum_next_threshold": surface.get("minimum_next_threshold").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();
    let instrumented = count_status_at_least(&status_counts, "instrumented");
    let thresholded = count_status_at_least(&status_counts, "thresholded");
    let enforced = count_status_at_least(&status_counts, "enforced");
    Ok(json!({
        "schema_version": REPORT_SCHEMA,
        "source": CONTRACT_PATH,
        "release_line": string_value(contract, "release_line").unwrap_or("unknown"),
        "traditional_code_coverage_is_100_percent": false,
        "summary": {
            "code_surface_count": surfaces.len(),
            "instrumented_surface_count": instrumented,
            "thresholded_surface_count": thresholded,
            "enforced_surface_count": enforced,
            "status_counts": status_counts,
            "status": if enforced == surfaces.len() { "enforced" } else { "not-yet-enforced" },
            "reason": "traditional code coverage is not 100% until every code surface has retained artifacts and enforced thresholds"
        },
        "code_surfaces": surface_reports,
        "non_code_coverage_gates": gates,
    }))
}

fn count_status_at_least(counts: &BTreeMap<String, usize>, minimum: &str) -> usize {
    let minimum_index = status_index(minimum).unwrap_or(usize::MAX);
    counts
        .iter()
        .filter(|(status, _)| status_index(status).is_some_and(|index| index >= minimum_index))
        .map(|(_, count)| *count)
        .sum()
}

fn all_code_surfaces_enforced(report: &Value) -> bool {
    report.pointer("/summary/status").and_then(Value::as_str) == Some("enforced")
}

fn status_index(status: &str) -> Option<usize> {
    STATUS_ORDER.iter().position(|value| *value == status)
}

fn print_summary(report: &Value, options: &Options) {
    let summary = pointer(report, "/summary");
    println!(
        "test coverage posture: {} code surface(s), {} instrumented, {} thresholded, {} enforced; traditional code coverage is not 100%.",
        number_at(summary, "code_surface_count"),
        number_at(summary, "instrumented_surface_count"),
        number_at(summary, "thresholded_surface_count"),
        number_at(summary, "enforced_surface_count"),
    );
    if let Some(out) = &options.out {
        println!("json report: {out}");
    }
    if let Some(out) = &options.markdown_out {
        println!("markdown report: {out}");
    }
}

fn render_markdown(report: &Value) -> String {
    let summary = pointer(report, "/summary");
    let mut lines = vec![
        "# Test Coverage Posture".to_string(),
        String::new(),
        format!("- Release line: `{}`", string_at(report, "release_line")),
        "- Traditional code coverage: `not 100%`".to_string(),
        format!(
            "- Code surfaces: `{}` total, `{}` instrumented, `{}` thresholded, `{}` enforced",
            number_at(summary, "code_surface_count"),
            number_at(summary, "instrumented_surface_count"),
            number_at(summary, "thresholded_surface_count"),
            number_at(summary, "enforced_surface_count"),
        ),
        String::new(),
        "## Code Surfaces".to_string(),
        String::new(),
        "| Surface | Owner | Language | Status | Planned tool | Artifact |".to_string(),
        "| --- | --- | --- | --- | --- | --- |".to_string(),
    ];
    for surface in report
        .get("code_surfaces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |",
            string_at(surface, "id"),
            string_at(surface, "owner"),
            string_at(surface, "language"),
            string_at(surface, "status"),
            string_at(surface, "planned_tool"),
            string_at(surface, "planned_artifact"),
        ));
    }
    lines.push(String::new());
    lines.push("## Existing Non-Code Coverage Gates".to_string());
    lines.push(String::new());
    lines.push("| Gate | Kind | Command |".to_string());
    lines.push("| --- | --- | --- |".to_string());
    for gate in report
        .get("non_code_coverage_gates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        lines.push(format!(
            "| `{}` | `{}` | `{}` |",
            string_at(gate, "id"),
            string_at(gate, "kind"),
            string_at(gate, "command"),
        ));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn read_json(root: &Path, relative: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(root.join(relative))
        .map_err(|error| format!("failed to read {relative}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("failed to parse {relative}: {error}"))
}

fn write_text(root: &Path, relative: &str, text: &str) -> RunnerResult<()> {
    validate_output_path(relative)?;
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(&path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn validate_output_path(relative: &str) -> RunnerResult<()> {
    if Path::new(relative).is_absolute() || relative.split('/').any(|part| part == "..") {
        return Err("coverage posture output path must be repository-relative".to_string());
    }
    Ok(())
}

fn format_json(value: &Value) -> RunnerResult<String> {
    serde_json::to_string_pretty(value).map_err(|error| format!("failed to encode json: {error}"))
}

fn array_at<'a>(value: &'a Value, field: &str, issues: &mut Vec<String>) -> Vec<&'a Value> {
    match value.get(field).and_then(Value::as_array) {
        Some(values) if !values.is_empty() => values.iter().collect(),
        Some(_) => {
            issues.push(format!("{CONTRACT_PATH}: {field} must not be empty"));
            Vec::new()
        }
        None => {
            issues.push(format!("{CONTRACT_PATH}: {field} must be an array"));
            Vec::new()
        }
    }
}

fn required_string<'a>(
    issues: &mut Vec<String>,
    value: &'a Value,
    field: &str,
    context: &str,
) -> Option<&'a str> {
    let found = string_value(value, field);
    if found.is_none_or(str::is_empty) {
        issues.push(format!("{context}.{field} must be a non-empty string"));
    }
    found
}

fn string_value<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn string_at<'a>(value: &'a Value, field: &str) -> &'a str {
    string_value(value, field).unwrap_or("")
}

fn number_at(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or_default()
}

fn pointer<'a>(value: &'a Value, path: &str) -> &'a Value {
    value.pointer(path).unwrap_or(&Value::Null)
}

fn run_self_test() -> RunnerResult<()> {
    let valid = json!({
        "schema_version": CONTRACT_SCHEMA,
        "release_line": "moxi 2.x",
        "truthfulness_policy": {
            "code_coverage_claim": REQUIRED_CLAIM,
            "separate_from": ["module-function coverage tensor"]
        },
        "code_surfaces": [{
            "id": "rust-workspace",
            "owner": "runtime-data-plane",
            "language": "rust",
            "root": ".",
            "test_command": "make test-rust",
            "planned_tool": "cargo llvm-cov",
            "planned_command": "cargo llvm-cov --workspace",
            "planned_artifact": "tmp/coverage/rust/lcov.info",
            "status": "declared",
            "minimum_next_threshold": {"lines": 70, "branches": 50, "functions": 60}
        }],
        "non_code_coverage_gates": [{
            "id": "module-function-coverage-tensor",
            "kind": "architecture tensor",
            "command": "make check-module-function-coverage-tensor"
        }]
    });
    let issues = validate_contract(Path::new("."), &valid)?;
    if !issues.is_empty() {
        return Err(format!("valid coverage posture fixture failed: {issues:?}"));
    }
    let report = build_report(Path::new("."), &valid)?;
    if all_code_surfaces_enforced(&report) {
        return Err("declared-only fixture must not be enforced".to_string());
    }

    let mut invalid = valid.clone();
    invalid["truthfulness_policy"]["code_coverage_claim"] = Value::String("100-percent".into());
    invalid["code_surfaces"][0]["planned_artifact"] = Value::String("/tmp/lcov.info".into());
    invalid["code_surfaces"][0]["minimum_next_threshold"]["lines"] = Value::from(101);
    let issues = validate_contract(Path::new("."), &invalid)?;
    for needle in [
        "truthfulness_policy.code_coverage_claim",
        "planned_artifact must be repository-relative",
        "minimum_next_threshold.lines",
    ] {
        if !issues.iter().any(|issue| issue.contains(needle)) {
            return Err(format!(
                "invalid fixture did not report {needle}: {issues:?}"
            ));
        }
    }
    Ok(())
}

fn run_rust_coverage_self_test() -> RunnerResult<()> {
    let parsed = RustCoverageOptions::parse(
        [
            "--out",
            "tmp/coverage/rust/lcov.info",
            "--package",
            "kyuubiki-script-runner",
            "--test-filter",
            "coverage",
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
    )?;
    if parsed.package.as_deref() != Some("kyuubiki-script-runner") {
        return Err("rust coverage package parsing failed".to_string());
    }
    if validate_output_path("/tmp/lcov.info").is_ok() {
        return Err("rust coverage output must reject absolute paths".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_test_passes() {
        run_self_test().unwrap();
    }

    #[test]
    fn rust_coverage_self_test_passes() {
        run_rust_coverage_self_test().unwrap();
    }
}
