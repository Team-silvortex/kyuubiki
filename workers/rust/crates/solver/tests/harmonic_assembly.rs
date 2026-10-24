use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model(dense: bool, damping: f64, sign: f64, reversed: bool) -> SolveHarmonicSpring1dRequest {
    let hard = 2.0_f64.powi(55);
    let mut value = json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"tip","x":1.0,"fix_x":false,"mass":hard,"load_x":sign}
        ],"elements":[
            {"id":"soft","node_i":0,"node_j":1,"stiffness":1.0,"damping":damping},
            {"id":"hard","node_i":0,"node_j":1,"stiffness":hard}
        ],"frequencies_hz":[1.0 / std::f64::consts::TAU]
    });
    if dense {
        for node in 2..=4 {
            value["nodes"].as_array_mut().unwrap().push(json!({
                "id":format!("control-{node}"),"x":node as f64,"fix_x":false,"mass":1.0,"load_x":0.0
            }));
            value["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("ground-{node}"),"node_i":0,"node_j":node,"stiffness":4.0
            }));
        }
        for (i, j) in [(2, 3), (3, 4), (4, 2)] {
            value["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("cycle-{i}-{j}"),"node_i":i,"node_j":j,"stiffness":1.0
            }));
        }
    }
    let mut input: SolveHarmonicSpring1dRequest = serde_json::from_value(value).unwrap();
    if reversed {
        input.elements.reverse();
        for element in &mut input.elements {
            std::mem::swap(&mut element.node_i, &mut element.node_j);
        }
    }
    input
}

#[test]
fn harmonic_assembly_public_damped_oscillator_matches_the_independent_dynamic_diagonal() {
    for dense in [false, true] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                let input = model(dense, 2.0, sign, reversed);
                let result = solve_harmonic_spring_1d(&input).unwrap();
                let response = &result.frequencies[0].nodes[1];
                // omega = 1; (-2^55 + 1 + 2^55) + 2i = 1 + 2i.
                let expected = (0.2_f64).hypot(0.4);
                assert!(
                    (response.displacement_amplitude / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                    "dense={dense}, reversed={reversed}: {response:?} != {expected}"
                );
                let phase = (-sign * 0.4_f64).atan2(sign * 0.2).to_degrees();
                assert!((response.displacement_phase_deg - phase).abs() <= 1e-12);
                assert_eq!(
                    serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                    serde_json::to_value(result).unwrap()
                );
            }
        }
    }
}

#[test]
fn harmonic_assembly_public_binary_scaled_oscillators_preserve_response_and_member_force() {
    for dense in [false, true] {
        for power in [-900, -400, 0, 400, 900] {
            let scale = 2.0_f64.powi(power);
            let mut input = model(dense, 2.0, 1.0, false);
            for node in &mut input.nodes {
                node.mass *= scale;
                node.load_x *= scale;
            }
            for element in &mut input.elements {
                element.stiffness *= scale;
                element.damping *= scale;
            }
            let result = solve_harmonic_spring_1d(&input).unwrap();
            let frequency = &result.frequencies[0];
            let expected = 0.2_f64.hypot(0.4);
            assert!(
                (frequency.nodes[1].displacement_amplitude / expected - 1.0).abs()
                    <= 4.0 * f64::EPSILON
            );
            assert!(
                (frequency.nodes[1].velocity_amplitude / expected - 1.0).abs()
                    <= 4.0 * f64::EPSILON
            );
            assert!(
                (frequency.nodes[1].acceleration_amplitude / expected - 1.0).abs()
                    <= 4.0 * f64::EPSILON
            );
            let hard = frequency.elements.iter().find(|e| e.id == "hard").unwrap();
            let coefficient = 2.0_f64.powi(55) * scale;
            let force = (0.2 * coefficient).hypot(0.4 * coefficient);
            assert!((hard.force_amplitude / force - 1.0).abs() <= 4.0 * f64::EPSILON);
        }
    }
}

#[test]
fn harmonic_assembly_public_true_singularity_and_overflow_still_fail_closed() {
    for dense in [false, true] {
        let mut singular = model(dense, 0.0, 1.0, false);
        singular.elements.remove(0);
        assert!(
            solve_harmonic_spring_1d(&singular)
                .unwrap_err()
                .contains("singular")
        );
        let mut overflow = model(dense, 2.0, 1.0, false);
        overflow.frequencies_hz = vec![0.0];
        overflow.elements[0].stiffness = f64::MAX;
        overflow.elements[1].stiffness = f64::MAX;
        let error = solve_harmonic_spring_1d(&overflow).unwrap_err();
        assert!(
            error.contains("assembly") && error.contains("range"),
            "{error}"
        );
    }
}

#[test]
fn harmonic_assembly_public_undamped_oscillator_is_not_falsely_singular() {
    for dense in [false, true] {
        for reversed in [false, true] {
            let result = solve_harmonic_spring_1d(&model(dense, 0.0, 1.0, reversed)).unwrap();
            assert_eq!(result.frequencies[0].nodes[1].displacement_amplitude, 1.0);
        }
    }
}

#[test]
fn harmonic_assembly_public_cancelled_and_late_failed_sweeps_leave_inputs_reusable() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    for dense in [false, true] {
        let input = model(dense, 2.0, 1.0, false);
        let before = serde_json::to_vec(&input).unwrap();
        let clean = serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap();
        for offset in [1, 2, 3] {
            let boundary = input.elements.len() as u64 + offset;
            let control = SolverControl::default();
            let cancel = control.clone();
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == SolverStage::ElementAssembly && p.completed_steps == boundary {
                        cancel.request_cancel();
                    }
                },
                || solve_harmonic_spring_1d(&input),
            )
            .unwrap_err();
            assert!(
                control.was_interrupted() && error.contains("cancelled"),
                "{error}"
            );
            assert_eq!(serde_json::to_vec(&input).unwrap(), before);
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap(),
                clean
            );
        }
        let mut late = input.clone();
        late.frequencies_hz.push(1e200);
        let error = solve_harmonic_spring_1d(&late).unwrap_err();
        assert!(
            error.contains("frequency 1") && error.contains("range"),
            "{error}"
        );
        assert_eq!(solve_harmonic_spring_1d_owned(late).unwrap_err(), error);
        assert_eq!(
            serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap(),
            clean
        );
    }
}
