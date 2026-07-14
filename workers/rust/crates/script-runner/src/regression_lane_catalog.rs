use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod profile;
mod render;

type RunnerResult<T> = Result<T, String>;

const DEFAULT_TMP_ROOT: &str = "tmp";

const DIRECT_ELAPSED_WARN_PCT: f64 = 8.0;
const DIRECT_ELAPSED_FAIL_PCT: f64 = 15.0;
const DIRECT_RSS_WARN_PCT: f64 = 10.0;
const DIRECT_RSS_FAIL_PCT: f64 = 20.0;
const CATALOG_MEDIAN_WARN_PCT: f64 = 20.0;
const CATALOG_AVG_WARN_PCT: f64 = 30.0;
const MESH_TOTAL_WARN_MS: f64 = 22_000.0;
const MESH_TOTAL_FAIL_MS: f64 = 30_000.0;
const MESH_SLOWEST_WARN_MS: f64 = 8_000.0;
const MESH_SLOWEST_FAIL_MS: f64 = 12_000.0;

pub(crate) fn run_build_regression_lane_catalog(
    repo_root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = parse_args(repo_root, args)?;
    let mut lanes = vec![
        read_direct_mesh_lane(&options.tmp_root)?,
        read_workflow_catalog_lane(&options.tmp_root)?,
        read_workflow_mesh_lane(&options.tmp_root)?,
        profile::read_benchmark_profile_lane(&options.tmp_root)?,
    ];
    lanes.retain(Value::is_object);
    lanes.sort_by(|left, right| {
        generated_at(right)
            .partial_cmp(&generated_at(left))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let root = display_path(repo_root, &options.tmp_root);
    let overall = worst_gate_status(&enforceable_gate_statuses(&lanes));
    let payload = json!({
        "schema_version": "kyuubiki.regression-lane-catalog/v1",
        "root": root,
        "generated_at_unix_s": unix_seconds_now(),
        "policy": policy_json(),
        "overall_gate_status": overall,
        "lanes": lanes,
    });

    fs::create_dir_all(&options.tmp_root).map_err(|error| {
        format!(
            "failed to create tmp root {}: {error}",
            options.tmp_root.display()
        )
    })?;
    write_json(
        &options.tmp_root.join("regression-lane-catalog.json"),
        &payload,
    )?;
    let empty_lanes = Vec::new();
    let payload_lanes = payload["lanes"].as_array().unwrap_or(&empty_lanes);
    fs::write(
        options.tmp_root.join("regression-lane-catalog.md"),
        render::render_readme(&root, payload_lanes),
    )
    .map_err(|error| format!("failed to write regression-lane-catalog.md: {error}"))?;
    fs::write(
        options.tmp_root.join("regression-lane-catalog.html"),
        render::render_html(&root, payload_lanes),
    )
    .map_err(|error| format!("failed to write regression-lane-catalog.html: {error}"))?;
    println!("{root}");
    Ok(0)
}

struct Options {
    tmp_root: PathBuf,
}

fn parse_args(repo_root: &Path, args: Vec<OsString>) -> RunnerResult<Options> {
    let mut options = Options {
        tmp_root: repo_root.join(DEFAULT_TMP_ROOT),
    };
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.to_string_lossy().as_ref() {
            "--help" | "-h" => {
                println!(
                    "usage: kyuubiki-script-runner build-regression-lane-catalog [--tmp-root tmp]"
                );
                return Ok(options);
            }
            "--tmp-root" => options.tmp_root = path_arg(repo_root, &mut iter, "--tmp-root")?,
            other => return Err(format!("unknown or incomplete argument: {other}")),
        }
    }
    Ok(options)
}

fn read_direct_mesh_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let run = match latest_run(&tmp_root.join("direct-mesh-benchmark-container"))? {
        Some(run) => run,
        None => return Ok(Value::Null),
    };
    let summary_path = run.dir.join("summary.json");
    if !summary_path.exists() {
        return Ok(Value::Null);
    }
    let summary = read_json(&summary_path)?;
    let compare_path = run.dir.join("compare.json");
    let compare = read_optional_json(&compare_path)?;
    let elapsed = find_metric(compare.as_ref(), "elapsed_mean");
    let rss = find_metric(compare.as_ref(), "rss_mean");
    let mut reasons = string_array(
        compare
            .as_ref()
            .and_then(|value| value.pointer("/failures")),
    );
    push_pct_reason(
        &mut reasons,
        "elapsed mean regression",
        number_at(elapsed.as_ref(), "/delta_pct"),
        DIRECT_ELAPSED_WARN_PCT,
        DIRECT_ELAPSED_FAIL_PCT,
    );
    push_pct_reason(
        &mut reasons,
        "rss mean regression",
        number_at(rss.as_ref(), "/delta_pct"),
        DIRECT_RSS_WARN_PCT,
        DIRECT_RSS_FAIL_PCT,
    );
    let compare_ok = compare.as_ref().and_then(|value| value["ok"].as_bool());
    let gate_status = if compare_ok == Some(false)
        || reasons
            .iter()
            .any(|reason| reason.contains("fail threshold"))
    {
        "fail"
    } else if reasons.is_empty() {
        "pass"
    } else {
        "warn"
    };
    Ok(json!({
        "id": "direct-mesh-docker",
        "title": "Direct-mesh Docker",
        "category": "benchmark",
        "generated_at_unix_s": compare.as_ref()
            .and_then(|value| value["generated_at"].as_str())
            .map(parse_iso_unix_seconds)
            .unwrap_or(run.generated_at_unix_s),
        "status": compare_ok.map(|ok| if ok { "pass" } else { "fail" }).unwrap_or("observed"),
        "gate": gate(gate_status, reasons),
        "summary": format!(
            "Repeat {}, {} run(s), {} subtest(s).",
            int_at(Some(&summary), "/repeat"),
            array_len(summary.pointer("/runs")),
            summary.pointer("/runs/0/subtests").and_then(Value::as_array).map_or(0, Vec::len)
        ),
        "metrics": [
            metric("elapsed_mean", "s", round(number_at(Some(&summary), "/aggregate/elapsed_s/mean"), 3), number_at(elapsed.as_ref(), "/baseline"), number_at(elapsed.as_ref(), "/delta_pct")),
            metric("rss_mean", "KiB", round(number_at(Some(&summary), "/aggregate/max_rss_kib/mean"), 3), number_at(rss.as_ref(), "/baseline"), number_at(rss.as_ref(), "/delta_pct")),
        ],
        "links": [
            display_path(tmp_root, &summary_path),
            display_path(tmp_root, &compare_path),
            format!("direct-mesh-benchmark-container/{}/compare.md", run.slug),
        ],
    }))
}

fn read_workflow_catalog_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let run = match latest_run(&tmp_root.join("workflow-catalog-benchmark"))? {
        Some(run) => run,
        None => return Ok(Value::Null),
    };
    let summary_path = run.dir.join("summary.json");
    if !summary_path.exists() {
        return Ok(Value::Null);
    }
    let summary = read_json(&summary_path)?;
    let compare_path = run.dir.join("compare.json");
    let compare = read_optional_json(&compare_path)?;
    let mut reasons = string_array(
        compare
            .as_ref()
            .and_then(|value| value.pointer("/failures")),
    );
    if let Some(cases) = compare.as_ref().and_then(|value| value["cases"].as_array()) {
        for entry in cases {
            let case_id = entry["case_id"].as_str().unwrap_or("unknown");
            let median = find_metric(Some(entry), "median_elapsed_ms");
            let avg = find_metric(Some(entry), "avg_elapsed_ms");
            if number_at(median.as_ref(), "/delta_pct") > CATALOG_MEDIAN_WARN_PCT {
                reasons.push(format!(
                    "{case_id} median regression {}% exceeded warn threshold {CATALOG_MEDIAN_WARN_PCT}%",
                    number_at(median.as_ref(), "/delta_pct")
                ));
            }
            if number_at(avg.as_ref(), "/delta_pct") > CATALOG_AVG_WARN_PCT {
                reasons.push(format!(
                    "{case_id} average regression {}% exceeded warn threshold {CATALOG_AVG_WARN_PCT}%",
                    number_at(avg.as_ref(), "/delta_pct")
                ));
            }
        }
    }
    let compare_ok = compare.as_ref().and_then(|value| value["ok"].as_bool());
    let gate_status = if compare_ok == Some(false) {
        "fail"
    } else if reasons.is_empty() {
        "pass"
    } else {
        "warn"
    };
    let range = summary
        .pointer("/summary/median_elapsed_ms_range")
        .and_then(Value::as_array);
    Ok(json!({
        "id": "workflow-catalog",
        "title": "Workflow catalog",
        "category": "workflow-benchmark",
        "generated_at_unix_s": summary["generated_at"].as_str().map(parse_iso_unix_seconds).filter(|value| *value > 0).unwrap_or(run.generated_at_unix_s),
        "status": compare_ok.map(|ok| if ok { "pass" } else { "fail" }).unwrap_or("observed"),
        "gate": gate(gate_status, reasons),
        "summary": format!(
            "{} case(s), fastest `{}`, slowest `{}`.",
            int_at(Some(&summary), "/summary/case_count").max(array_len(summary.pointer("/cases")) as i64),
            summary.pointer("/summary/fastest_case_id").and_then(Value::as_str).unwrap_or("n/a"),
            summary.pointer("/summary/slowest_case_id").and_then(Value::as_str).unwrap_or("n/a")
        ),
        "metrics": [
            metric_plain("case_count", "case", json!(int_at(Some(&summary), "/summary/case_count").max(array_len(summary.pointer("/cases")) as i64))),
            metric_plain("median_elapsed_min", "ms", json!(range.and_then(|items| items.first()).and_then(Value::as_f64).unwrap_or(0.0))),
            metric_plain("median_elapsed_max", "ms", json!(range.and_then(|items| items.get(1)).and_then(Value::as_f64).unwrap_or(0.0))),
            metric_plain("regression_failures", "count", json!(array_len(compare.as_ref().and_then(|value| value.pointer("/failures"))))),
        ],
        "links": [
            display_path(tmp_root, &summary_path),
            display_path(tmp_root, &compare_path),
            format!("workflow-catalog-benchmark/{}/compare.md", run.slug),
        ],
    }))
}

fn read_workflow_mesh_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let index_path = tmp_root.join("workflow-mesh-regression/index.json");
    if !index_path.exists() {
        return Ok(Value::Null);
    }
    let index = read_json(&index_path)?;
    let latest = match index.pointer("/retained_runs/0") {
        Some(value) => value,
        None => return Ok(Value::Null),
    };
    let max_duration = latest["tests"]
        .as_array()
        .map(|tests| {
            tests
                .iter()
                .map(|test| number_at(Some(test), "/duration_ms"))
                .fold(0.0, f64::max)
        })
        .unwrap_or(0.0);
    let mut reasons = Vec::new();
    if latest["status"].as_str() != Some("passed") {
        reasons.push("latest workflow mesh regression run did not pass".to_string());
    }
    push_ms_reason(
        &mut reasons,
        "total duration",
        number_at(Some(latest), "/total_duration_ms"),
        MESH_TOTAL_WARN_MS,
        MESH_TOTAL_FAIL_MS,
    );
    push_ms_reason(
        &mut reasons,
        "slowest test duration",
        max_duration,
        MESH_SLOWEST_WARN_MS,
        MESH_SLOWEST_FAIL_MS,
    );
    let gate_status = if latest["status"].as_str() != Some("passed")
        || reasons
            .iter()
            .any(|reason| reason.contains("fail threshold"))
    {
        "fail"
    } else if reasons.is_empty() {
        "pass"
    } else {
        "warn"
    };
    let slug = latest["slug"].as_str().unwrap_or("latest");
    let summary_link = latest
        .pointer("/files/summary_json")
        .and_then(Value::as_str)
        .map(|file| format!("workflow-mesh-regression/{file}"))
        .unwrap_or_else(|| format!("workflow-mesh-regression/{slug}/summary.json"));
    Ok(json!({
        "id": "workflow-mesh",
        "title": "Workflow mesh",
        "category": "workflow-regression",
        "generated_at_unix_s": int_at(Some(latest), "/generated_at_unix_s").max(int_at(Some(&index), "/generated_at_unix_s")),
        "status": if latest["status"].as_str() == Some("passed") { "pass" } else { "fail" },
        "gate": gate(gate_status, reasons),
        "summary": format!("{} test(s), pass {}, fail {}.", int_at(Some(latest), "/total_tests"), int_at(Some(latest), "/total_pass"), int_at(Some(latest), "/total_fail")),
        "metrics": [
            metric_plain("total_duration", "ms", json!(round(number_at(Some(latest), "/total_duration_ms"), 3))),
            metric_plain("slowest_test_duration", "ms", json!(round(max_duration, 3))),
            metric_plain("retained_runs", "run", json!(array_len(index.pointer("/retained_runs")))),
        ],
        "links": ["workflow-mesh-regression/index.json", "workflow-mesh-regression/index.html", summary_link],
    }))
}

#[derive(Debug)]
struct RunDir {
    dir: PathBuf,
    generated_at_unix_s: u64,
    slug: String,
}

fn latest_run(root: &Path) -> RunnerResult<Option<RunDir>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Ok(None),
    };
    let mut runs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("failed to read {}: {error}", root.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to stat {}: {error}", entry.path().display()))?;
        if !file_type.is_dir() {
            continue;
        }
        let modified = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_secs());
        runs.push(RunDir {
            dir: entry.path(),
            generated_at_unix_s: modified,
            slug: entry.file_name().to_string_lossy().into_owned(),
        });
    }
    runs.sort_by(|left, right| right.generated_at_unix_s.cmp(&left.generated_at_unix_s));
    Ok(runs.into_iter().next())
}

fn read_optional_json(path: &Path) -> RunnerResult<Option<Value>> {
    if path.exists() {
        read_json(path).map(Some)
    } else {
        Ok(None)
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
    fs::write(path, format!("{content}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
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

fn policy_json() -> Value {
    json!({
        "directMeshDocker": {"elapsedWarnPct": 8, "elapsedFailPct": 15, "rssWarnPct": 10, "rssFailPct": 20},
        "workflowCatalog": {"caseMedianWarnPct": 20, "caseAvgWarnPct": 30},
        "workflowMesh": {"totalDurationWarnMs": 22000, "totalDurationFailMs": 30000, "slowestTestWarnMs": 8000, "slowestTestFailMs": 12000},
    })
}

fn enforceable_gate_statuses(lanes: &[Value]) -> Vec<String> {
    lanes
        .iter()
        .filter(|lane| lane["gate_scope"].as_str() != Some("advisory"))
        .map(|lane| {
            lane.pointer("/gate/status")
                .or_else(|| lane.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("pass")
                .to_string()
        })
        .collect()
}

fn worst_gate_status(statuses: &[String]) -> &'static str {
    if statuses.iter().any(|status| status == "fail") {
        "fail"
    } else if statuses.iter().any(|status| status == "warn") {
        "warn"
    } else {
        "pass"
    }
}

fn gate(status: &str, reasons: Vec<String>) -> Value {
    json!({"status": status, "reasons": reasons})
}

fn metric(name: &str, unit: &str, value: f64, baseline: f64, delta_pct: f64) -> Value {
    json!({"name": name, "unit": unit, "value": value, "baseline": null_if_zero(baseline), "delta_pct": null_if_zero(delta_pct)})
}

fn metric_plain(name: &str, unit: &str, value: Value) -> Value {
    json!({"name": name, "unit": unit, "value": value})
}

fn null_if_zero(value: f64) -> Value {
    if value == 0.0 {
        Value::Null
    } else {
        json!(value)
    }
}

fn find_metric(container: Option<&Value>, name: &str) -> Option<Value> {
    let metrics = container?.get("metrics")?.as_array()?;
    metrics
        .iter()
        .find(|metric| metric["name"].as_str() == Some(name))
        .cloned()
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn int_at(value: Option<&Value>, pointer: &str) -> i64 {
    value
        .and_then(|item| item.pointer(pointer))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

fn number_at(value: Option<&Value>, pointer: &str) -> f64 {
    value
        .and_then(|item| item.pointer(pointer))
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
}

fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map_or(0, Vec::len)
}

fn generated_at(value: &Value) -> f64 {
    value["generated_at_unix_s"].as_f64().unwrap_or(0.0)
}

fn push_pct_reason(reasons: &mut Vec<String>, label: &str, delta: f64, warn: f64, fail: f64) {
    if delta > fail {
        reasons.push(format!("{label} {delta}% exceeded fail threshold {fail}%"));
    } else if delta > warn {
        reasons.push(format!("{label} {delta}% exceeded warn threshold {warn}%"));
    }
}

fn push_ms_reason(reasons: &mut Vec<String>, label: &str, value: f64, warn: f64, fail: f64) {
    if value > fail {
        reasons.push(format!(
            "{label} {}ms exceeded fail threshold {fail}ms",
            round(value, 3)
        ));
    } else if value > warn {
        reasons.push(format!(
            "{label} {}ms exceeded warn threshold {warn}ms",
            round(value, 3)
        ));
    }
}

fn round(value: f64, digits: i32) -> f64 {
    let factor = 10_f64.powi(digits);
    (value * factor).round() / factor
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn unix_seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn parse_iso_unix_seconds(value: &str) -> u64 {
    let Some(date) = value.get(0..10) else {
        return 0;
    };
    let Some(time) = value.get(11..19) else {
        return 0;
    };
    let mut date_parts = date.split('-').filter_map(|part| part.parse::<i64>().ok());
    let mut time_parts = time.split(':').filter_map(|part| part.parse::<i64>().ok());
    let (Some(year), Some(month), Some(day)) =
        (date_parts.next(), date_parts.next(), date_parts.next())
    else {
        return 0;
    };
    let (Some(hour), Some(minute), Some(second)) =
        (time_parts.next(), time_parts.next(), time_parts.next())
    else {
        return 0;
    };
    let days = days_from_civil(year, month, day);
    (days * 86_400 + hour * 3_600 + minute * 60 + second).max(0) as u64
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
