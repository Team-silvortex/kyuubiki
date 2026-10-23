use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;
use std::f64::consts::TAU;

fn model(power: i32, dense: bool, damped: bool) -> SolveHarmonicSpring1dRequest {
    let omega = 2.0_f64.powi(power);
    let inertia = 2.0_f64.powi(power / 2);
    let mut elements = (1..=3)
        .map(|index| {
            json!({
                "id":format!("ground{index}"), "node_i":0, "node_j":index,
                "stiffness":2.0*inertia, "damping":if damped { inertia/omega } else { 0.0 }
            })
        })
        .collect::<Vec<_>>();
    for (index, (first, second)) in [(1, 2), (2, 3), (3, 1)]
        .into_iter()
        .take(if dense { 3 } else { 2 })
        .enumerate()
    {
        elements.push(
            json!({"id":format!("link{index}"), "node_i":first, "node_j":second,
            "stiffness":inertia, "damping":0.0}),
        );
    }
    serde_json::from_value(json!({
        "nodes":(0..=3).map(|index| json!({
            "id":format!("n{index}"), "x":index, "fix_x":index == 0,
            "load_x":if index == 0 { 17.0 } else { 2.0_f64.powi(-power/2) },
            "mass":if index == 0 { 1.0 } else { 2.0_f64.powi(-3*power/2) }
        })).collect::<Vec<_>>(), "elements":elements, "frequencies_hz":[omega/TAU]
    }))
    .unwrap()
}

fn assert_relative(actual: f64, expected: f64) {
    assert!(
        actual.is_finite() && (actual / expected - 1.0).abs() < 1e-12,
        "actual={actual:e}, expected={expected:e}"
    );
}

fn check_extreme_square(power: i32) {
    for dense in [false, true] {
        for damped in [false, true] {
            let input = model(power, dense, damped);
            let result = solve_harmonic_spring_1d(&input).unwrap();
            let attenuation = if damped { 0.5_f64.sqrt() } else { 1.0 };
            let frame = &result.frequencies[0];
            for node in frame.nodes.iter().skip(1) {
                assert_relative(
                    node.displacement_amplitude,
                    attenuation * 2.0_f64.powi(-power),
                );
                assert_relative(node.velocity_amplitude, attenuation);
                assert_relative(
                    node.acceleration_amplitude,
                    attenuation * 2.0_f64.powi(power),
                );
                assert!(
                    (node.displacement_phase_deg - if damped { -45.0 } else { 0.0 }).abs() < 1e-10
                );
            }
            for ground in frame.elements.iter().take(3) {
                assert_relative(
                    ground.force_amplitude,
                    if damped { 2.5_f64.sqrt() } else { 2.0 } * 2.0_f64.powi(-power / 2),
                );
            }
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                serde_json::to_value(result).unwrap()
            );
        }
    }
}

#[test]
fn harmonic_frequency_range_recovers_underflowed_square_inertia_and_acceleration() {
    check_extreme_square(-600);
}

#[test]
fn harmonic_frequency_range_recovers_overflowed_square_inertia_and_acceleration() {
    check_extreme_square(600);
}

#[test]
fn harmonic_frequency_range_does_not_magnify_a_rounded_subnormal_square() {
    let mut input = model(0, false, false);
    let omega = 1.5 * 2.0_f64.powi(-537);
    for node in input.nodes.iter_mut().skip(1) {
        node.mass = 2.0_f64.powi(800);
        node.load_x = 2.25 * 2.0_f64.powi(226);
    }
    for element in &mut input.elements {
        element.stiffness *= 2.25 * 2.0_f64.powi(-274);
    }
    input.frequencies_hz = vec![omega / TAU];
    let result = solve_harmonic_spring_1d(&input).unwrap();
    for node in result.frequencies[0].nodes.iter().skip(1) {
        assert_relative(node.displacement_amplitude, 2.0_f64.powi(500));
        assert_relative(node.acceleration_amplitude, 2.25 * 2.0_f64.powi(-574));
    }
}

#[test]
fn harmonic_frequency_range_rejects_unrepresentable_inertia_and_damping_then_replays() {
    for dense in [false, true] {
        for mass in [1.0, 1.5 * 2.0_f64.powi(126)] {
            let mut input = model(-600, dense, false);
            input.nodes[1].mass = mass;
            let error = solve_harmonic_spring_1d(&input).unwrap_err();
            assert!(
                error.contains("frequency 0")
                    && error.contains("node 1 inertia coefficient")
                    && error.contains("lose numerical range"),
                "{error}"
            );
        }
        for damping in [2.0_f64.powi(-1000), 1.5 * 2.0_f64.powi(-974)] {
            let mut input = model(0, dense, false);
            input.frequencies_hz = vec![2.0_f64.powi(-100) / TAU];
            input.elements[0].damping = damping;
            let error = solve_harmonic_spring_1d(&input).unwrap_err();
            assert!(
                error.contains("frequency 0")
                    && error.contains("element 0 damping coefficient")
                    && error.contains("lose numerical range"),
                "{error}"
            );
        }
        assert!(solve_harmonic_spring_1d(&model(0, dense, true)).is_ok());
    }
}

#[test]
fn harmonic_frequency_range_zero_frequency_and_zero_response_remain_valid() {
    for dense in [false, true] {
        let mut input = model(0, dense, true);
        input.frequencies_hz = vec![0.0];
        for element in &mut input.elements {
            element.damping = f64::MAX;
        }
        let result = solve_harmonic_spring_1d(&input).unwrap();
        assert_relative(result.max_displacement, 0.5);
        assert_eq!(result.max_velocity, 0.0);
        assert_eq!(result.max_acceleration, 0.0);

        let mut input = model(600, dense, true);
        for node in input.nodes.iter_mut().skip(1) {
            node.load_x = 0.0;
        }
        let result = solve_harmonic_spring_1d(&input).unwrap();
        assert_eq!(result.max_displacement, 0.0);
        assert_eq!(result.max_velocity, 0.0);
        assert_eq!(result.max_acceleration, 0.0);
        assert_eq!(result.max_force, 0.0);
    }
}

#[test]
fn harmonic_frequency_range_rejects_unrepresentable_published_acceleration() {
    let mut input = model(0, false, false);
    input.frequencies_hz = vec![2.0_f64.powi(-600) / TAU];
    for node in input.nodes.iter_mut().skip(1) {
        node.mass = 2.0_f64.powi(900);
        node.load_x = 2.0_f64.powi(-300);
    }
    for element in &mut input.elements {
        element.stiffness *= 2.0_f64.powi(-300);
    }
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("node 1 acceleration amplitude") && error.contains("lose numerical range"),
        "{error}"
    );
    assert!(solve_harmonic_spring_1d(&model(0, false, true)).is_ok());
}

#[test]
fn harmonic_frequency_range_rejects_true_coefficient_overflow_with_local_context() {
    for dense in [false, true] {
        let mut input = model(600, dense, false);
        input.nodes[1].mass = 1.0;
        let error = solve_harmonic_spring_1d(&input).unwrap_err();
        assert!(
            error.contains("node 1 inertia coefficient") && error.contains("finite numeric range"),
            "{error}"
        );
        let mut input = model(600, dense, true);
        input.elements[0].damping = f64::MAX;
        let error = solve_harmonic_spring_1d(&input).unwrap_err();
        assert!(
            error.contains("element 0 damping coefficient")
                && error.contains("finite numeric range"),
            "{error}"
        );
        assert!(solve_harmonic_spring_1d(&model(600, dense, true)).is_ok());
    }
}
