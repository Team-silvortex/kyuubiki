use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_protocol::AnalysisResult;

use crate::models::BenchmarkWorkload;
use crate::runner_structural::WorkloadMetrics;

pub(crate) fn run_dynamic_workload(
    workload: &BenchmarkWorkload,
) -> Option<Result<WorkloadMetrics, String>> {
    let result = match workload {
        BenchmarkWorkload::TransientHeatBar1d(request) => {
            solve(EngineSolveRequest::TransientHeatBar1d(request.clone())).map(|result| {
                let AnalysisResult::TransientHeatBar1d(result) = result else {
                    unreachable!("transient heat solve should return transient heat result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len(),
                    result.max_temperature,
                    result.max_heat_flux,
                )
            })
        }
        BenchmarkWorkload::TransientSpring1d(request) => {
            solve(EngineSolveRequest::TransientSpring1d(request.clone())).map(|result| {
                let AnalysisResult::TransientSpring1d(result) = result else {
                    unreachable!("transient spring solve should return transient spring result")
                };
                WorkloadMetrics::from_counts(
                    result.nodes.len(),
                    result.elements.len(),
                    result.nodes.len(),
                    result.max_displacement,
                    result.max_force,
                )
            })
        }
        BenchmarkWorkload::HarmonicSpring1d(request) => {
            solve(EngineSolveRequest::HarmonicSpring1d(request.clone())).map(|result| {
                let AnalysisResult::HarmonicSpring1d(result) = result else {
                    unreachable!("harmonic spring solve should return harmonic spring result")
                };
                WorkloadMetrics::from_counts(
                    result.input.nodes.len(),
                    result.input.elements.len(),
                    result.input.nodes.len(),
                    result.max_displacement,
                    result.max_force,
                )
            })
        }
        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::run_dynamic_workload;
    use crate::generators_dynamic::{
        generate_harmonic_spring_1d_case, generate_transient_heat_bar_case,
        generate_transient_spring_1d_case,
    };
    use crate::models::{BenchmarkCase, BenchmarkWorkload};

    #[test]
    fn dynamic_workloads_execute_through_the_engine() {
        for workload in dynamic_workloads() {
            let metrics = run_dynamic_workload(&workload)
                .expect("dynamic workload should be recognized")
                .expect("dynamic workload should solve");
            assert_eq!(metrics.node_count, 9);
            assert_eq!(metrics.element_count, 8);
            assert!(metrics.max_displacement.is_finite());
            assert!(metrics.max_stress.is_finite());
        }
    }

    #[test]
    fn dynamic_workloads_execute_through_the_workflow_contract() {
        for (index, workload) in dynamic_workloads().into_iter().enumerate() {
            let case = BenchmarkCase {
                id: format!("dynamic-{index}"),
                family: "dynamic_test",
                workload,
            };
            let (operator_id, payload) = crate::workflow_payloads::workflow_payload_for_case(&case);

            kyuubiki_engine::run_solve_operator(operator_id, payload)
                .unwrap_or_else(|error| panic!("{operator_id} workflow solve failed: {error}"));
        }
    }

    fn dynamic_workloads() -> [BenchmarkWorkload; 3] {
        [
            BenchmarkWorkload::TransientHeatBar1d(generate_transient_heat_bar_case(8)),
            BenchmarkWorkload::TransientSpring1d(generate_transient_spring_1d_case(8)),
            BenchmarkWorkload::HarmonicSpring1d(generate_harmonic_spring_1d_case(8)),
        ]
    }
}
