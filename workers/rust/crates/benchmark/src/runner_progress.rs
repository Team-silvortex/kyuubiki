use crate::models::{BenchmarkCase, BenchmarkResult};
use crate::runner_shape::workload_shape;

pub(crate) fn print_case_start(
    case: &BenchmarkCase,
    preconditioner: &str,
    selection_reason: &str,
    repeat: usize,
) {
    let (nodes, elements, dofs) = workload_shape(&case.workload);
    eprintln!(
        "benchmark progress: start case={} preconditioner={} selection_reason={} nodes={} elements={} dofs={} repeat={}",
        case.id, preconditioner, selection_reason, nodes, elements, dofs, repeat
    );
}

pub(crate) fn print_case_done(result: &BenchmarkResult) {
    eprintln!(
        "benchmark progress: done case={} ok={} preconditioner={} selection_reason={} median_ms={:.3} peak_rss_mib={:.1} error={}",
        result.id,
        result.ok,
        result.solver_preconditioner.as_deref().unwrap_or("--"),
        result
            .solver_preconditioner_reason
            .as_deref()
            .unwrap_or("--"),
        result.median_ms,
        result.peak_rss_kib as f64 / 1024.0,
        result.error.as_deref().unwrap_or("--")
    );
}
