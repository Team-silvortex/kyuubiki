use serde_json::{Value, json};
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(super) fn read_benchmark_profile_lane(tmp_root: &Path) -> RunnerResult<Value> {
    let index_path = tmp_root.join("benchmark-profile/index.json");
    if !index_path.exists() {
        return Ok(Value::Null);
    }
    let index = read_json(&index_path)?;
    let latest = index.pointer("/retained_runs/0");
    let matrix = index.pointer("/matrix_summaries/0");
    let coverage = index.pointer("/coverage_summaries/0");
    let gate = index
        .get("gate")
        .cloned()
        .unwrap_or_else(|| json!({"status": "warn", "reasons": ["missing profile index gate"]}));
    Ok(json!({
        "id": "benchmark-profile",
        "title": "Benchmark profile exploration",
        "category": "evidence",
        "gate_scope": "advisory",
        "generated_at_unix_s": latest.and_then(|value| value["generated_at_unix_s"].as_u64()).unwrap_or_else(|| index["generated_at_unix_s"].as_u64().unwrap_or(0)),
        "status": if gate["status"].as_str() == Some("pass") { "observed" } else { "needs-review" },
        "gate": gate,
        "summary": summary_text(latest, matrix, coverage),
        "metrics": profile_metrics(&index, latest, matrix, coverage),
        "links": profile_links(latest),
    }))
}

fn summary_text(
    latest: Option<&Value>,
    matrix: Option<&Value>,
    coverage: Option<&Value>,
) -> String {
    let coverage_text = coverage.map_or(String::new(), |item| {
        format!(
            " Coverage {}/{}.",
            int_at(Some(item), "/covered_case_count"),
            int_at(Some(item), "/expected_case_count")
        )
    });
    if let Some(matrix) = matrix {
        return format!(
            "{} run(s), {} case(s), leading matrix `{}`.{}",
            int_at(Some(matrix), "/run_count"),
            int_at(Some(matrix), "/case_count"),
            matrix["matrix"].as_str().unwrap_or("unknown"),
            coverage_text
        );
    }
    if let Some(latest) = latest {
        return format!(
            "{} case(s), matrix `{}`, profile `{}`.{}",
            int_at(Some(latest), "/case_count"),
            latest["matrix"].as_str().unwrap_or("unknown"),
            latest["profile"].as_str().unwrap_or("unknown"),
            coverage_text
        );
    }
    "No retained exploratory benchmark profile runs.".to_string()
}

fn profile_metrics(
    index: &Value,
    latest: Option<&Value>,
    matrix: Option<&Value>,
    coverage: Option<&Value>,
) -> Value {
    json!([
        {"name": "retained_runs", "unit": "run", "value": array_len(index.pointer("/retained_runs"))},
        {"name": "failed_runs", "unit": "run", "value": array_len(index.pointer("/failed_runs"))},
        {"name": "skipped_runs", "unit": "run", "value": array_len(index.pointer("/skipped_runs"))},
        {"name": "leading_matrix_total_median", "unit": "ms", "value": round(number_at(matrix, "/total_median_ms").max(number_at(latest, "/total_median_ms")), 3)},
        {"name": "leading_matrix_peak_rss", "unit": "MiB", "value": round(number_at(matrix, "/peak_rss_mib").max(number_at(latest, "/peak_rss_mib")), 3)},
        {"name": "coverage", "unit": "case", "value": format!("{}/{}", int_at(coverage, "/covered_case_count"), int_at(coverage, "/expected_case_count"))},
    ])
}

fn profile_links(latest: Option<&Value>) -> Value {
    let Some(latest) = latest else {
        return json!([
            "benchmark-profile/index.json",
            "benchmark-profile/README.md"
        ]);
    };
    let slug = latest["slug"].as_str().unwrap_or("latest");
    let summary = latest
        .pointer("/files/summary_json")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("{slug}/summary.json"));
    let readme = latest
        .pointer("/files/readme_md")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("{slug}/README.md"));
    json!([
        "benchmark-profile/index.json",
        "benchmark-profile/README.md",
        format!("benchmark-profile/{summary}"),
        format!("benchmark-profile/{readme}"),
    ])
}

fn read_json(path: &Path) -> RunnerResult<Value> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
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

fn round(value: f64, digits: i32) -> f64 {
    let factor = 10_f64.powi(digits);
    (value * factor).round() / factor
}
