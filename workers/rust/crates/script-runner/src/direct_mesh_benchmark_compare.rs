use serde_json::{Map, Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_compare_direct_mesh_benchmark(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(repo_root, args)?;
    let baseline = read_json(&options.baseline)?;
    let current = read_json(&options.current)?;
    let comparison = build_comparison(&baseline, &current, &options);

    if let Some(json_out) = &options.json_out {
        write_json(json_out, &comparison)?;
    }
    if let Some(report_out) = &options.report_out {
        write_text(report_out, &render_report(&comparison))?;
    }

    let ok = comparison["ok"].as_bool().unwrap_or(false);
    if options.json_out.is_none() && options.report_out.is_none() {
        let content = serde_json::to_string_pretty(&comparison)
            .map_err(|error| format!("failed to encode comparison: {error}"))?;
        println!("{content}");
    } else {
        println!(
            "{}",
            if ok {
                "comparison ok"
            } else {
                "comparison failed"
            }
        );
    }
    Ok(if ok { 0 } else { 1 })
}

struct Options {
    baseline: PathBuf,
    baseline_label: String,
    current: PathBuf,
    current_label: String,
    fail_on_elapsed_regression_pct: f64,
    fail_on_rss_regression_pct: f64,
    json_out: Option<PathBuf>,
    report_out: Option<PathBuf>,
}

fn parse_args(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut baseline: Option<(PathBuf, String)> = None;
    let mut current: Option<(PathBuf, String)> = None;
    let mut json_out = None;
    let mut report_out = None;
    let mut fail_on_elapsed_regression_pct = f64::INFINITY;
    let mut fail_on_rss_regression_pct = f64::INFINITY;
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                print_usage();
                return Err(
                    "compare-direct-mesh-benchmark requires --current and --baseline".into(),
                );
            }
            "--current" => current = Some(path_arg(repo_root, &mut iter, "--current")?),
            "--baseline" => baseline = Some(path_arg(repo_root, &mut iter, "--baseline")?),
            "--json-out" => {
                json_out = Some(path_arg(repo_root, &mut iter, "--json-out")?.0);
            }
            "--report-out" => {
                report_out = Some(path_arg(repo_root, &mut iter, "--report-out")?.0);
            }
            "--fail-on-elapsed-regression-pct" => {
                fail_on_elapsed_regression_pct =
                    number_arg(&mut iter, "--fail-on-elapsed-regression-pct")?
            }
            "--fail-on-rss-regression-pct" => {
                fail_on_rss_regression_pct = number_arg(&mut iter, "--fail-on-rss-regression-pct")?
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
        fail_on_elapsed_regression_pct,
        fail_on_rss_regression_pct,
        json_out,
        report_out,
    })
}

fn build_comparison(baseline: &Value, current: &Value, options: &Options) -> Value {
    let baseline_subtests = index_subtest_means(baseline);
    let current_subtests = index_subtest_means(current);
    let mut subtests = Vec::new();
    for (name, baseline_stats) in &baseline_subtests {
        if let Some((_, current_stats)) = current_subtests
            .iter()
            .find(|(current_name, _)| current_name == name)
        {
            subtests.push(compare_metric(
                name,
                "ms",
                baseline_stats.mean_duration_ms,
                current_stats.mean_duration_ms,
            ));
        }
    }
    let metrics = vec![
        compare_metric(
            "elapsed_mean",
            "s",
            number_at(baseline, "/aggregate/elapsed_s/mean"),
            number_at(current, "/aggregate/elapsed_s/mean"),
        ),
        compare_metric(
            "rss_mean",
            "KiB",
            number_at(baseline, "/aggregate/max_rss_kib/mean"),
            number_at(current, "/aggregate/max_rss_kib/mean"),
        ),
        compare_metric(
            "user_cpu_mean",
            "s",
            number_at(baseline, "/aggregate/user_s/mean"),
            number_at(current, "/aggregate/user_s/mean"),
        ),
        compare_metric(
            "sys_cpu_mean",
            "s",
            number_at(baseline, "/aggregate/sys_s/mean"),
            number_at(current, "/aggregate/sys_s/mean"),
        ),
    ];
    let mut failures = Vec::new();
    push_threshold_failure(
        &mut failures,
        &metrics,
        "elapsed_mean",
        "elapsed mean regression",
        options.fail_on_elapsed_regression_pct,
    );
    push_threshold_failure(
        &mut failures,
        &metrics,
        "rss_mean",
        "rss mean regression",
        options.fail_on_rss_regression_pct,
    );
    let ok = failures.is_empty();
    let mut comparison = Map::new();
    comparison.insert(
        "generated_at".to_string(),
        json!(crate::native_time::utc_iso_timestamp()),
    );
    comparison.insert("baseline_path".to_string(), json!(options.baseline_label));
    comparison.insert("current_path".to_string(), json!(options.current_label));
    comparison.insert(
        "baseline_id".to_string(),
        json!(
            baseline["id"]
                .as_str()
                .unwrap_or("direct-mesh-docker-baseline")
        ),
    );
    if !current["repeat"].is_null() {
        comparison.insert("current_repeat".to_string(), current["repeat"].clone());
    }
    if !baseline["repeat"].is_null() {
        comparison.insert("baseline_repeat".to_string(), baseline["repeat"].clone());
    }
    comparison.insert("metrics".to_string(), json!(metrics));
    comparison.insert("subtests".to_string(), json!(subtests));
    comparison.insert("failures".to_string(), json!(failures));
    comparison.insert("ok".to_string(), json!(ok));
    Value::Object(comparison)
}

#[derive(Clone, Copy)]
struct SubtestStats {
    mean_duration_ms: f64,
}

fn index_subtest_means(summary: &Value) -> Vec<(String, SubtestStats)> {
    let mut durations: Vec<(String, Vec<f64>)> = Vec::new();
    if let Some(runs) = summary["runs"].as_array() {
        for run in runs {
            if let Some(subtests) = run["subtests"].as_array() {
                for subtest in subtests {
                    let name = subtest["name"].as_str().unwrap_or("").to_string();
                    if let Some((_, values)) = durations.iter_mut().find(|(item, _)| item == &name)
                    {
                        values.push(number_at(subtest, "/duration_ms"));
                    } else {
                        durations.push((name, vec![number_at(subtest, "/duration_ms")]));
                    }
                }
            }
        }
    }
    durations
        .into_iter()
        .map(|(name, values)| {
            let mean_duration_ms = if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            };
            (name, SubtestStats { mean_duration_ms })
        })
        .collect()
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

fn push_threshold_failure(
    failures: &mut Vec<String>,
    metrics: &[Value],
    metric_name: &str,
    label: &str,
    threshold: f64,
) {
    let Some(metric) = metrics
        .iter()
        .find(|metric| metric["name"].as_str() == Some(metric_name))
    else {
        return;
    };
    let Some(delta_pct) = metric["delta_pct"].as_f64() else {
        return;
    };
    if delta_pct > threshold {
        failures.push(format!("{label} {delta_pct}% exceeded {threshold}%"));
    }
}

fn render_report(comparison: &Value) -> String {
    let failure_section = if array_len(comparison.pointer("/failures")) == 0 {
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
        "# Direct-mesh Docker benchmark comparison\n\n\
- Baseline: `{}`\n\
- Current: `{}`\n\
- Baseline repeat: `{}`\n\
- Current repeat: `{}`\n\n\
## Status\n\n\
{}
## Aggregate metrics\n\n\
| Metric | Baseline | Current | Delta | Delta % |\n\
| --- | ---: | ---: | ---: | ---: |\n\
{}\n\n\
## Subtest means\n\n\
| Subtest | Baseline | Current | Delta | Delta % |\n\
| --- | ---: | ---: | ---: | ---: |\n\
{}\n",
        str_at(comparison, "/baseline_path"),
        str_at(comparison, "/current_path"),
        optional_value_text(comparison.get("baseline_repeat")),
        optional_value_text(comparison.get("current_repeat")),
        failure_section,
        markdown_table_rows(&comparison["metrics"], false),
        markdown_table_rows(&comparison["subtests"], true),
    )
}

fn markdown_table_rows(metrics: &Value, force_ms: bool) -> String {
    metrics
        .as_array()
        .into_iter()
        .flatten()
        .map(|metric| {
            let unit = if force_ms {
                "ms"
            } else {
                str_at(metric, "/unit")
            };
            let delta_pct = metric["delta_pct"]
                .as_f64()
                .map(|value| format!("{value}%"))
                .unwrap_or_else(|| "n/a".to_string());
            format!(
                "| {} | {} {} | {} {} | {} {} | {} |",
                str_at(metric, "/name"),
                markdown_value_text(&metric["baseline"]),
                unit,
                markdown_value_text(&metric["current"]),
                unit,
                markdown_value_text(&metric["delta"]),
                unit,
                delta_pct
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn safe_percent_delta(current: f64, baseline: f64) -> Option<f64> {
    if baseline == 0.0 {
        if current == 0.0 { Some(0.0) } else { None }
    } else {
        Some(((current - baseline) / baseline) * 100.0)
    }
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to encode {}: {error}", path.display()))?;
    write_text(path, &format!("{content}\n"))
}

fn write_text(path: &Path, content: &str) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(path, content).map_err(|error| format!("failed to write {}: {error}", path.display()))
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
    let value = iter
        .next()
        .ok_or_else(|| format!("unknown option: {flag}"))?
        .to_string_lossy()
        .parse::<f64>()
        .map_err(|error| format!("invalid number for {flag}: {error}"))?;
    Ok(value)
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

fn optional_value_text(value: Option<&Value>) -> String {
    value
        .map(value_text)
        .unwrap_or_else(|| "undefined".to_string())
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki compare-direct-mesh-benchmark [options]\n\n\
Options:\n  --current <path>                    Current benchmark summary JSON.\n  \
--baseline <path>                   Baseline benchmark JSON.\n  \
--json-out <path>                   Optional compare JSON output path.\n  \
--report-out <path>                 Optional Markdown report output path.\n  \
--fail-on-elapsed-regression-pct <n>\n                                      \
Fail when elapsed mean regresses by more than n percent.\n  \
--fail-on-rss-regression-pct <n>    Fail when RSS mean regresses by more than n percent.\n  \
--help                              Show this message."
    );
}
