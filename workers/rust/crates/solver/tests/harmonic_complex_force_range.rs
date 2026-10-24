use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;
use std::f64::consts::TAU;

fn coefficient_loss_model(dense: bool, hard: f64, soft: f64) -> SolveHarmonicSpring1dRequest {
    let mut input = json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"hard", "x":1.0, "fix_x":false, "mass":1.0, "load_x":0.0}
        ], "elements":[
            {"id":"hard-ground", "node_i":0, "node_j":1, "stiffness":hard}
        ], "frequencies_hz":[0.0]
    });
    for node in 2..=if dense { 3 } else { 2 } {
        input["nodes"].as_array_mut().unwrap().push(json!({
            "id":format!("soft{node}"), "x":node as f64, "fix_x":false, "mass":1.0, "load_x":2.0
        }));
        input["elements"].as_array_mut().unwrap().extend([
            json!({"id":format!("ground{node}"), "node_i":0, "node_j":node, "stiffness":soft}),
            json!({"id":format!("link{node}"), "node_i":1, "node_j":node, "stiffness":soft}),
        ]);
    }
    if dense {
        input["elements"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"cycle", "node_i":2, "node_j":3, "stiffness":soft}));
    }
    serde_json::from_value(input).unwrap()
}

#[test]
fn harmonic_coefficient_range_public_operator_rejects_erased_coefficients_before_factorization() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    for dense in [false, true] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                let mut input = coefficient_loss_model(dense, 1e308, 1e-308);
                for node in input.nodes.iter_mut().skip(2) {
                    node.load_x *= sign;
                }
                if reversed {
                    for element in &mut input.elements {
                        std::mem::swap(&mut element.node_i, &mut element.node_j);
                    }
                }
                let entered = Rc::new(Cell::new(false));
                let observed = Rc::clone(&entered);
                let error = with_solver_observer(
                    &SolverControl::default(),
                    move |point| {
                        if matches!(
                            point.stage,
                            SolverStage::TridiagonalFactor
                                | SolverStage::DenseFactor
                                | SolverStage::ConstraintValidate
                        ) {
                            observed.set(true);
                        }
                    },
                    || solve_harmonic_spring_1d(&input),
                )
                .unwrap_err();
                assert!(
                    !entered.get(),
                    "coefficient loss reached elimination: {error}"
                );
                assert!(
                    error.contains("frequency 0")
                        && error.contains("row 0 column 1")
                        && error
                            .contains("real coefficient normalization would lose numerical range"),
                    "{error}"
                );
                assert_eq!(solve_harmonic_spring_1d_owned(input).unwrap_err(), error);
            }
        }
        let corrected = coefficient_loss_model(dense, 1e4, 1.0);
        let result = solve_harmonic_spring_1d(&corrected).unwrap();
        let count = if dense { 2.0 } else { 1.0 };
        let expected = count / (1e4 + count / 2.0);
        assert!(
            (result.frequencies[0].nodes[1].displacement_amplitude / expected - 1.0).abs() < 1e-12
        );
    }
}

#[test]
fn harmonic_coefficient_range_late_frequency_loss_returns_no_partial_sweep() {
    let mut input = quantum_force_model();
    input.nodes[1].load_x = 1.0;
    input.nodes[1].mass = f64::from_bits(1);
    input.elements[0].stiffness = 1e-308;
    input.elements[0].damping = 1e308;
    input.frequencies_hz = vec![0.0];
    assert!(solve_harmonic_spring_1d(&input).is_ok());
    input.frequencies_hz.push(1.0 / TAU);
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("frequency 1")
            && error.contains("row 0 column 0")
            && error.contains("real coefficient normalization would lose numerical range"),
        "{error}"
    );
    assert_eq!(
        solve_harmonic_spring_1d_owned(input.clone()).unwrap_err(),
        error
    );
    input.elements[0].damping = 0.0;
    assert_eq!(
        solve_harmonic_spring_1d(&input).unwrap().frequencies.len(),
        2
    );
}

fn quantum_force_model() -> SolveHarmonicSpring1dRequest {
    serde_json::from_value(json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"tip", "x":1.0, "fix_x":false, "mass":2.0_f64.powi(-700), "load_x":f64::from_bits(1)}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1, "stiffness":2.0_f64.powi(-600), "damping":2.0_f64.powi(-600)}],
        "frequencies_hz":[1.0/TAU]
    })).unwrap()
}

fn wide_rhs_model(dense: bool) -> SolveHarmonicSpring1dRequest {
    let mut input = json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0}
        ], "elements":[], "frequencies_hz":[1.0/TAU]
    });
    let size = if dense { 3 } else { 1 };
    for index in 1..=size {
        input["nodes"].as_array_mut().unwrap().push(json!({
            "id":format!("tip{index}"), "x":index as f64, "fix_x":false,
            "mass":f64::from_bits(4), "load_x":2.0
        }));
        input["elements"].as_array_mut().unwrap().push(json!({
            "id":format!("ground{index}"), "node_i":0, "node_j":index,
            "stiffness":1e-308, "damping":1e-308
        }));
    }
    if dense {
        for (first, second) in [(1, 2), (2, 3), (3, 1)] {
            input["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("link{first}"), "node_i":first, "node_j":second,
                "stiffness":1e-308 * 2.0_f64.powi(-20), "damping":0.0
            }));
        }
    }
    serde_json::from_value(input).unwrap()
}

#[test]
fn harmonic_complex_rhs_range_public_solve_recovers_finite_displacement_and_force() {
    for dense in [false, true] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                let mut input = wide_rhs_model(dense);
                for node in input.nodes.iter_mut().skip(1) {
                    node.load_x *= sign;
                }
                if reversed {
                    for element in &mut input.elements {
                        std::mem::swap(&mut element.node_i, &mut element.node_j);
                    }
                }
                let result = solve_harmonic_spring_1d(&input).unwrap();
                assert!((result.max_displacement / (1e308 * 2.0_f64.sqrt()) - 1.0).abs() < 1e-12);
                assert!((result.max_force / 2.0 - 1.0).abs() < 1e-12);
                let frequency = &result.frequencies[0];
                for node in frequency.nodes.iter().skip(1) {
                    let phase = node.displacement_phase_deg.to_radians();
                    let re = node.displacement_amplitude / 1e308 * phase.cos();
                    let im = node.displacement_amplitude / 1e308 * phase.sin();
                    assert!((re - sign).abs() < 1e-12 && (im + sign).abs() < 1e-12);
                    assert!((re - im - sign * 2.0).abs() < 1e-12);
                    assert!((re + im).abs() < 1e-12);
                }
                for element in &frequency.elements {
                    if element.node_i == 0 || element.node_j == 0 {
                        assert!((element.force_amplitude - 2.0).abs() < 1e-12);
                    } else {
                        assert!(element.force_amplitude < 1e-12);
                    }
                }
                assert_eq!(
                    serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                    serde_json::to_value(result).unwrap()
                );
            }
        }
    }
}

#[test]
fn harmonic_complex_rhs_range_recovered_rhs_does_not_allow_unrepresentable_output() {
    for dense in [false, true] {
        for load in [3.0, 4.0] {
            let mut input = wide_rhs_model(dense);
            for node in input.nodes.iter_mut().skip(1) {
                node.load_x = load;
            }
            let error = solve_harmonic_spring_1d(&input).unwrap_err();
            assert!(error.contains("frequency 0"), "{error}");
            if load == 3.0 {
                assert!(error.contains("displacement amplitude"), "{error}");
            } else {
                assert!(error.contains("representable range"), "{error}");
            }
        }
        let mut late = wide_rhs_model(dense);
        for node in late.nodes.iter_mut().skip(1) {
            node.load_x = 2.05;
        }
        late.frequencies_hz.push(0.5 / TAU);
        let error = solve_harmonic_spring_1d(&late).unwrap_err();
        assert!(
            error.contains("frequency 1") && error.contains("displacement amplitude"),
            "{error}"
        );
        assert!(solve_harmonic_spring_1d(&wide_rhs_model(dense)).is_ok());
    }
}

#[test]
fn harmonic_complex_force_range_publishes_a_nonzero_quantum_force() {
    for sign in [-1.0, 1.0] {
        for reversed in [false, true] {
            let mut input = quantum_force_model();
            input.nodes[1].load_x *= sign;
            if reversed {
                input.elements[0].node_i = 1;
                input.elements[0].node_j = 0;
            }
            let result = solve_harmonic_spring_1d(&input).unwrap();
            assert_eq!(result.max_force, f64::from_bits(1));
            assert_eq!(
                result.frequencies[0].elements[0].force_amplitude,
                f64::from_bits(1)
            );
            let phase = if sign > 0.0 { -45.0 } else { 135.0 };
            assert!((result.frequencies[0].nodes[1].displacement_phase_deg - phase).abs() < 1e-12);
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                serde_json::to_value(result).unwrap()
            );
        }
    }
}

#[test]
fn harmonic_complex_force_range_rejects_unrepresentable_per_element_forces() {
    let mut input = quantum_force_model();
    input.elements = (0..4)
        .map(|index| {
            let mut element = input.elements[0].clone();
            element.id = format!("parallel{index}");
            element
        })
        .collect();
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("element 0 force") && error.contains("numerical range"),
        "{error}"
    );
    assert_eq!(
        solve_harmonic_spring_1d(&quantum_force_model())
            .unwrap()
            .max_force,
        f64::from_bits(1)
    );
}

#[test]
fn harmonic_complex_force_range_reports_unrepresentable_extension_separately() {
    let input: SolveHarmonicSpring1dRequest = serde_json::from_value(json!({
        "nodes":[
            {"id":"root", "x":0.0, "fix_x":true, "mass":1.0, "load_x":0.0},
            {"id":"left", "x":1.0, "fix_x":false, "mass":1.0, "load_x":4.0},
            {"id":"right", "x":2.0, "fix_x":false, "mass":1.0, "load_x":-4.0}
        ], "elements":[
            {"id":"left-ground", "node_i":0, "node_j":1, "stiffness":2e-308},
            {"id":"right-ground", "node_i":0, "node_j":2, "stiffness":2e-308},
            {"id":"link", "node_i":1, "node_j":2, "stiffness":1e-308}
        ], "frequencies_hz":[0.0]
    }))
    .unwrap();
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("element 2 extension amplitude") && error.contains("finite numeric range"),
        "{error}"
    );
}
