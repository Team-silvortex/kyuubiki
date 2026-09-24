use crate::generators_structural::generate_nonlinear_spring_1d_case;
use crate::models::{BenchmarkCase, BenchmarkWorkload};
use crate::runner::run_case;
use kyuubiki_protocol::{ContactGap1dContactInput, SolveContactGap1dRequest};

fn case(contact: bool, iterations: usize) -> BenchmarkCase {
    let mut request = generate_nonlinear_spring_1d_case(1);
    request.elements[0].stiffness = 1.0;
    request.elements[0].cubic_stiffness = if contact { 0.0 } else { 1.0 };
    request.nodes[1].load_x = 2.0;
    request.load_steps = Some(if contact { 4 } else { 1 });
    request.max_iterations = Some(iterations);
    request.tolerance = Some(1e-12);
    let workload = if contact {
        BenchmarkWorkload::ContactGap1d(SolveContactGap1dRequest {
            nodes: request.nodes,
            elements: request.elements,
            load_steps: request.load_steps,
            max_iterations: request.max_iterations,
            tolerance: request.tolerance,
            contacts: vec![ContactGap1dContactInput {
                id: "stop".into(),
                node: 1,
                gap: 1.0,
                normal_stiffness: 10.0,
            }],
        })
    } else {
        BenchmarkWorkload::NonlinearSpring1d(request)
    };
    BenchmarkCase {
        id: "spring-recovery".into(),
        family: "structural",
        workload,
    }
}

#[test]
fn nonconverged_spring_benchmarks_are_failures_not_fast_successes() {
    for contact in [false, true] {
        let result = run_case(&case(contact, 1), 2);
        assert!(!result.ok);
        let error = result.error.unwrap();
        assert!(error.contains("did not converge"), "{error}");
        assert!(
            error.contains(if contact {
                "achieved=0.5"
            } else {
                "achieved=0"
            }),
            "{error}"
        );
        assert!(
            error.contains(if contact { "residual=5" } else { "residual=8" }),
            "{error}"
        );
    }
}

#[test]
fn sufficient_budget_benchmarks_remain_successful_after_a_failed_run() {
    for contact in [false, true] {
        let _ = run_case(&case(contact, 1), 1);
        let result = run_case(&case(contact, 32), 2);
        assert!(result.ok, "{:?}", result.error);
        assert!(result.max_displacement > 0.0);
        assert!(result.solver_iterations.unwrap() > 0);
        assert!(result.solver_residual_norm.unwrap() <= 1e-12);
    }
}
