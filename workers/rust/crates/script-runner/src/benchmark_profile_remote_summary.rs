use crate::RunnerResult;
use serde_json::{Value, json};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) fn write_profile_outputs(
    json_path: &Path,
    md_path: &Path,
    summary_path: &Path,
) -> RunnerResult<()> {
    let report = read_profile_report(json_path)?;
    write_markdown_summary(&report, md_path)?;
    write_json_summary(&report, summary_path)?;
    Ok(())
}

fn read_profile_report(json_path: &Path) -> RunnerResult<Value> {
    let content = std::fs::read_to_string(json_path)
        .map_err(|error| format!("failed to read {}: {error}", json_path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("failed to parse {}: {error}", json_path.display()))
}

fn write_markdown_summary(report: &Value, md_path: &Path) -> RunnerResult<()> {
    let cases = report["cases"].as_array().ok_or_else(|| {
        format!(
            "benchmark profile report is missing cases array: {}",
            md_path.display()
        )
    })?;
    let mut output = File::create(md_path)
        .map_err(|error| format!("failed to create {}: {error}", md_path.display()))?;
    writeln!(output, "# Benchmark profile smoke\n")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Profile: `{}`", string_field(report, "profile"))
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Matrix: `{}`", string_field(report, "matrix"))
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Repeat: `{}`", number_field(report, "repeat"))
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Case count: `{}`", cases.len())
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    let summary = SummaryStats::from_cases(cases);
    writeln!(
        output,
        "- Total median ms: `{:.3}`",
        summary.total_median_ms
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Peak RSS MiB: `{:.1}`", summary.peak_rss_mib)
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "- Slowest case: `{}`\n", summary.slowest_case)
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    write_case_table(&mut output, cases)?;
    write_solver_comparison(&mut output, cases)?;
    Ok(())
}

fn write_case_table(output: &mut File, cases: &[Value]) -> RunnerResult<()> {
    writeln!(
        output,
        "| Case | Nodes | Elements | Median ms | Peak RSS MiB | Solver | Iterations | Residual |"
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "|---|---:|---:|---:|---:|---|---:|---:|")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    for entry in cases {
        writeln!(
            output,
            "| `{}` | {} | {} | {:.3} | {} | `{}` | {} | {} |",
            string_field(entry, "id"),
            number_field(entry, "node_count"),
            number_field(entry, "element_count"),
            entry["median_ms"].as_f64().unwrap_or(0.0),
            rss_mib_field(entry),
            string_field(entry, "solver_preconditioner"),
            number_field(entry, "solver_iterations"),
            residual_field(entry)
        )
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    }
    Ok(())
}

fn write_json_summary(report: &Value, summary_path: &Path) -> RunnerResult<()> {
    let cases = report["cases"].as_array().ok_or_else(|| {
        format!(
            "benchmark profile report is missing cases array: {}",
            summary_path.display()
        )
    })?;
    let summary = SummaryStats::from_cases(cases);
    let payload = json!({
        "schema_version": "kyuubiki.benchmark-profile-summary/v1",
        "profile": string_field(report, "profile"),
        "matrix": string_field(report, "matrix"),
        "repeat": report["repeat"].clone(),
        "case_count": cases.len(),
        "case_ids": summary.case_ids,
        "total_median_ms": summary.total_median_ms,
        "peak_rss_mib": summary.peak_rss_mib,
        "slowest_case": summary.slowest_case,
    });
    std::fs::write(
        summary_path,
        format!("{}\n", serde_json::to_string_pretty(&payload).unwrap()),
    )
    .map_err(|error| format!("failed to write {}: {error}", summary_path.display()))
}

struct SummaryStats {
    case_ids: Vec<String>,
    total_median_ms: f64,
    peak_rss_mib: f64,
    slowest_case: String,
}

impl SummaryStats {
    fn from_cases(cases: &[Value]) -> Self {
        let case_ids = cases
            .iter()
            .map(|entry| string_field(entry, "id"))
            .collect();
        let total_median_ms = cases
            .iter()
            .filter_map(|entry| entry["median_ms"].as_f64())
            .sum();
        let peak_rss_mib = cases
            .iter()
            .filter_map(|entry| entry["peak_rss_kib"].as_f64())
            .fold(0.0_f64, f64::max)
            / 1024.0;
        let slowest_case = cases
            .iter()
            .max_by(|left, right| {
                left["median_ms"]
                    .as_f64()
                    .unwrap_or(0.0)
                    .total_cmp(&right["median_ms"].as_f64().unwrap_or(0.0))
            })
            .map(|entry| string_field(entry, "id"))
            .unwrap_or_else(|| "--".to_string());

        Self {
            case_ids,
            total_median_ms,
            peak_rss_mib,
            slowest_case,
        }
    }
}

fn write_solver_comparison(output: &mut File, cases: &[Value]) -> RunnerResult<()> {
    let pairs = solver_pairs(cases);
    if pairs.is_empty() {
        return Ok(());
    }

    writeln!(output, "\n## Solver Strategy Comparison\n")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(
        output,
        "| Base Case | Reference | Candidate | Median Delta | Solve Delta | Iteration Delta | Peak RSS Delta |"
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "|---|---|---|---:|---:|---:|---:|")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    for (base, reference, candidate) in pairs {
        writeln!(
            output,
            "| `{}` | `{}` | `{}` | {} | {} | {} | {} |",
            base,
            string_field(reference, "solver_preconditioner"),
            string_field(candidate, "solver_preconditioner"),
            delta_pct(
                reference["median_ms"].as_f64(),
                candidate["median_ms"].as_f64()
            ),
            delta_pct(
                stage_elapsed_ms(reference, "solve_system"),
                stage_elapsed_ms(candidate, "solve_system")
            ),
            delta_pct(
                reference["solver_iterations"].as_f64(),
                candidate["solver_iterations"].as_f64()
            ),
            delta_pct(
                reference["peak_rss_kib"].as_f64(),
                candidate["peak_rss_kib"].as_f64()
            )
        )
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    }
    Ok(())
}

fn solver_pairs<'a>(cases: &'a [Value]) -> Vec<(String, &'a Value, &'a Value)> {
    cases
        .iter()
        .filter_map(|reference| {
            let id = string_field(reference, "id");
            let base = id.strip_suffix("#jacobi")?.to_string();
            let candidate_id = format!("{base}#symmetric-gauss-seidel");
            cases
                .iter()
                .find(|case| string_field(case, "id") == candidate_id)
                .map(|candidate| (base, reference, candidate))
        })
        .collect()
}

fn delta_pct(reference: Option<f64>, candidate: Option<f64>) -> String {
    let (Some(reference), Some(candidate)) = (reference, candidate) else {
        return "--".to_string();
    };
    if reference.abs() <= f64::EPSILON {
        return "--".to_string();
    }
    format!("{:+.2}%", ((candidate - reference) / reference) * 100.0)
}

fn stage_elapsed_ms(value: &Value, label: &str) -> Option<f64> {
    value["memory_stages"]
        .as_array()?
        .iter()
        .find(|stage| stage["label"].as_str() == Some(label))
        .and_then(|stage| stage["elapsed_ms"].as_f64())
}

fn rss_mib_field(value: &Value) -> String {
    value["peak_rss_kib"]
        .as_f64()
        .map(|value| format!("{:.1}", value / 1024.0))
        .unwrap_or_else(|| "--".to_string())
}

fn residual_field(value: &Value) -> String {
    value["solver_residual_norm"]
        .as_f64()
        .map(|value| format!("{value:.3e}"))
        .unwrap_or_else(|| "--".to_string())
}

fn number_field(value: &Value, name: &str) -> String {
    value[name]
        .as_i64()
        .map(|number| number.to_string())
        .or_else(|| value[name].as_u64().map(|number| number.to_string()))
        .or_else(|| value[name].as_f64().map(|number| number.to_string()))
        .unwrap_or_else(|| "--".to_string())
}

fn string_field(value: &Value, name: &str) -> String {
    value[name].as_str().unwrap_or("--").to_string()
}

#[cfg(test)]
mod tests {
    use super::{SummaryStats, delta_pct, solver_pairs, stage_elapsed_ms};
    use serde_json::json;

    #[test]
    fn summary_stats_capture_matrix_totals() {
        let cases = vec![
            json!({
                "id": "thermal-plane-triangle-400k",
                "median_ms": 64092.147,
                "peak_rss_kib": 1549000
            }),
            json!({
                "id": "thermal-plane-quad-400k",
                "median_ms": 57214.733,
                "peak_rss_kib": 1664800
            }),
        ];

        let summary = SummaryStats::from_cases(&cases);

        assert_eq!(
            summary.case_ids,
            vec![
                "thermal-plane-triangle-400k".to_string(),
                "thermal-plane-quad-400k".to_string()
            ]
        );
        assert_eq!(summary.slowest_case, "thermal-plane-triangle-400k");
        assert!((summary.total_median_ms - 121306.88).abs() < 0.001);
        assert!((summary.peak_rss_mib - 1625.78125).abs() < 0.001);
    }

    #[test]
    fn solver_pairs_match_jacobi_and_sgs_rows() {
        let cases = vec![
            json!({
                "id": "truss-roof-300k#jacobi",
                "solver_preconditioner": "jacobi",
                "memory_stages": [
                    { "label": "solve_system", "elapsed_ms": 100.0 }
                ]
            }),
            json!({
                "id": "truss-roof-300k#symmetric-gauss-seidel",
                "solver_preconditioner": "symmetric-gauss-seidel",
                "memory_stages": [
                    { "label": "solve_system", "elapsed_ms": 75.0 }
                ]
            }),
        ];

        let pairs = solver_pairs(&cases);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "truss-roof-300k");
        assert_eq!(stage_elapsed_ms(pairs[0].1, "solve_system"), Some(100.0));
        assert_eq!(stage_elapsed_ms(pairs[0].2, "solve_system"), Some(75.0));
    }

    #[test]
    fn delta_pct_formats_signed_relative_change() {
        assert_eq!(delta_pct(Some(100.0), Some(75.0)), "-25.00%");
        assert_eq!(delta_pct(Some(100.0), Some(125.0)), "+25.00%");
        assert_eq!(delta_pct(Some(0.0), Some(1.0)), "--");
    }
}
