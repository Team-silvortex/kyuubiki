use crate::models::BenchmarkMemoryStage;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BenchmarkHotspotSummary {
    pub(crate) label: String,
    pub(crate) elapsed_ms: f64,
    pub(crate) share_pct: f64,
    pub(crate) hint: String,
}

pub(crate) fn summarize_hotspot(
    stages: &[BenchmarkMemoryStage],
    median_ms: f64,
) -> Option<BenchmarkHotspotSummary> {
    let denominator = median_ms.max(1.0);
    let prefer_solver_kernels = stages
        .iter()
        .any(|stage| stage.label.starts_with("solve_spd_"));
    let stage = stages
        .iter()
        .filter(|stage| !prefer_solver_kernels || stage.label.starts_with("solve_spd_"))
        .filter_map(|stage| stage.elapsed_ms.map(|elapsed| (stage, elapsed)))
        .max_by(|(_, left), (_, right)| left.total_cmp(right))?;

    Some(BenchmarkHotspotSummary {
        label: stage.0.label.clone(),
        elapsed_ms: stage.1,
        share_pct: stage.1 / denominator * 100.0,
        hint: hotspot_hint(&stage.0.label).to_string(),
    })
}

fn hotspot_hint(label: &str) -> &'static str {
    match label {
        "solve_spd_preconditioner" => {
            "solver-bound: consider stencil-aware, multigrid, or parallel preconditioners"
        }
        "solve_spd_matvec" => "solver-bound: consider matrix-free/stencil matvec or parallel CSR",
        "solve_spd_dot" | "solve_spd_vector_update" | "solve_spd_direction_update" => {
            "vector-kernel-bound: consider fused reductions or parallel vector kernels"
        }
        "assemble_global" => "assembly-bound: consider element batching or specialized assemblers",
        "reduce_system" => "constraint-reduction-bound: consider cached free-dof maps",
        "solve_system" => "solver-bound: inspect nested solve_spd_* stages for the true kernel",
        "assemble" => "result-assembly-bound: consider streaming or sparse result materialization",
        _ => "inspect this stage before changing solver or workload strategy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotspot_summary_picks_largest_timed_stage() {
        let stages = vec![
            stage("assemble_global", 10.0),
            stage("solve_spd_preconditioner", 60.0),
            stage("solve_spd_matvec", 30.0),
        ];

        let summary = summarize_hotspot(&stages, 120.0).expect("summary");

        assert_eq!(summary.label, "solve_spd_preconditioner");
        assert_eq!(summary.elapsed_ms, 60.0);
        assert_eq!(summary.share_pct, 50.0);
        assert!(summary.hint.contains("multigrid"));
    }

    #[test]
    fn hotspot_summary_prefers_nested_solver_kernel_over_solve_wrapper() {
        let stages = vec![
            stage("solve_system", 100.0),
            stage("solve_spd_preconditioner", 60.0),
            stage("solve_spd_matvec", 30.0),
        ];

        let summary = summarize_hotspot(&stages, 120.0).expect("summary");

        assert_eq!(summary.label, "solve_spd_preconditioner");
    }

    fn stage(label: &str, elapsed_ms: f64) -> BenchmarkMemoryStage {
        BenchmarkMemoryStage {
            label: label.to_string(),
            rss_kib: 0,
            elapsed_ms: Some(elapsed_ms),
        }
    }
}
