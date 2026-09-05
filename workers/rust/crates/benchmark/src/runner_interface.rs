use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_protocol::AnalysisResult;

use crate::models::BenchmarkWorkload;
use crate::runner_structural::WorkloadMetrics;

pub(crate) fn run_interface_workload(
    workload: &BenchmarkWorkload,
) -> Option<Result<WorkloadMetrics, String>> {
    let result = match workload {
        BenchmarkWorkload::CohesiveInterface1d(request) => {
            solve(EngineSolveRequest::CohesiveInterface1d(request.clone())).map(|result| {
                let AnalysisResult::CohesiveInterface1d(result) = result else {
                    unreachable!("cohesive interface 1d solve should return matching result")
                };
                let max_separation = result
                    .steps
                    .iter()
                    .map(|step| step.separation.abs())
                    .fold(0.0_f64, f64::max);
                let mut metrics =
                    WorkloadMetrics::from_counts(2, 1, 1, max_separation, result.max_traction);
                metrics.history_step_count = Some(result.steps.len());
                metrics
            })
        }
        BenchmarkWorkload::CohesiveInterface2d(request) => {
            solve(EngineSolveRequest::CohesiveInterface2d(request.clone())).map(|result| {
                let AnalysisResult::CohesiveInterface2d(result) = result else {
                    unreachable!("cohesive interface 2d solve should return matching result")
                };
                let max_separation = result
                    .steps
                    .iter()
                    .map(|step| step.local_separation[0].hypot(step.local_separation[1]))
                    .fold(0.0_f64, f64::max);
                let mut metrics = WorkloadMetrics::from_counts(
                    result.input.nodes.len(),
                    1,
                    result.input.nodes.len() * 2,
                    max_separation,
                    result.max_resultant_traction,
                );
                metrics.history_step_count = Some(result.steps.len());
                metrics
            })
        }
        BenchmarkWorkload::CohesiveInterfaceMesh2d(request) => {
            solve(EngineSolveRequest::CohesiveInterfaceMesh2d(request.clone())).and_then(|result| {
                let AnalysisResult::CohesiveInterfaceMesh2d(result) = result else {
                    unreachable!("cohesive mesh 2d solve should return matching result")
                };
                ensure_converged_2d(&result)?;
                let max_traction = result
                    .steps
                    .iter()
                    .map(|step| step.max_resultant_traction)
                    .fold(0.0_f64, f64::max);
                let mut metrics = WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 2,
                    result.max_displacement,
                    max_traction,
                );
                metrics.solver_iterations =
                    Some(result.steps.iter().map(|step| step.iterations).sum());
                metrics.history_step_count = Some(result.steps.len());
                metrics.solver_matrix_non_zero_count = Some(result.max_tangent_non_zero_count);
                metrics.solver_residual_norm = Some(result.residual_norm);
                Ok(metrics)
            })
        }
        BenchmarkWorkload::CohesiveInterfaceMesh3d(request) => {
            solve(EngineSolveRequest::CohesiveInterfaceMesh3d(request.clone())).and_then(|result| {
                let AnalysisResult::CohesiveInterfaceMesh3d(result) = result else {
                    unreachable!("cohesive mesh 3d solve should return matching result")
                };
                ensure_converged_3d(&result)?;
                let mut metrics = WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len() * 3,
                    result.max_displacement,
                    result.max_resultant_traction,
                );
                metrics.solver_iterations =
                    Some(result.steps.iter().map(|step| step.iterations).sum());
                metrics.history_step_count = Some(result.steps.len());
                metrics.solver_matrix_non_zero_count = Some(result.max_tangent_non_zero_count);
                metrics.solver_residual_norm = Some(result.residual_norm);
                Ok(metrics)
            })
        }
        _ => return None,
    };

    Some(result)
}

fn ensure_converged_2d(
    result: &kyuubiki_protocol::SolveCohesiveInterfaceMesh2dResult,
) -> Result<(), String> {
    if result.converged {
        Ok(())
    } else {
        Err(format!(
            "cohesive interface mesh 2d benchmark did not converge: {}",
            result.failure_reason.as_deref().unwrap_or("unknown reason")
        ))
    }
}

fn ensure_converged_3d(
    result: &kyuubiki_protocol::SolveCohesiveInterfaceMesh3dResult,
) -> Result<(), String> {
    if result.converged {
        Ok(())
    } else {
        Err(format!(
            "cohesive interface mesh 3d benchmark did not converge: {}",
            result.failure_reason.as_deref().unwrap_or("unknown reason")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::run_interface_workload;
    use crate::generators_interface::{
        generate_cohesive_interface_1d_case, generate_cohesive_interface_2d_case,
        generate_cohesive_interface_mesh_2d_case, generate_cohesive_interface_mesh_3d_case,
    };
    use crate::models::{BenchmarkCase, BenchmarkWorkload};

    #[test]
    fn interface_workloads_execute_through_engine_and_workflow_contracts() {
        for (index, workload) in interface_workloads().into_iter().enumerate() {
            let metrics = run_interface_workload(&workload)
                .expect("interface workload should be recognized")
                .expect("interface workload should solve");
            assert!(metrics.node_count > 0);
            assert!(metrics.element_count > 0);
            assert!(metrics.history_step_count.is_some_and(|steps| steps > 0));
            assert!(metrics.max_displacement.is_finite());
            assert!(metrics.max_stress.is_finite());

            let case = BenchmarkCase {
                id: format!("interface-{index}"),
                family: "interface_test",
                workload,
            };
            let (operator_id, payload) = crate::workflow_payloads::workflow_payload_for_case(&case);
            let result = kyuubiki_engine::run_solve_operator(operator_id, payload)
                .unwrap_or_else(|error| panic!("{operator_id} workflow solve failed: {error}"));
            if operator_id == "solve.cohesive_interface_2d" {
                assert!(result["max_normal_damage"].as_f64().unwrap_or_default() > 0.0);
                assert!(result["max_shear_damage"].as_f64().unwrap_or_default() > 0.0);
            }
        }
    }

    fn interface_workloads() -> [BenchmarkWorkload; 4] {
        [
            BenchmarkWorkload::CohesiveInterface1d(generate_cohesive_interface_1d_case(8)),
            BenchmarkWorkload::CohesiveInterface2d(generate_cohesive_interface_2d_case(8)),
            BenchmarkWorkload::CohesiveInterfaceMesh2d(generate_cohesive_interface_mesh_2d_case(8)),
            BenchmarkWorkload::CohesiveInterfaceMesh3d(generate_cohesive_interface_mesh_3d_case(
                12,
            )),
        ]
    }
}
