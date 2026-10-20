use crate::generators_structural::generate_nonlinear_spring_1d_case;
use crate::models::{BenchmarkCase, BenchmarkWorkload};
use crate::runner::run_case;
use kyuubiki_protocol::{ContactGap1dContactInput, SolveContactGap1dRequest};

fn case(contact: bool, iterations: usize) -> BenchmarkCase {
    scaled_case(contact, iterations, 1.0, 1.0)
}

fn scaled_case(contact: bool, iterations: usize, unit: f64, force: f64) -> BenchmarkCase {
    let mut request = generate_nonlinear_spring_1d_case(1);
    request.elements[0].stiffness = force;
    request.elements[0].cubic_stiffness = if contact { 0.0 } else { force / (unit * unit) };
    request.nodes[1].load_x = if contact { 2.0 } else { 3.0 } * unit * force;
    request.load_steps = Some(if contact { 4 } else { 1 });
    request.max_iterations = Some(iterations);
    request.tolerance = Some(1e-12 * unit * force);
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
                gap: unit,
                normal_stiffness: 10.0 * force,
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
            error.contains(if contact {
                "residual=0.1875"
            } else {
                "residual=1.875"
            }),
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

#[test]
fn small_scale_benchmarks_do_not_count_relaxed_tolerance_as_success() {
    let unit = 2.0_f64.powi(-50);
    for contact in [false, true] {
        let failed = run_case(&scaled_case(contact, 1, unit, 1.0), 2);
        assert!(!failed.ok);
        assert!(failed.error.unwrap().contains("did not converge"));
        let replay = run_case(&scaled_case(contact, 32, unit, 1.0), 2);
        assert!(replay.ok, "{:?}", replay.error);
        assert!(replay.max_displacement > 0.0);
        assert!(replay.solver_iterations.unwrap() > 0);
        assert!(replay.solver_residual_norm.unwrap() <= 1e-12 * unit);
    }
}

#[test]
fn force_scaled_spring_benchmarks_keep_the_same_displacement() {
    for contact in [false, true] {
        let baseline = run_case(&case(contact, 32), 1);
        assert!(baseline.ok, "{:?}", baseline.error);
        for force in [1e-100, 1e-20, 1e100] {
            let result = run_case(&scaled_case(contact, 32, 1.0, force), 1);
            assert!(result.ok, "{:?}", result.error);
            assert!((result.max_displacement / baseline.max_displacement - 1.0).abs() < 1e-10);
            assert!(result.solver_residual_norm.unwrap() <= 1e-12 * force);
        }
    }
}
