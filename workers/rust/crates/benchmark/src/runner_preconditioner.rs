use kyuubiki_solver::SpdPreconditioner;

use crate::models::{
    BenchmarkCase, BenchmarkPreconditionerComparison, BenchmarkPreconditionerResult,
    BenchmarkResult, BenchmarkWorkload,
};

pub(crate) fn parse_preconditioner(value: &str) -> SpdPreconditioner {
    match value {
        "sgs" | "symmetric-gauss-seidel" => SpdPreconditioner::SymmetricGaussSeidel,
        _ => SpdPreconditioner::Jacobi,
    }
}

pub(crate) fn solver_preconditioners(value: &str) -> Vec<&'static str> {
    match value {
        "all" | "compare" => vec!["jacobi", "symmetric-gauss-seidel"],
        "auto" => vec!["auto"],
        "sgs" | "symmetric-gauss-seidel" => vec!["symmetric-gauss-seidel"],
        _ => vec!["jacobi"],
    }
}

pub(crate) fn effective_preconditioner<'a>(case: &BenchmarkCase, requested: &'a str) -> &'a str {
    if requested != "auto" {
        return requested;
    }

    match case.workload {
        BenchmarkWorkload::Truss2d(_)
        | BenchmarkWorkload::HeatPlaneQuad2d(_)
        | BenchmarkWorkload::PlaneTriangle2d(_)
        | BenchmarkWorkload::PlaneQuad2d(_)
        | BenchmarkWorkload::ThermalPlaneTriangle2d(_)
        | BenchmarkWorkload::ThermalPlaneQuad2d(_) => "symmetric-gauss-seidel",
        _ => "jacobi",
    }
}

pub(crate) fn preconditioner_comparisons(
    cases: &[BenchmarkResult],
) -> Vec<BenchmarkPreconditionerComparison> {
    let mut groups = std::collections::BTreeMap::<String, Vec<&BenchmarkResult>>::new();
    for case in cases
        .iter()
        .filter(|case| case.ok && case.solver_preconditioner.is_some())
    {
        groups
            .entry(base_case_id(&case.id).to_string())
            .or_default()
            .push(case);
    }

    groups
        .into_iter()
        .filter_map(|(base_case_id, items)| {
            let mut compared = items
                .into_iter()
                .filter(|case| case.id.contains('#'))
                .map(|case| BenchmarkPreconditionerResult {
                    median_ms: case.median_ms,
                    solver_iterations: case.solver_iterations,
                    solver_preconditioner: case.solver_preconditioner.clone().unwrap_or_default(),
                })
                .collect::<Vec<_>>();
            compared.sort_by(|left, right| left.median_ms.total_cmp(&right.median_ms));
            if compared
                .iter()
                .map(|item| item.solver_preconditioner.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                < 2
            {
                return None;
            }
            let winner = compared[0].clone();
            let slowest = compared
                .last()
                .expect("comparison should contain at least two items")
                .clone();
            Some(BenchmarkPreconditionerComparison {
                base_case_id,
                winner_median_ms: winner.median_ms,
                winner_preconditioner: winner.solver_preconditioner,
                winner_solver_iterations: winner.solver_iterations,
                winner_speedup_ratio: slowest.median_ms / winner.median_ms.max(f64::EPSILON),
                winner_iteration_reduction_pct: iteration_reduction_pct(
                    winner.solver_iterations,
                    slowest.solver_iterations,
                ),
                compared,
            })
        })
        .collect()
}

fn iteration_reduction_pct(winner: Option<usize>, slowest: Option<usize>) -> Option<f64> {
    let winner = winner?;
    let slowest = slowest?;
    if slowest == 0 {
        return None;
    }
    Some((slowest.saturating_sub(winner) as f64 / slowest as f64) * 100.0)
}

fn base_case_id(case_id: &str) -> &str {
    case_id.split_once('#').map_or(case_id, |(base, _)| base)
}
