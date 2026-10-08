use kyuubiki_protocol::{
    AdvectionDiffusionBar1dElementInput as Element, AdvectionDiffusionBar1dNodeInput as Node,
    SolveAdvectionDiffusionBar1dRequest as Request,
};
use kyuubiki_solver::{
    solve_advection_diffusion_bar_1d as solve,
    solver_control::{SolverControl, SolverStage, with_solver_observer},
};

fn single(left: f64, right: f64, length: f64, diffusivity: f64, velocity: f64) -> Request {
    Request {
        scheme: Default::default(),
        nodes: vec![node(0, 0.0, left, true), node(1, length, right, true)],
        elements: vec![element(0, 0, 1, diffusivity, velocity)],
    }
}

fn node(index: usize, x: f64, concentration: f64, fixed: bool) -> Node {
    Node {
        id: format!("n{index}"),
        x,
        concentration,
        fix_concentration: fixed,
        source: 0.0,
    }
}

fn element(index: usize, node_i: usize, node_j: usize, diffusivity: f64, velocity: f64) -> Element {
    Element {
        id: format!("e{index}"),
        node_i,
        node_j,
        diffusivity,
        velocity,
        area: 1.0,
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite(), "non-finite output: {actual}");
    if expected == 0.0 {
        assert_eq!(actual, 0.0);
    } else {
        assert!(
            (actual / expected - 1.0).abs() < 1e-12,
            "{actual} != {expected}"
        );
    }
}

fn assert_orientation_invariant(mut request: Request) {
    let original = solve(&request).unwrap();
    for index in 0..request.elements.len() {
        let e = &mut request.elements[index];
        std::mem::swap(&mut e.node_i, &mut e.node_j);
        let reversed = solve(&request).unwrap();
        for (a, b) in original.nodes.iter().zip(&reversed.nodes) {
            close(b.concentration, a.concentration);
        }
        for (a, b) in original.elements.iter().zip(&reversed.elements) {
            close(b.concentration_gradient, a.concentration_gradient);
            close(b.diffusive_flux, a.diffusive_flux);
            close(b.advective_flux, a.advective_flux);
            close(b.total_flux, a.total_flux);
            close(b.peclet_number, a.peclet_number);
        }
    }
}

#[test]
fn fixed_element_flux_uses_global_x_not_connectivity_direction() {
    assert_orientation_invariant(single(2.0, 4.0, 1.0, 1.0, 0.5));
}

#[test]
fn free_concentration_is_orientation_invariant_on_path_and_dense_cycle() {
    for dense in [false, true] {
        let mut request = Request {
            scheme: Default::default(),
            nodes: vec![
                node(0, 0.0, 2.0, true),
                node(1, 0.5, 0.0, false),
                node(2, 1.0, 4.0, true),
            ],
            elements: vec![element(0, 0, 1, 1.0, 0.5), element(1, 1, 2, 1.0, 0.5)],
        };
        request.nodes[1].source = 4.0;
        if dense {
            request.elements.push(element(2, 0, 2, 1.0, -0.25));
        }
        close(solve(&request).unwrap().nodes[1].concentration, 3.875);
        assert_orientation_invariant(request);
    }
}

#[test]
fn representable_large_average_and_small_advective_flux_survive() {
    let result = solve(&single(1e308, 1e308, 1.0, 1.0, 1e-308)).unwrap();
    close(result.elements[0].average_concentration, 1e308);
    close(result.elements[0].total_flux, 1.0);
    assert!(!serde_json::to_string(&result).unwrap().contains("null"));
}

#[test]
fn representable_gradient_survives_overflowing_concentration_difference() {
    let result = solve(&single(-1e308, 1e308, 2.0, 1e-308, 0.0)).unwrap();
    close(result.elements[0].concentration_gradient, 1e308);
    close(result.elements[0].diffusive_flux, -1.0);
}

#[test]
fn peclet_denominator_does_not_overflow_or_underflow_early() {
    for (length, diffusivity, velocity) in [(1.0, 1e308, 1e308), (1e308, 1e-308, 1e-308)] {
        let mut request = single(0.0, 0.0, length, diffusivity, velocity);
        if length > 1.0 {
            request.elements[0].area = 1e308;
        }
        let result = solve(&request).unwrap();
        close(result.elements[0].peclet_number, length * 0.5);
    }
}

#[test]
fn diffusion_coefficient_avoids_intermediate_overflow() {
    let mut request = single(0.0, 0.0, 1e308, 1e308, 0.0);
    request.elements[0].area = 1e308;
    assert!(solve(&request).is_ok());
}

#[test]
fn half_advection_coefficient_avoids_intermediate_overflow() {
    let mut request = single(0.0, 0.0, 1.0, 1.0, 1e308);
    request.elements[0].area = 2.0;
    close(solve(&request).unwrap().max_peclet_number, 5e307);
}

#[test]
fn unrepresentable_flux_and_peclet_are_errors_not_successful_nulls() {
    for (request, field) in [
        (single(1e308, 1e308, 1.0, 1.0, 2.0), "advective flux"),
        (single(1.5e308, 0.5e308, 1.0, 1.0, 1.0), "total flux"),
        (single(1.0, -1.0, 1.0, 1e-308, 10.0), "Peclet number"),
    ] {
        let error = solve(&request).unwrap_err();
        assert!(
            error.contains(field) && error.contains("representable"),
            "{error}"
        );
    }
}

#[test]
fn unrepresentable_nonzero_diffusion_cannot_be_silently_zeroed() {
    let mut request = single(1.0, 1.0, 1.0, 1e-200, 0.0);
    request.elements[0].area = 1e-200;
    assert!(
        solve(&request)
            .unwrap_err()
            .contains("diffusion coefficient")
    );
}

#[test]
fn fixed_dense_boundaries_do_not_hide_assembled_matrix_overflow() {
    let mut request = single(0.0, 0.0, 1.0, 1e308, 0.0);
    let mut duplicate = request.elements[0].clone();
    duplicate.id = "duplicate".into();
    request.elements.push(duplicate);
    let error = solve(&request).unwrap_err();
    assert!(
        error.contains("assembled") && error.contains("representable"),
        "{error}"
    );
}

#[test]
fn fixed_dense_rows_do_not_require_discarded_rhs_products() {
    let mut request = single(1e308, 1e308, 1.0, 4.0, 0.0);
    let mut duplicate = request.elements[0].clone();
    duplicate.id = "duplicate".into();
    request.elements.push(duplicate);
    let result = solve(&request).unwrap();
    close(result.max_total_flux, 0.0);
}

#[test]
fn result_generation_can_be_cancelled_and_next_call_recovers() {
    for stage in [
        SolverStage::ResultNodes,
        SolverStage::ResultElements,
        SolverStage::ResultNodeSummary,
        SolverStage::ResultElementSummary,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let request = single(2.0, 4.0, 1.0, 1.0, 0.5);
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage {
                    cancel.request_cancel();
                }
            },
            || solve(&request),
        );
        assert!(result.unwrap_err().starts_with("solver cancelled"));
        assert!(control.was_interrupted());
        close(solve(&request).unwrap().max_total_flux, 0.5);
    }
}

#[test]
fn result_generation_cancels_inside_chunks_and_after_summary_scans() {
    let request = Request {
        scheme: Default::default(),
        nodes: (0..101).map(|i| node(i, i as f64, 2.0, true)).collect(),
        elements: (0..100).map(|i| element(i, i, i + 1, 1.0, 0.5)).collect(),
    };
    for (stage, steps) in [
        (SolverStage::ResultNodes, 64),
        (SolverStage::ResultElements, 64),
        (SolverStage::ResultNodeSummary, 101),
        (SolverStage::ResultElementSummary, 100),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == steps {
                    cancel.request_cancel();
                }
            },
            || solve(&request),
        );
        assert!(result.unwrap_err().starts_with("solver cancelled"));
        assert_eq!(control.last_checkpoint().unwrap().completed_steps, steps);
        close(solve(&request).unwrap().max_total_flux, 1.0);
    }
}

#[test]
fn binary_scaled_free_equations_preserve_normal_and_subnormal_coefficients() {
    for (d, a, l) in [
        (700, 700, 700),
        (-700, -350, -30),
        (-700, 700, 300),
        (-1000, 0, 50),
    ] {
        let mut request = single(0.0, 0.0, 2.0_f64.powi(l), 2.0_f64.powi(d), 0.0);
        request.elements[0].area = 2.0_f64.powi(a);
        request.nodes[1].fix_concentration = false;
        // Avoid the test fixture itself overflowing an inverse powi intermediate.
        request.nodes[1].source = binary_scale(d + a - l);
        let result = solve(&request).unwrap();
        close(result.nodes[1].concentration, 1.0);
        close(result.elements[0].diffusive_flux, -binary_scale(d - l));
    }
}

fn binary_scale(exponent: i32) -> f64 {
    assert!((-1074..=1023).contains(&exponent));
    if exponent >= -1022 {
        f64::from_bits(((exponent + 1023) as u64) << 52)
    } else {
        f64::from_bits(1_u64 << (exponent + 1074))
    }
}

#[test]
fn constant_fields_preserve_largest_and_smallest_finite_concentrations() {
    for concentration in [f64::MAX, f64::MIN_POSITIVE, f64::from_bits(1)] {
        let result = solve(&single(concentration, concentration, 1.0, 1.0, 0.0)).unwrap();
        assert_eq!(result.elements[0].average_concentration, concentration);
        close(result.elements[0].total_flux, 0.0);
    }
}

#[test]
fn nonzero_output_underflow_is_explicit_instead_of_false_zero() {
    let mut peclet = single(0.0, 0.0, 1.0, 1.0, f64::from_bits(1));
    peclet.elements[0].area = 2.0;
    for (request, field) in [
        (single(1e-308, 1e-308, 1.0, 1.0, 1e-308), "advective flux"),
        (
            single(0.0, f64::from_bits(1), 2.0, 1.0, 0.0),
            "concentration gradient",
        ),
        (peclet, "Peclet number"),
    ] {
        let error = solve(&request).unwrap_err();
        assert!(
            error.contains(field) && error.contains("underflow"),
            "{error}"
        );
    }
}
