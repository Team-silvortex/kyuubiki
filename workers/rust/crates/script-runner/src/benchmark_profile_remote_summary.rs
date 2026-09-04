use crate::RunnerResult;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub(crate) fn write_profile_outputs(
    json_path: &Path,
    md_path: &Path,
    summary_path: &Path,
) -> RunnerResult<()> {
    let report = read_profile_report(json_path)?;
    validate_material_integration_report(&report)?;
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
    writeln!(
        output,
        "- Peak RSS scope: `{}`",
        string_field(report, "rss_scope")
    )
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
    write_material_integration_comparison(
        &mut output,
        summary.material_integration_comparison.as_ref(),
    )?;
    write_solver_comparison(&mut output, cases)?;
    Ok(())
}

fn write_case_table(output: &mut File, cases: &[Value]) -> RunnerResult<()> {
    writeln!(
        output,
        "| Case | Nodes | Elements | Median ms | Peak RSS MiB | Solver | Solver reason | Iterations | Residual |"
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(output, "|---|---:|---:|---:|---:|---|---|---:|---:|")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    for entry in cases {
        writeln!(
            output,
            "| `{}` | {} | {} | {:.3} | {} | `{}` | `{}` | {} | {} |",
            string_field(entry, "id"),
            number_field(entry, "node_count"),
            number_field(entry, "element_count"),
            entry["median_ms"].as_f64().unwrap_or(0.0),
            rss_mib_field(entry),
            string_field(entry, "solver_preconditioner"),
            string_field(entry, "solver_preconditioner_reason"),
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
        "schema_version": "kyuubiki.benchmark-profile-summary/v2",
        "profile": string_field(report, "profile"),
        "matrix": string_field(report, "matrix"),
        "repeat": report["repeat"].clone(),
        "rss_scope": report["rss_scope"].clone(),
        "case_count": cases.len(),
        "case_ids": summary.case_ids,
        "solver_case_metrics": summary.solver_case_metrics,
        "solver_preconditioners": summary.solver_preconditioners,
        "material_integration_comparison": summary.material_integration_comparison,
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
    solver_case_metrics: Vec<Value>,
    solver_preconditioners: Vec<String>,
    material_integration_comparison: Option<Value>,
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
        let solver_preconditioners = cases
            .iter()
            .filter_map(|entry| entry["solver_preconditioner"].as_str())
            .map(str::to_string)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let solver_case_metrics = cases
            .iter()
            .filter_map(|entry| {
                let id = entry["id"].as_str()?;
                let preconditioner = entry["solver_preconditioner"].as_str()?;
                Some(json!({
                    "id": id,
                    "solver_preconditioner": preconditioner,
                    "solver_preconditioner_reason": entry["solver_preconditioner_reason"].clone(),
                    "solver_iterations": entry["solver_iterations"].clone(),
                    "solver_residual_norm": entry["solver_residual_norm"].clone(),
                }))
            })
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
        let material_integration_comparison = material_integration_comparison(cases);

        Self {
            case_ids,
            solver_case_metrics,
            solver_preconditioners,
            material_integration_comparison,
            total_median_ms,
            peak_rss_mib,
            slowest_case,
        }
    }
}

fn material_integration_comparison(cases: &[Value]) -> Option<Value> {
    let fixed = cases
        .iter()
        .find(|entry| entry["family"].as_str() == Some("frame_2d_material_fixed"))?;
    let adaptive = cases
        .iter()
        .find(|entry| entry["family"].as_str() == Some("frame_2d_material_adaptive"))?;
    let displacement_delta = absolute_delta(fixed, adaptive, "max_displacement")?;
    let stress_delta = absolute_delta(fixed, adaptive, "max_stress")?;
    let residual_delta = absolute_delta(fixed, adaptive, "solver_residual_norm")?;
    let fixed_iterations = fixed["solver_iterations"].as_u64();
    let adaptive_iterations = adaptive["solver_iterations"].as_u64();
    let iterations_match = fixed_iterations.is_some() && fixed_iterations == adaptive_iterations;
    let response_match = iterations_match
        && fields_close(fixed, adaptive, "max_displacement")
        && fields_close(fixed, adaptive, "max_stress")
        && fields_close(fixed, adaptive, "solver_residual_norm");

    Some(json!({
        "fixed_case_id": string_field(fixed, "id"),
        "adaptive_case_id": string_field(adaptive, "id"),
        "fixed_median_ms": fixed["median_ms"].clone(),
        "adaptive_median_ms": adaptive["median_ms"].clone(),
        "median_delta_pct": relative_delta_pct(
            fixed["median_ms"].as_f64(),
            adaptive["median_ms"].as_f64(),
        ),
        "fixed_solver_iterations": fixed_iterations,
        "adaptive_solver_iterations": adaptive_iterations,
        "solver_iterations_match": iterations_match,
        "residual_abs_delta": residual_delta,
        "max_displacement_abs_delta": displacement_delta,
        "max_stress_abs_delta": stress_delta,
        "response_match": response_match,
    }))
}

fn validate_material_integration_report(report: &Value) -> RunnerResult<()> {
    if report["matrix"].as_str() != Some("material-integration") {
        return Ok(());
    }
    let cases = report["cases"]
        .as_array()
        .ok_or_else(|| "material-integration report is missing cases array".to_string())?;
    if cases
        .iter()
        .any(|entry| entry["ok"].as_bool() != Some(true))
    {
        return Err(
            "material-integration report requires every benchmark case to declare ok=true".into(),
        );
    }
    let fixed_count = cases
        .iter()
        .filter(|entry| entry["family"].as_str() == Some("frame_2d_material_fixed"))
        .count();
    let adaptive_count = cases
        .iter()
        .filter(|entry| entry["family"].as_str() == Some("frame_2d_material_adaptive"))
        .count();
    if fixed_count != 1 || adaptive_count != 1 {
        return Err(format!(
            "material-integration report requires exactly one fixed and one adaptive family; found fixed={fixed_count}, adaptive={adaptive_count}"
        ));
    }
    let comparison = material_integration_comparison(cases).ok_or_else(|| {
        "material-integration report requires fixed and adaptive benchmark families".to_string()
    })?;
    if comparison["response_match"].as_bool() != Some(true) {
        return Err(format!(
            "material-integration response mismatch: iterations={}, residual_delta={:.3e}, displacement_delta={:.3e}, stress_delta={:.3e}",
            comparison["solver_iterations_match"]
                .as_bool()
                .unwrap_or(false),
            comparison["residual_abs_delta"]
                .as_f64()
                .unwrap_or(f64::NAN),
            comparison["max_displacement_abs_delta"]
                .as_f64()
                .unwrap_or(f64::NAN),
            comparison["max_stress_abs_delta"]
                .as_f64()
                .unwrap_or(f64::NAN),
        ));
    }
    Ok(())
}

fn write_material_integration_comparison(
    output: &mut File,
    comparison: Option<&Value>,
) -> RunnerResult<()> {
    let Some(comparison) = comparison else {
        return Ok(());
    };
    writeln!(output, "\n## Material Integration Comparison\n")
        .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(
        output,
        "- Median delta: `{:+.2}%`",
        comparison["median_delta_pct"].as_f64().unwrap_or_default()
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(
        output,
        "- Solver iterations match: `{}`",
        comparison["solver_iterations_match"]
            .as_bool()
            .unwrap_or(false)
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(
        output,
        "- Response match: `{}`",
        comparison["response_match"].as_bool().unwrap_or(false)
    )
    .map_err(|error| format!("failed to write markdown: {error}"))?;
    writeln!(
        output,
        "- Absolute deltas: residual `{:.3e}`, displacement `{:.3e}`, stress `{:.3e}`",
        comparison["residual_abs_delta"]
            .as_f64()
            .unwrap_or_default(),
        comparison["max_displacement_abs_delta"]
            .as_f64()
            .unwrap_or_default(),
        comparison["max_stress_abs_delta"]
            .as_f64()
            .unwrap_or_default()
    )
    .map_err(|error| format!("failed to write markdown: {error}"))
}

fn absolute_delta(left: &Value, right: &Value, field: &str) -> Option<f64> {
    Some((left[field].as_f64()? - right[field].as_f64()?).abs())
}

fn fields_close(left: &Value, right: &Value, field: &str) -> bool {
    let (Some(left), Some(right)) = (left[field].as_f64(), right[field].as_f64()) else {
        return false;
    };
    (left - right).abs() <= 1.0e-12 * left.abs().max(right.abs()).max(1.0)
}

fn relative_delta_pct(reference: Option<f64>, candidate: Option<f64>) -> Option<f64> {
    let (Some(reference), Some(candidate)) = (reference, candidate) else {
        return None;
    };
    (reference.abs() > f64::EPSILON).then_some(((candidate - reference) / reference) * 100.0)
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

fn solver_pairs(cases: &[Value]) -> Vec<(String, &Value, &Value)> {
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
    use super::{
        SummaryStats, delta_pct, material_integration_comparison, solver_pairs, stage_elapsed_ms,
        validate_material_integration_report,
    };
    use serde_json::{Value, json};
    use std::path::Path;

    #[test]
    fn summary_stats_capture_matrix_totals() {
        let cases = vec![
            json!({
                "id": "thermal-plane-triangle-400k",
                "median_ms": 64092.147,
                "peak_rss_kib": 1549000,
                "solver_preconditioner": "ic0"
            }),
            json!({
                "id": "thermal-plane-quad-400k",
                "median_ms": 57214.733,
                "peak_rss_kib": 1664800,
                "solver_preconditioner": "symmetric-gauss-seidel"
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
        assert_eq!(
            summary.solver_preconditioners,
            vec!["ic0".to_string(), "symmetric-gauss-seidel".to_string()]
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

    #[test]
    fn material_integration_pair_reports_cost_and_response_equivalence() {
        let cases = vec![
            material_case("fixed", "frame_2d_material_fixed", 100.0, 12),
            material_case("adaptive", "frame_2d_material_adaptive", 101.3, 12),
        ];

        let comparison = material_integration_comparison(&cases).expect("paired comparison");

        assert!(
            (comparison["median_delta_pct"]
                .as_f64()
                .expect("median delta")
                - 1.3)
                .abs()
                < 1.0e-12
        );
        assert_eq!(comparison["solver_iterations_match"], true);
        assert_eq!(comparison["response_match"], true);
        assert_eq!(comparison["max_displacement_abs_delta"], 0.0);
    }

    #[test]
    fn material_integration_comparison_requires_both_families() {
        let cases = vec![material_case("fixed", "frame_2d_material_fixed", 100.0, 12)];

        assert!(material_integration_comparison(&cases).is_none());
    }

    #[test]
    fn material_integration_report_rejects_response_drift() {
        let report = json!({
            "matrix": "material-integration",
            "cases": [
                material_case("fixed", "frame_2d_material_fixed", 100.0, 12),
                material_case("adaptive", "frame_2d_material_adaptive", 101.3, 13),
            ]
        });

        let error = validate_material_integration_report(&report).unwrap_err();

        assert!(error.contains("response mismatch"));
    }

    #[test]
    fn material_integration_report_rejects_incomplete_or_implicit_cases() {
        let mut fixed = material_case("fixed", "frame_2d_material_fixed", 100.0, 12);
        fixed.as_object_mut().expect("case object").remove("ok");
        let report = json!({
            "matrix": "material-integration",
            "cases": [fixed]
        });

        let error = validate_material_integration_report(&report).unwrap_err();

        assert!(error.contains("ok=true"));

        let report = json!({
            "matrix": "material-integration",
            "cases": [material_case("fixed", "frame_2d_material_fixed", 100.0, 12)]
        });
        let error = validate_material_integration_report(&report).unwrap_err();
        assert!(error.contains("exactly one fixed and one adaptive"));
    }

    #[test]
    fn retained_linux_material_integration_evidence_is_self_consistent() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../../evidence/operator-screening/frame-2d-material-integration-linux.json",
        );
        let evidence: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("retained evidence"))
                .expect("valid retained evidence JSON");

        validate_material_integration_report(&evidence).expect("valid material evidence");
        assert_eq!(
            evidence["schema_version"],
            "kyuubiki.material-integration-cross-host-screening/v1"
        );
        let computed =
            material_integration_comparison(evidence["cases"].as_array().expect("evidence cases"))
                .expect("paired evidence");
        assert_eq!(
            evidence["comparison"]["response_match"],
            computed["response_match"]
        );
        let stored_delta = evidence["comparison"]["median_delta_pct"]
            .as_f64()
            .expect("stored median delta");
        let computed_delta = computed["median_delta_pct"]
            .as_f64()
            .expect("computed median delta");
        assert!((stored_delta - computed_delta).abs() < 1.0e-12);
    }

    fn material_case(id: &str, family: &str, median_ms: f64, iterations: u64) -> Value {
        json!({
            "id": id,
            "family": family,
            "ok": true,
            "median_ms": median_ms,
            "solver_iterations": iterations,
            "solver_residual_norm": 1.0e-12,
            "max_displacement": 2.0e-5,
            "max_stress": 3.0e5,
        })
    }
}
