use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_compare_workflow_catalog_benchmark(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(repo_root, args)?;
    let baseline = read_json(&options.baseline)?;
    let current = read_json(&options.current)?;
    let comparison = build_comparison(&baseline, &current, &options);

    if let Some(json_out) = &options.json_out {
        write_json_no_trailing_newline(json_out, &comparison)?;
    }
    if let Some(report_out) = &options.report_out {
        write_text(report_out, &render_report(&comparison))?;
    }

    let content = serde_json::to_string_pretty(&comparison)
        .map_err(|error| format!("failed to encode comparison: {error}"))?;
    println!("{content}");
    Ok(if comparison["ok"].as_bool().unwrap_or(false) {
        0
    } else {
        1
    })
}

struct Options {
    baseline: PathBuf,
    baseline_label: String,
    current: PathBuf,
    current_label: String,
    fail_on_avg_regression_pct: f64,
    fail_on_median_regression_pct: f64,
    json_out: Option<PathBuf>,
    report_out: Option<PathBuf>,
}

#[derive(Clone)]
struct NormalizedSummary {
    case_count: usize,
    cases: Vec<Value>,
    generated_at: Value,
    id: String,
    repeat: Value,
}

fn parse_args(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut baseline = None;
    let mut current = None;
    let mut json_out = None;
    let mut report_out = None;
    let mut fail_on_avg_regression_pct = f64::INFINITY;
    let mut fail_on_median_regression_pct = f64::INFINITY;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                print_usage();
                return Err(
                    "compare-workflow-catalog-benchmark requires --current and --baseline".into(),
                );
            }
            "--current" => current = Some(path_arg(repo_root, &mut iter, "--current")?),
            "--baseline" => baseline = Some(path_arg(repo_root, &mut iter, "--baseline")?),
            "--json-out" => json_out = Some(path_arg(repo_root, &mut iter, "--json-out")?.0),
            "--report-out" => report_out = Some(path_arg(repo_root, &mut iter, "--report-out")?.0),
            "--fail-on-median-regression-pct" => {
                fail_on_median_regression_pct =
                    number_arg(&mut iter, "--fail-on-median-regression-pct")?
            }
            "--fail-on-avg-regression-pct" => {
                fail_on_avg_regression_pct = number_arg(&mut iter, "--fail-on-avg-regression-pct")?
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }
    let (baseline, baseline_label) =
        baseline.ok_or_else(|| "--current and --baseline are required".to_string())?;
    let (current, current_label) =
        current.ok_or_else(|| "--current and --baseline are required".to_string())?;
    Ok(Options {
        baseline,
        baseline_label,
        current,
        current_label,
        fail_on_avg_regression_pct,
        fail_on_median_regression_pct,
        json_out,
        report_out,
    })
}

fn build_comparison(baseline: &Value, current: &Value, options: &Options) -> Value {
    let baseline = normalize_summary(baseline);
    let current = normalize_summary(current);
    let baseline_cases = index_cases(&baseline);
    let current_cases = index_cases(&current);
    let baseline_case_ids = sorted_keys(&baseline_cases);
    let current_case_ids = sorted_keys(&current_cases);
    let missing_cases = baseline_case_ids
        .iter()
        .filter(|case_id| !current_cases.contains_key(*case_id))
        .cloned()
        .collect::<Vec<_>>();
    let added_cases = current_case_ids
        .iter()
        .filter(|case_id| !baseline_cases.contains_key(*case_id))
        .cloned()
        .collect::<Vec<_>>();
    let cases = baseline_case_ids
        .iter()
        .filter_map(|case_id| {
            Some(compare_case(
                baseline_cases.get(case_id)?,
                current_cases.get(case_id)?,
            ))
        })
        .collect::<Vec<_>>();
    let failures = build_failures(&missing_cases, &cases, options);

    json!({
        "generated_at": crate::native_time::utc_iso_timestamp(),
        "baseline_path": options.baseline_label,
        "current_path": options.current_label,
        "baseline_id": baseline.id,
        "baseline_generated_at": baseline.generated_at,
        "current_generated_at": current.generated_at,
        "baseline_repeat": baseline.repeat,
        "current_repeat": current.repeat,
        "baseline_case_count": baseline.case_count,
        "current_case_count": current.case_count,
        "missing_cases": missing_cases,
        "added_cases": added_cases,
        "cases": cases,
        "failures": failures,
        "ok": failures.is_empty(),
    })
}

fn normalize_summary(summary: &Value) -> NormalizedSummary {
    let cases = summary["cases"].as_array().cloned().unwrap_or_default();
    let repeat = if !summary["repeat"].is_null() {
        summary["repeat"].clone()
    } else if !summary
        .pointer("/source/repeat")
        .unwrap_or(&Value::Null)
        .is_null()
    {
        summary.pointer("/source/repeat").unwrap().clone()
    } else {
        Value::Null
    };
    NormalizedSummary {
        case_count: cases.len(),
        cases,
        generated_at: summary.get("generated_at").cloned().unwrap_or(Value::Null),
        id: summary["id"]
            .as_str()
            .unwrap_or("workflow-catalog-benchmark")
            .to_string(),
        repeat,
    }
}

fn index_cases(summary: &NormalizedSummary) -> BTreeMap<String, Value> {
    summary
        .cases
        .iter()
        .filter_map(|entry| Some((entry["case_id"].as_str()?.to_string(), entry.clone())))
        .collect()
}

fn compare_case(baseline_case: &Value, current_case: &Value) -> Value {
    let baseline_summary = baseline_case.get("summary").unwrap_or(&Value::Null);
    let current_summary = current_case.get("summary").unwrap_or(&Value::Null);
    let metrics = vec![
        compare_metric(
            "median_elapsed_ms",
            "ms",
            number_at(baseline_summary, "/median_elapsed_ms"),
            number_at(current_summary, "/median_elapsed_ms"),
        ),
        compare_metric(
            "avg_elapsed_ms",
            "ms",
            number_at(baseline_summary, "/avg_elapsed_ms"),
            number_at(current_summary, "/avg_elapsed_ms"),
        ),
        compare_metric(
            "min_elapsed_ms",
            "ms",
            number_at(baseline_summary, "/min_elapsed_ms"),
            number_at(current_summary, "/min_elapsed_ms"),
        ),
        compare_metric(
            "max_elapsed_ms",
            "ms",
            number_at(baseline_summary, "/max_elapsed_ms"),
            number_at(current_summary, "/max_elapsed_ms"),
        ),
    ];
    let baseline_range = baseline_summary["completed_node_count_range"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let current_range = current_summary["completed_node_count_range"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    json!({
        "case_id": baseline_case["case_id"].as_str().unwrap_or(""),
        "workflow_id": current_case["workflow_id"].as_str().or_else(|| baseline_case["workflow_id"].as_str()).unwrap_or(""),
        "baseline_repeat": baseline_case.get("repeat").cloned().unwrap_or(Value::Null),
        "current_repeat": current_case.get("repeat").cloned().unwrap_or(Value::Null),
        "metrics": metrics,
        "completed_node_count_range_match": baseline_range == current_range,
        "baseline_completed_node_count_range": baseline_range,
        "current_completed_node_count_range": current_range,
    })
}

fn compare_metric(name: &str, unit: &str, baseline: f64, current: f64) -> Value {
    let delta = current - baseline;
    let delta_pct = safe_percent_delta(current, baseline);
    json!({
        "name": name,
        "unit": unit,
        "baseline": round(baseline, 3),
        "current": round(current, 3),
        "delta": round(delta, 3),
        "delta_pct": delta_pct.map(|value| round(value, 3)),
        "direction": if delta > 0.0 { "up" } else if delta < 0.0 { "down" } else { "flat" },
    })
}

fn build_failures(missing_cases: &[String], cases: &[Value], options: &Options) -> Vec<String> {
    let mut failures = Vec::new();
    if !missing_cases.is_empty() {
        failures.push(format!(
            "missing benchmark cases: {}",
            missing_cases.join(", ")
        ));
    }
    for entry in cases {
        if entry["completed_node_count_range_match"].as_bool() != Some(true) {
            failures.push(format!(
                "{} completed node count range drifted ({} vs {})",
                str_at(entry, "/case_id"),
                joined_range(&entry["baseline_completed_node_count_range"]),
                joined_range(&entry["current_completed_node_count_range"])
            ));
        }
        push_threshold_failure(
            &mut failures,
            entry,
            "median_elapsed_ms",
            "median",
            options.fail_on_median_regression_pct,
        );
        push_threshold_failure(
            &mut failures,
            entry,
            "avg_elapsed_ms",
            "average",
            options.fail_on_avg_regression_pct,
        );
    }
    failures
}

fn push_threshold_failure(
    failures: &mut Vec<String>,
    entry: &Value,
    metric_name: &str,
    label: &str,
    threshold: f64,
) {
    let Some(metric) = entry["metrics"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|metric| metric["name"].as_str() == Some(metric_name))
    else {
        return;
    };
    let Some(delta_pct) = metric["delta_pct"].as_f64() else {
        return;
    };
    if delta_pct > threshold {
        failures.push(format!(
            "{} {label} regression {delta_pct}% exceeded {threshold}%",
            str_at(entry, "/case_id")
        ));
    }
}

fn render_report(comparison: &Value) -> String {
    let status = if array_len(comparison.pointer("/failures")) == 0 {
        "- Status: pass\n".to_string()
    } else {
        format!(
            "{}\n",
            comparison["failures"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|failure| format!("- {failure}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    format!(
        "# Workflow catalog benchmark comparison\n\n\
- Baseline: `{}`\n\
- Current: `{}`\n\
- Baseline repeat: `{}`\n\
- Current repeat: `{}`\n\
- Baseline cases: `{}`\n\
- Current cases: `{}`\n\n\
## Status\n\n\
{}
## Coverage\n\n\
{}\n\
{}\n\n\
## Case comparisons\n\n\
{}\n",
        str_at(comparison, "/baseline_path"),
        str_at(comparison, "/current_path"),
        value_text(&comparison["baseline_repeat"]),
        value_text(&comparison["current_repeat"]),
        value_text(&comparison["baseline_case_count"]),
        value_text(&comparison["current_case_count"]),
        status,
        coverage_line("Missing baseline cases", &comparison["missing_cases"]),
        coverage_line("Added current cases", &comparison["added_cases"]),
        comparison["cases"]
            .as_array()
            .into_iter()
            .flatten()
            .map(render_case_section)
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn render_case_section(entry: &Value) -> String {
    let range_status = if entry["completed_node_count_range_match"].as_bool() == Some(true) {
        "match".to_string()
    } else {
        format!(
            "drift ({} vs {})",
            joined_range(&entry["baseline_completed_node_count_range"]),
            joined_range(&entry["current_completed_node_count_range"])
        )
    };
    format!(
        "### {}\n\n\
- Workflow: `{}`\n\
- Completed node range: {}\n\
- Baseline repeat: `{}`\n\
- Current repeat: `{}`\n\n\
| Metric | Baseline | Current | Delta | Delta % |\n\
| --- | ---: | ---: | ---: | ---: |\n\
{}\n",
        str_at(entry, "/case_id"),
        str_at(entry, "/workflow_id"),
        range_status,
        value_text(&entry["baseline_repeat"]),
        value_text(&entry["current_repeat"]),
        metric_rows(&entry["metrics"])
    )
}

fn metric_rows(metrics: &Value) -> String {
    metrics
        .as_array()
        .into_iter()
        .flatten()
        .map(|metric| {
            let delta_pct = metric["delta_pct"]
                .as_f64()
                .map(|value| format!("{}%", markdown_number(value)))
                .unwrap_or_else(|| "n/a".to_string());
            format!(
                "| {} | {} {} | {} {} | {} {} | {} |",
                str_at(metric, "/name"),
                markdown_value_text(&metric["baseline"]),
                str_at(metric, "/unit"),
                markdown_value_text(&metric["current"]),
                str_at(metric, "/unit"),
                markdown_value_text(&metric["delta"]),
                str_at(metric, "/unit"),
                delta_pct
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn coverage_line(label: &str, cases: &Value) -> String {
    let items = cases
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    if items.is_empty() {
        format!("- {label}: none")
    } else {
        format!("- {label}: {}", items.join(", "))
    }
}

fn path_arg(
    repo_root: &Path,
    iter: &mut impl Iterator<Item = OsString>,
    flag: &str,
) -> RunnerResult<(PathBuf, String)> {
    let raw = iter
        .next()
        .ok_or_else(|| format!("unknown option: {flag}"))?
        .to_string_lossy()
        .into_owned();
    let value = PathBuf::from(&raw);
    let path = if value.is_absolute() {
        value
    } else {
        repo_root.join(value)
    };
    Ok((path, raw))
}

fn number_arg(iter: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<f64> {
    iter.next()
        .ok_or_else(|| format!("unknown option: {flag}"))?
        .to_string_lossy()
        .parse::<f64>()
        .map_err(|error| format!("invalid number for {flag}: {error}"))
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_json_no_trailing_newline(path: &Path, value: &Value) -> RunnerResult<()> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {}: {error}", path.display()))?;
    write_text(path, &content)
}

fn write_text(path: &Path, content: &str) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(path, content).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn sorted_keys(map: &BTreeMap<String, Value>) -> Vec<String> {
    map.keys().cloned().collect()
}

fn safe_percent_delta(current: f64, baseline: f64) -> Option<f64> {
    if baseline == 0.0 {
        if current == 0.0 { Some(0.0) } else { None }
    } else {
        Some(((current - baseline) / baseline) * 100.0)
    }
}

fn number_at(value: &Value, pointer: &str) -> f64 {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn str_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value.pointer(pointer).and_then(Value::as_str).unwrap_or("")
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

fn round(value: f64, digits: i32) -> f64 {
    let factor = 10_f64.powi(digits);
    (value * factor).round() / factor
}

fn joined_range(value: &Value) -> String {
    value
        .as_array()
        .into_iter()
        .flatten()
        .map(value_text)
        .collect::<Vec<_>>()
        .join("..")
}

fn value_text(value: &Value) -> String {
    match value {
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

fn markdown_value_text(value: &Value) -> String {
    if let Some(number) = value.as_f64() {
        if number.fract() == 0.0 {
            return format!("{}", number as i64);
        }
    }
    value_text(value)
}

fn markdown_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki compare-workflow-catalog-benchmark [options]\n\n\
Options:\n  --current <path>                    Current workflow catalog benchmark JSON.\n  \
--baseline <path>                   Baseline workflow catalog benchmark JSON.\n  \
--json-out <path>                   Optional compare JSON output path.\n  \
--report-out <path>                 Optional Markdown report output path.\n  \
--fail-on-median-regression-pct <n> Fail when any case median regresses by more than n percent.\n  \
--fail-on-avg-regression-pct <n>    Fail when any case average regresses by more than n percent.\n  \
--help                              Show this message."
    );
}
