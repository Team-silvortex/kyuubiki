use crate::models::BenchmarkMemoryStage;
use crate::runner_structural::WorkloadMetrics;
use crate::runner_util::percentile;

pub(crate) fn aggregate_memory_stage_runs(
    runs: &[Vec<BenchmarkMemoryStage>],
) -> Vec<BenchmarkMemoryStage> {
    let Some(first) = runs.first() else {
        return Vec::new();
    };

    first
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            let matching = runs
                .iter()
                .filter_map(|run| run.get(index))
                .filter(|candidate| candidate.label == stage.label)
                .collect::<Vec<_>>();
            let mut elapsed = matching
                .iter()
                .filter_map(|candidate| candidate.elapsed_ms)
                .collect::<Vec<_>>();
            elapsed.sort_by(|left, right| left.total_cmp(right));
            BenchmarkMemoryStage {
                label: stage.label.clone(),
                rss_kib: matching
                    .iter()
                    .map(|candidate| candidate.rss_kib)
                    .max()
                    .unwrap_or(stage.rss_kib),
                elapsed_ms: (!elapsed.is_empty()).then(|| percentile(&elapsed, 0.5)),
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_metrics(
    metrics: WorkloadMetrics,
    node_count: &mut usize,
    element_count: &mut usize,
    dof_count: &mut usize,
    max_displacement: &mut f64,
    max_stress: &mut f64,
    memory_stages: &mut Vec<BenchmarkMemoryStage>,
    solver_iterations: &mut Option<usize>,
    solver_matrix_non_zero_count: &mut Option<usize>,
    solver_residual_norm: &mut Option<f64>,
    solver_preconditioner_name: &mut Option<String>,
) {
    *node_count = metrics.node_count;
    *element_count = metrics.element_count;
    *dof_count = metrics.dof_count;
    *max_displacement = metrics.max_displacement;
    *max_stress = metrics.max_stress;

    if !metrics.memory_stages.is_empty() {
        *memory_stages = metrics.memory_stages;
    }
    if metrics.solver_iterations.is_some() {
        *solver_iterations = metrics.solver_iterations;
    }
    if metrics.solver_matrix_non_zero_count.is_some() {
        *solver_matrix_non_zero_count = metrics.solver_matrix_non_zero_count;
    }
    if metrics.solver_residual_norm.is_some() {
        *solver_residual_norm = metrics.solver_residual_norm;
    }
    if metrics.solver_preconditioner.is_some() {
        *solver_preconditioner_name = metrics.solver_preconditioner;
    }
}

#[cfg(test)]
mod tests {
    use super::aggregate_memory_stage_runs;
    use crate::models::BenchmarkMemoryStage;

    #[test]
    fn aggregates_stage_elapsed_median_and_peak_rss_across_repeats() {
        let runs = vec![
            vec![stage("solve", 100, 4.0), stage("assemble", 80, 2.0)],
            vec![stage("solve", 140, 8.0), stage("assemble", 90, 6.0)],
            vec![stage("solve", 120, 6.0), stage("assemble", 85, 4.0)],
        ];

        let aggregated = aggregate_memory_stage_runs(&runs);

        assert_eq!(aggregated[0].label, "solve");
        assert_eq!(aggregated[0].rss_kib, 140);
        assert_eq!(aggregated[0].elapsed_ms, Some(6.0));
        assert_eq!(aggregated[1].rss_kib, 90);
        assert_eq!(aggregated[1].elapsed_ms, Some(4.0));
    }

    fn stage(label: &str, rss_kib: u64, elapsed_ms: f64) -> BenchmarkMemoryStage {
        BenchmarkMemoryStage {
            label: label.to_string(),
            rss_kib,
            elapsed_ms: Some(elapsed_ms),
        }
    }
}
