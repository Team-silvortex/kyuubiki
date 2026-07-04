use crate::models::BenchmarkMemoryStage;
use crate::runner_structural::WorkloadMetrics;

pub(crate) fn apply_metrics(
    metrics: WorkloadMetrics,
    node_count: &mut usize,
    element_count: &mut usize,
    dof_count: &mut usize,
    max_displacement: &mut f64,
    max_stress: &mut f64,
    memory_stages: &mut Vec<BenchmarkMemoryStage>,
    solver_iterations: &mut Option<usize>,
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
    if metrics.solver_residual_norm.is_some() {
        *solver_residual_norm = metrics.solver_residual_norm;
    }
    if metrics.solver_preconditioner.is_some() {
        *solver_preconditioner_name = metrics.solver_preconditioner;
    }
}
