use kyuubiki_solver::SpdPreconditioner;

use crate::models::{BenchmarkCase, BenchmarkWorkload};

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
        BenchmarkWorkload::ThermalPlaneTriangle2d(_) | BenchmarkWorkload::ThermalPlaneQuad2d(_) => {
            "symmetric-gauss-seidel"
        }
        _ => "jacobi",
    }
}
