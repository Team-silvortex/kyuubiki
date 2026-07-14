use crate::native_time::utc_iso_timestamp;
use analysis::{
    latest_preconditioner_comparisons_for_runs, optimization_targets, preconditioner_economics,
    solver_tuning_notes, sparse_matvec_throughput, summarize_stage_rows,
};
use markdown::write_markdown;
use self_test::run_self_test;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod analysis;
mod markdown;
mod self_test;

const DEFAULT_DIR: &str = "tmp/remote-material-research";
const SCHEMA_VERSION: &str = "kyuubiki.remote-material-benchmark-summary/v1";

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_build_remote_material_benchmark_summary(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(root, args)?;
    let result = if options.self_test {
        run_self_test()
    } else {
        build_summary_files(&options)
    };
    match result {
        Ok(message) => {
            println!("{message}");
            Ok(0)
        }
        Err(issue) => {
            eprintln!("remote material benchmark summary failed: {issue}");
            Ok(1)
        }
    }
}

struct Options {
    input_dir: PathBuf,
    json_out: PathBuf,
    markdown_out: PathBuf,
    self_test: bool,
}

#[derive(Clone)]
pub(super) struct Run {
    benchmark: Value,
    name: String,
    research: Option<Value>,
}

fn parse_args(root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let default = root.join(DEFAULT_DIR);
    let mut options = Options {
        input_dir: default.clone(),
        json_out: default.join("summary.json"),
        markdown_out: default.join("README.md"),
        self_test: false,
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner build-remote-material-benchmark-summary [--self-test] [--input-dir tmp/remote-material-research] [--json-out tmp/remote-material-research/summary.json] [--markdown-out tmp/remote-material-research/README.md]"
                );
                return Ok(options);
            }
            "--self-test" => options.self_test = true,
            "--input-dir" => {
                options.input_dir = root.join(required_value(&mut iter, "--input-dir")?)
            }
            "--json-out" => options.json_out = root.join(required_value(&mut iter, "--json-out")?),
            "--markdown-out" => {
                options.markdown_out = root.join(required_value(&mut iter, "--markdown-out")?)
            }
            other => return Err(format!("unknown or incomplete argument: {other}")),
        }
    }
    Ok(options)
}

fn build_summary_files(options: &Options) -> RunnerResult<String> {
    let runs = read_runs(&options.input_dir)?;
    let summary = build_summary(&runs);
    write_json(&options.json_out, &summary)?;
    write_markdown(&options.markdown_out, &summary)?;
    Ok(format!(
        "remote material benchmark summary: {}",
        options.markdown_out.display()
    ))
}

fn read_runs(input_dir: &Path) -> RunnerResult<Vec<Run>> {
    if !input_dir.exists() {
        return Ok(Vec::new());
    }
    let mut runs = Vec::new();
    let mut names = fs::read_dir(input_dir)
        .map_err(|error| format!("failed to read {}: {error}", input_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to list {}: {error}", input_dir.display()))?;
    names.sort_by_key(|entry| entry.file_name());
    for entry in names {
        let dir = entry.path();
        let benchmark_path = dir.join("remote-material-research-benchmark.json");
        if !dir.is_dir() || !benchmark_path.exists() {
            continue;
        }
        let research_path = dir.join("material-research-example.json");
        runs.push(Run {
            benchmark: read_json(&benchmark_path)?,
            name: entry.file_name().to_string_lossy().to_string(),
            research: research_path
                .exists()
                .then(|| read_json(&research_path))
                .transpose()?,
        });
    }
    Ok(runs)
}

fn build_summary(runs: &[Run]) -> Value {
    let mut sorted_runs = runs.to_vec();
    sorted_runs.sort_by(|left, right| left.name.cmp(&right.name));
    let cases = case_rows(runs);
    let latest_cases = latest_case_results(&sorted_runs);
    let best_cases = best_case_results(&cases);
    let stage_hotspots = stage_rows_for_runs(runs);
    let latest_stage_hotspots = latest_stage_rows(&sorted_runs, &latest_cases);
    let latest_stage_summary = summarize_stage_rows(&latest_stage_hotspots);
    let stage_summary = summarize_stage_rows(&stage_hotspots);
    let latest_preconditioner_comparisons =
        latest_preconditioner_comparisons_for_runs(&sorted_runs, &latest_cases);
    let latest_preconditioner_economics =
        preconditioner_economics(&latest_preconditioner_comparisons, &latest_stage_hotspots);
    json!({
        "schema_version": SCHEMA_VERSION,
        "generated_at_utc": utc_iso_timestamp(),
        "run_count": runs.len(),
        "case_count": cases.len(),
        "best_cases": take(best_cases, 12),
        "failed_cases": cases.iter().filter(|item| !bool_field(item, "ok")).cloned().collect::<Vec<_>>(),
        "hottest_cases": take(sort_by_number_desc(cases.clone(), "median_ms"), 8),
        "latest_cases": latest_cases,
        "latest_optimization_targets": take(optimization_targets(&latest_stage_summary), 8),
        "latest_preconditioner_economics": latest_preconditioner_economics,
        "latest_preconditioner_comparisons": latest_preconditioner_comparisons,
        "latest_solver_tuning_notes": solver_tuning_notes(&latest_preconditioner_economics),
        "latest_sparse_matvec_throughput": sparse_matvec_throughput(&latest_stage_summary),
        "latest_stage_summary": latest_stage_summary,
        "latest_stage_hotspots": take(latest_stage_hotspots, 12),
        "memory_heaviest_cases": take(sort_by_number_desc(cases.clone(), "peak_rss_mib"), 8),
        "stage_summary": stage_summary,
        "stage_hotspots": take(stage_hotspots, 12),
        "runs": sorted_runs.iter().map(run_summary).collect::<Vec<_>>(),
    })
}

fn case_rows(runs: &[Run]) -> Vec<Value> {
    runs.iter()
        .flat_map(|run| {
            benchmark_cases(run)
                .into_iter()
                .map(|case| case_row(run, case))
        })
        .collect()
}

fn case_row(run: &Run, item: &Value) -> Value {
    json!({
        "case_id": field(item, "id"),
        "dof_count": item.get("dof_count").cloned().unwrap_or(Value::Null),
        "matrix": field(&run.benchmark, "matrix"),
        "median_ms": item.get("median_ms").cloned().unwrap_or(Value::Null),
        "ok": item.get("ok").cloned().unwrap_or(Value::Bool(false)),
        "peak_rss_mib": item.get("peak_rss_kib").and_then(Value::as_f64).map(|value| json!(value / 1024.0)).unwrap_or(Value::Null),
        "profile": field(&run.benchmark, "profile"),
        "residual_norm": item.get("solver_residual_norm").cloned().unwrap_or(Value::Null),
        "run": run.name,
        "solver_iterations": item.get("solver_iterations").cloned().unwrap_or(Value::Null),
        "solver_matrix_non_zero_count": item.get("solver_matrix_non_zero_count").cloned().unwrap_or(Value::Null),
        "solver_preconditioner": item.get("solver_preconditioner").and_then(Value::as_str).map(str::to_string).or_else(|| preconditioner_from_case_id(field(item, "id"))).map(Value::String).unwrap_or(Value::Null),
    })
}

fn latest_case_results(sorted_runs: &[Run]) -> Vec<Value> {
    let mut latest = BTreeMap::<String, Value>::new();
    for run in sorted_runs {
        for case in benchmark_cases(run) {
            let row = case_row(run, case);
            latest.insert(case_key(&row), row);
        }
    }
    let mut rows = latest.into_values().collect::<Vec<_>>();
    rows.sort_by(compare_case_rows);
    rows
}

fn latest_stage_rows(sorted_runs: &[Run], latest_cases: &[Value]) -> Vec<Value> {
    let latest_keys = latest_cases
        .iter()
        .map(|item| {
            format!(
                "{}/{}/{}/{}",
                field(item, "run"),
                field(item, "matrix"),
                field(item, "profile"),
                field(item, "case_id")
            )
        })
        .collect::<std::collections::HashSet<_>>();
    stage_rows_for_runs(sorted_runs)
        .into_iter()
        .filter(|item| {
            latest_keys.contains(&format!(
                "{}/{}/{}/{}",
                field(item, "run"),
                field(item, "matrix"),
                field(item, "profile"),
                field(item, "case_id")
            ))
        })
        .collect()
}

fn stage_rows_for_runs(runs: &[Run]) -> Vec<Value> {
    let mut rows = Vec::new();
    for run in runs {
        for case in benchmark_cases(run) {
            for stage in case
                .get("memory_stages")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if !finite(stage, "elapsed_ms")
                    .is_some_and(|_| is_hotspot_stage(field(stage, "label")))
                {
                    continue;
                }
                let median = finite(case, "median_ms").unwrap_or(0.0);
                rows.push(json!({
                    "case_id": field(case, "id"),
                    "elapsed_ms": finite(stage, "elapsed_ms").unwrap_or(0.0),
                    "matrix": field(&run.benchmark, "matrix"),
                    "profile": field(&run.benchmark, "profile"),
                    "run": run.name,
                    "solver_iterations": case.get("solver_iterations").cloned().unwrap_or(Value::Null),
                    "solver_matrix_non_zero_count": case.get("solver_matrix_non_zero_count").cloned().unwrap_or(Value::Null),
                    "stage": field(stage, "label"),
                    "stage_share_pct": if median > 0.0 { json!(finite(stage, "elapsed_ms").unwrap_or(0.0) / median * 100.0) } else { Value::Null },
                    "stage_rss_mib": stage.get("rss_kib").and_then(Value::as_f64).map(|value| json!(value / 1024.0)).unwrap_or(Value::Null),
                }));
            }
        }
    }
    sort_by_number_desc(rows, "elapsed_ms")
}

pub(super) fn benchmark_cases(run: &Run) -> Vec<&Value> {
    run.benchmark
        .get("cases")
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn run_summary(run: &Run) -> Value {
    json!({
        "matrix": field(&run.benchmark, "matrix"),
        "name": run.name,
        "profile": field(&run.benchmark, "profile"),
        "repeat": run.benchmark.get("repeat").cloned().unwrap_or(Value::Null),
        "winner_candidate_id": run.research.as_ref().and_then(|research| research.pointer("/exploration/report/winner_candidate_id")).cloned().unwrap_or(Value::Null),
    })
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode summary: {error}"))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn required_value(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    iter.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("unknown or incomplete argument: {flag}"))
}

fn compare_case_rows(left: &Value, right: &Value) -> std::cmp::Ordering {
    field(left, "matrix")
        .cmp(field(right, "matrix"))
        .then_with(|| field(left, "profile").cmp(field(right, "profile")))
        .then_with(|| field(left, "case_id").cmp(field(right, "case_id")))
}

pub(super) fn sort_by_number_desc(mut rows: Vec<Value>, key: &str) -> Vec<Value> {
    rows.sort_by(|left, right| {
        finite(right, key)
            .partial_cmp(&finite(left, key))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn best_case_results(cases: &[Value]) -> Vec<Value> {
    let mut best = BTreeMap::<String, Value>::new();
    for item in cases.iter().filter(|candidate| bool_field(candidate, "ok")) {
        let key = case_key(item);
        if best
            .get(&key)
            .is_none_or(|current| finite(item, "median_ms") < finite(current, "median_ms"))
        {
            best.insert(key, item.clone());
        }
    }
    sort_by_number_desc(best.into_values().collect(), "median_ms")
}

fn take(rows: Vec<Value>, limit: usize) -> Vec<Value> {
    rows.into_iter().take(limit).collect()
}

fn is_hotspot_stage(stage: &str) -> bool {
    !matches!(stage, "solve_system" | "solve_spd_system")
}

pub(super) fn base_case_id(case_id: &str) -> &str {
    case_id.split('#').next().unwrap_or(case_id)
}

fn preconditioner_from_case_id(case_id: &str) -> Option<String> {
    case_id
        .split_once('#')
        .map(|(_, preconditioner)| preconditioner.to_string())
}

pub(super) fn case_key(item: &Value) -> String {
    format!(
        "{}/{}/{}",
        field(item, "matrix"),
        field(item, "profile"),
        field(item, "case_id")
    )
}

pub(super) fn ratio(numerator: Option<f64>, denominator: Option<f64>) -> Option<f64> {
    let (numerator, denominator) = numerator.zip(denominator)?;
    (denominator > 0.0).then_some(numerator / denominator)
}

pub(super) fn opt_num(value: Option<f64>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

pub(super) fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

pub(super) fn bool_field(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

pub(super) fn finite(value: &Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

pub(super) fn finite_opt(value: Option<&Value>, key: &str) -> Option<f64> {
    value.and_then(|item| finite(item, key))
}

pub(super) fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
