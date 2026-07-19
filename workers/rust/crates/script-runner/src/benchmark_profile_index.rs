use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_ROOT: &str = "tmp/benchmark-profile";
const DEFAULT_COVERAGE_TARGETS: &str = "config/benchmark-profile-coverage.json";

type RunnerResult<T> = Result<T, String>;

mod coverage;
mod markdown;
mod summaries;

use coverage::{coverage_summaries, profile_coverage_summaries};
use summaries::{matrix_summaries, solver_strategy_summaries};

pub(crate) fn run_build_benchmark_profile_index(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(repo_root, args)?;
    let coverage_targets = read_coverage_targets(&options.coverage_targets)?;
    let mut discovery = discover_runs(&options.root)?;
    annotate_resolved_failures(&mut discovery.failures, &discovery.runs);
    let coverage_summaries = coverage_summaries(&discovery.runs, &coverage_targets);
    let profile_coverage_summaries = profile_coverage_summaries(&coverage_summaries);
    let gate = evaluate_gate(
        &discovery.runs,
        &discovery.failures,
        &discovery.skipped,
        &coverage_targets,
    );
    let payload = json!({
        "schema_version": "kyuubiki.benchmark-profile-index/v1",
        "root": display_path(repo_root, &options.root),
        "coverage_targets_manifest": display_path(repo_root, &options.coverage_targets),
        "generated_at_unix_s": unix_seconds_now(),
        "gate": gate,
        "coverage_summaries": coverage_summaries,
        "profile_coverage_summaries": profile_coverage_summaries,
        "matrix_summaries": matrix_summaries(&discovery.runs),
        "solver_strategy_summaries": solver_strategy_summaries(&discovery.runs),
        "failed_runs": discovery.failures,
        "skipped_runs": discovery.skipped,
        "retained_runs": discovery.runs,
    });
    fs::create_dir_all(&options.root)
        .map_err(|error| format!("failed to create {}: {error}", options.root.display()))?;
    write_json(&options.root.join("index.json"), &payload)?;
    fs::write(
        options.root.join("README.md"),
        markdown::render_readme(
            &display_path(repo_root, &options.root),
            &display_path(repo_root, &options.coverage_targets),
            &payload,
        ),
    )
    .map_err(|error| format!("failed to write README.md: {error}"))?;
    println!("{}", display_path(repo_root, &options.root));
    Ok(0)
}

struct Options {
    coverage_targets: PathBuf,
    root: PathBuf,
}

fn parse_args(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        coverage_targets: repo_root.join(DEFAULT_COVERAGE_TARGETS),
        root: repo_root.join(DEFAULT_ROOT),
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner build-benchmark-profile-index [--root tmp/benchmark-profile] [--coverage-targets config/benchmark-profile-coverage.json]"
                );
                return Ok(options);
            }
            "--root" => options.root = path_arg(repo_root, &mut iter, "--root")?,
            "--coverage-targets" => {
                options.coverage_targets = path_arg(repo_root, &mut iter, "--coverage-targets")?
            }
            other => return Err(format!("unknown or incomplete argument: {other}")),
        }
    }
    Ok(options)
}

fn path_arg(
    repo_root: &Path,
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<PathBuf> {
    let value = iter
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))?;
    Ok(if value.is_absolute() {
        value
    } else {
        repo_root.join(value)
    })
}

#[derive(Clone, Debug)]
struct CoverageTarget {
    expected_cases: Vec<String>,
    matrix: String,
    profile: String,
    scale_limit_reasons: BTreeMap<String, String>,
    scale_limit_remediations: BTreeMap<String, String>,
}

fn read_coverage_targets(path: &Path) -> RunnerResult<Vec<CoverageTarget>> {
    let payload = read_json(path)?;
    let targets = payload
        .get("targets")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "coverage targets {} must define a non-empty targets array",
                path.display()
            )
        })?;
    if targets.is_empty() {
        return Err(format!(
            "coverage targets {} must define a non-empty targets array",
            path.display()
        ));
    }
    targets
        .iter()
        .enumerate()
        .map(|(index, target)| validate_coverage_target(path, target, index))
        .collect()
}

fn validate_coverage_target(
    path: &Path,
    target: &Value,
    index: usize,
) -> RunnerResult<CoverageTarget> {
    let prefix = format!("coverage target {index} in {}", path.display());
    let matrix = non_empty_string(target, "matrix")
        .ok_or_else(|| format!("{prefix} must define a non-empty matrix"))?;
    let profile = non_empty_string(target, "profile")
        .ok_or_else(|| format!("{prefix} must define a non-empty profile"))?;
    let expected = target
        .get("expected_cases")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{prefix} must define a non-empty expected_cases array"))?;
    if expected.is_empty() {
        return Err(format!(
            "{prefix} must define a non-empty expected_cases array"
        ));
    }
    let mut seen = BTreeSet::new();
    let mut expected_cases = Vec::new();
    for (case_index, value) in expected.iter().enumerate() {
        let case = value
            .as_str()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| {
                format!("{prefix} expected_cases[{case_index}] must be a non-empty string")
            })?;
        if !seen.insert(case.to_string()) {
            return Err(format!("{prefix} has duplicate expected_cases entries"));
        }
        expected_cases.push(case.to_string());
    }
    let mut scale_limit_reasons = BTreeMap::new();
    let mut scale_limit_remediations = BTreeMap::new();
    if let Some(reasons) = target.get("scale_limit_reasons") {
        let reasons = reasons.as_object().ok_or_else(|| {
            format!("{prefix} scale_limit_reasons must be an object when provided")
        })?;
        for (case, reason) in reasons {
            if !seen.contains(case) {
                return Err(format!(
                    "{prefix} scale_limit_reasons contains unknown case {case}"
                ));
            }
            let reason = reason
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    format!("{prefix} scale_limit_reasons[{case}] must be a non-empty string")
                })?;
            scale_limit_reasons.insert(case.clone(), reason.to_string());
        }
    }
    if let Some(remediations) = target.get("scale_limit_remediations") {
        let remediations = remediations.as_object().ok_or_else(|| {
            format!("{prefix} scale_limit_remediations must be an object when provided")
        })?;
        for (case, remediation) in remediations {
            if !seen.contains(case) {
                return Err(format!(
                    "{prefix} scale_limit_remediations contains unknown case {case}"
                ));
            }
            let remediation = remediation
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    format!("{prefix} scale_limit_remediations[{case}] must be a non-empty string")
                })?;
            scale_limit_remediations.insert(case.clone(), remediation.to_string());
        }
    }
    Ok(CoverageTarget {
        expected_cases,
        matrix,
        profile,
        scale_limit_reasons,
        scale_limit_remediations,
    })
}

struct Discovery {
    failures: Vec<Value>,
    runs: Vec<Value>,
    skipped: Vec<Value>,
}

fn discover_runs(root: &Path) -> RunnerResult<Discovery> {
    let mut failures = Vec::new();
    let mut runs = Vec::new();
    let mut skipped = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(Discovery {
            failures: Vec::new(),
            runs,
            skipped,
        });
    };
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read {}: {error}", root.display()))?;
        let run_dir = entry.path();
        if !run_dir.is_dir() {
            continue;
        }
        let summary_path = run_dir.join("summary.json");
        let slug = entry.file_name().to_string_lossy().to_string();
        if summary_path.exists() {
            match read_json(&summary_path) {
                Ok(summary) => runs.push(run_row(root, &slug, &run_dir, &summary_path, &summary)?),
                Err(error) => skipped.push(
                    json!({ "slug": slug, "reason": format!("failed to parse summary.json: {error}") }),
                ),
            }
        }
        let failure_path = run_dir.join("failure.json");
        if failure_path.exists() {
            match read_json(&failure_path) {
                Ok(failure) => failures.push(failure_row(root, &slug, &failure_path, &failure)?),
                Err(error) => skipped.push(
                    json!({ "slug": slug, "reason": format!("failed to parse failure.json: {error}") }),
                ),
            }
        }
    }
    runs.sort_by(|left, right| {
        number_field(right, "generated_at_unix_s")
            .total_cmp(&number_field(left, "generated_at_unix_s"))
    });
    failures.sort_by(|left, right| {
        number_field(right, "generated_at_unix_s")
            .total_cmp(&number_field(left, "generated_at_unix_s"))
    });
    skipped.sort_by(|left, right| string_field(left, "slug").cmp(&string_field(right, "slug")));
    Ok(Discovery {
        failures,
        runs,
        skipped,
    })
}

fn run_row(
    root: &Path,
    slug: &str,
    run_dir: &Path,
    summary_path: &Path,
    summary: &Value,
) -> RunnerResult<Value> {
    let modified = summary_path
        .metadata()
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs());
    Ok(json!({
        "slug": slug,
        "generated_at_unix_s": modified,
        "profile": string_field(summary, "profile").if_empty("unknown"),
        "matrix": string_field(summary, "matrix").if_empty("unknown"),
        "case_count": summary.get("case_count").cloned().unwrap_or_else(|| json!(0)),
        "case_ids": summary.get("case_ids").and_then(Value::as_array).map(|items| {
            items.iter().filter_map(Value::as_str).map(Value::from).collect::<Vec<_>>()
        }).unwrap_or_default(),
        "case_shapes": run_case_shapes(run_dir),
        "solver_case_metrics": summary_case_metrics(summary, run_dir),
        "solver_preconditioners": summary_preconditioners(summary, run_dir),
        "total_median_ms": summary.get("total_median_ms").cloned().unwrap_or_else(|| json!(0)),
        "peak_rss_mib": summary.get("peak_rss_mib").cloned().unwrap_or_else(|| json!(0)),
        "slowest_case": string_field(summary, "slowest_case").if_empty("--"),
        "files": {
            "summary_json": display_path(root, summary_path),
            "readme_md": display_path(root, &run_dir.join("README.md")),
        },
    }))
}

fn summary_case_metrics(summary: &Value, run_dir: &Path) -> Vec<Value> {
    let declared = summary
        .get("solver_case_metrics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !declared.is_empty() {
        return declared;
    }
    raw_reports(run_dir)
        .into_iter()
        .flat_map(|report| report_case_metrics(&report))
        .collect()
}

fn summary_preconditioners(summary: &Value, run_dir: &Path) -> Vec<Value> {
    let declared = array_strings(summary, "solver_preconditioners");
    if !declared.is_empty() {
        return declared.into_iter().map(Value::from).collect();
    }
    raw_reports(run_dir)
        .iter()
        .flat_map(|report| report_preconditioners(&report))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(Value::from)
        .collect()
}

fn raw_reports(run_dir: &Path) -> Vec<Value> {
    let Ok(entries) = fs::read_dir(run_dir) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
                && !matches!(
                    path.file_name().and_then(|name| name.to_str()),
                    Some("summary.json" | "failure.json")
                )
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .iter()
        .filter_map(|path| read_json(path).ok())
        .collect()
}

fn report_preconditioners(report: &Value) -> Vec<String> {
    report
        .get("cases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|case| non_empty_string(case, "solver_preconditioner"))
        .collect()
}

fn report_case_metrics(report: &Value) -> Vec<Value> {
    report
        .get("cases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|case| {
            let id = non_empty_string(case, "id")?;
            let preconditioner = non_empty_string(case, "solver_preconditioner")?;
            Some(json!({
                "id": id,
                "solver_preconditioner": preconditioner,
                "solver_preconditioner_reason": case["solver_preconditioner_reason"].clone(),
                "solver_iterations": case["solver_iterations"].clone(),
                "solver_residual_norm": case["solver_residual_norm"].clone(),
            }))
        })
        .collect()
}

fn run_case_shapes(run_dir: &Path) -> Vec<Value> {
    raw_reports(run_dir)
        .into_iter()
        .flat_map(|report| {
            report
                .get("cases")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|case| {
                    let id = non_empty_string(case, "id")?;
                    Some(json!({
                        "id": id,
                        "node_count": case["node_count"].clone(),
                        "element_count": case["element_count"].clone(),
                        "dof_count": case["dof_count"].clone(),
                    }))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn failure_row(
    root: &Path,
    slug: &str,
    failure_path: &Path,
    failure: &Value,
) -> RunnerResult<Value> {
    let modified = failure_path
        .metadata()
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs());
    Ok(json!({
        "slug": slug,
        "generated_at_unix_s": modified,
        "profile": string_field(failure, "profile").if_empty("unknown"),
        "matrix": string_field(failure, "matrix").if_empty("unknown"),
        "case": string_field(failure, "case").if_empty("--"),
        "phase": string_field(failure, "phase").if_empty("unknown"),
        "failure_kind": normalized_failure_kind(failure),
        "exit_code": failure.get("exit_code").cloned().unwrap_or_else(|| json!(0)),
        "timed_out": failure.get("timed_out").and_then(Value::as_bool).unwrap_or(false),
        "resolved_by_success": false,
        "remote_host": string_field(failure, "remote_host").if_empty("unknown"),
        "files": { "failure_json": display_path(root, failure_path) },
    }))
}

fn annotate_resolved_failures(failures: &mut [Value], runs: &[Value]) {
    let mut successful_cases = BTreeMap::new();
    for run in runs {
        let matrix = string_field(run, "matrix");
        let profile = string_field(run, "profile");
        let slug = string_field(run, "slug");
        for case in observed_case_ids(run) {
            successful_cases
                .entry(benchmark_case_key(&matrix, &profile, &case))
                .or_insert_with(|| slug.clone());
        }
    }
    for failure in failures {
        let key = benchmark_case_key(
            &string_field(failure, "matrix"),
            &string_field(failure, "profile"),
            &string_field(failure, "case"),
        );
        if let Some(slug) = successful_cases.get(&key) {
            failure["resolved_by_success"] = Value::Bool(true);
            failure["resolved_by_slug"] = Value::String(slug.clone());
        }
    }
}

trait EmptyFallback {
    fn if_empty(self, fallback: &str) -> String;
}

impl EmptyFallback for String {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn evaluate_gate(
    runs: &[Value],
    failures: &[Value],
    skipped: &[Value],
    coverage_targets: &[CoverageTarget],
) -> Value {
    let mut reasons = skipped
        .iter()
        .map(|item| {
            format!(
                "skipped run {}: {}",
                string_field(item, "slug"),
                string_field(item, "reason")
            )
        })
        .collect::<Vec<_>>();
    for failure in failures {
        if failure
            .get("resolved_by_success")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let outcome = match normalized_failure_kind(failure).as_str() {
            "configuration" => "configuration error".to_string(),
            "timeout" => "timed out".to_string(),
            _ if failure
                .get("timed_out")
                .and_then(Value::as_bool)
                .unwrap_or(false) =>
            {
                "timed out".to_string()
            }
            _ => format!("exited {}", number_field(failure, "exit_code") as u64),
        };
        reasons.push(format!(
            "failed run {} ({}/{} {}): {outcome}",
            string_field(failure, "slug"),
            string_field(failure, "matrix"),
            string_field(failure, "profile"),
            string_field(failure, "phase"),
        ));
    }
    for entry in coverage_summaries(runs, coverage_targets) {
        let missing = number_field(&entry, "missing_case_count") as usize;
        let covered = number_field(&entry, "covered_case_count") as usize;
        if missing > 0 && covered > 0 {
            let missing_cases = array_strings(&entry, "missing_cases").join(", ");
            reasons.push(format!(
                "coverage {}/{} missing {missing} case(s): {missing_cases}",
                string_field(&entry, "matrix"),
                string_field(&entry, "profile")
            ));
        }
    }
    if runs.is_empty() {
        reasons.push("no retained benchmark profile runs".to_string());
        return json!({ "status": "warn", "reasons": reasons });
    }
    let latest = &runs[0];
    if latest
        .get("case_count")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        <= 0
    {
        reasons.push(format!(
            "latest run {} has no benchmark cases",
            string_field(latest, "slug")
        ));
    }
    if number_field(latest, "total_median_ms") <= 0.0 {
        reasons.push(format!(
            "latest run {} has invalid total median time",
            string_field(latest, "slug")
        ));
    }
    if number_field(latest, "peak_rss_mib") <= 0.0 {
        reasons.push(format!(
            "latest run {} has invalid peak RSS",
            string_field(latest, "slug")
        ));
    }
    json!({ "status": if reasons.is_empty() { "pass" } else { "warn" }, "reasons": reasons })
}

fn observed_case_ids(run: &Value) -> Vec<String> {
    let case_ids = array_strings(run, "case_ids");
    if case_ids.is_empty() {
        vec![normalize_case_id(&string_field(run, "slowest_case"))]
    } else {
        case_ids
            .iter()
            .map(|case| normalize_case_id(case))
            .collect()
    }
}

fn normalize_case_id(case_id: &str) -> String {
    case_id.split('#').next().unwrap_or(case_id).to_string()
}

fn benchmark_case_key(matrix: &str, profile: &str, case: &str) -> String {
    format!(
        "{}|{}|{}",
        matrix,
        normalized_profile(profile),
        normalize_case_id(case)
    )
}

fn normalized_profile(profile: &str) -> &str {
    match profile {
        "1m" | "1000k" => "one_million",
        "500k" => "five_hundred_k",
        "400k" => "four_hundred_k",
        "300k" => "three_hundred_k",
        "200k" => "two_hundred_k",
        "100k" => "hundred_k",
        "20k" => "twenty_k",
        "15k" => "fifteen_k",
        "10k" => "ten_k",
        other => other,
    }
}

fn normalized_failure_kind(failure: &Value) -> String {
    let configured = string_field(failure, "failure_kind");
    if !configured.is_empty() {
        return configured;
    }
    if failure
        .get("timed_out")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "timeout".to_string()
    } else {
        "execution".to_string()
    }
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    fs::write(
        path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(value)
                .map_err(|error| format!("failed to encode {}: {error}", path.display()))?
        ),
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn display_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn non_empty_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn number_field(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn array_strings(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
#[path = "benchmark_profile_index/tests.rs"]
mod tests;

fn unix_seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
