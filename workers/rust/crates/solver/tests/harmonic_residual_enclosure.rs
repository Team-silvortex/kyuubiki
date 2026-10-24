use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model(dense: bool, sign: f64, power: i32, reversed: bool) -> SolveHarmonicSpring1dRequest {
    let scale = 2.0_f64.powi(power);
    let mut input = json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":scale,"load_x":0.0},
            {"id":"tip","x":1.0,"fix_x":false,"mass":(1.0+f64::EPSILON)*2.0_f64.powi(-1000)*scale,"load_x":sign*0.5*scale}
        ],"elements":[
            {"id":"spring","node_i":0,"node_j":1,"stiffness":scale,"damping":0.5*scale}
        ],"frequencies_hz":[1.0/std::f64::consts::TAU]
    });
    if dense {
        for node in 2..=4 {
            input["nodes"].as_array_mut().unwrap().push(json!({
                "id":format!("control-{node}"),"x":node as f64,"fix_x":false,"mass":scale,"load_x":0.0
            }));
            input["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("ground-{node}"),"node_i":0,"node_j":node,"stiffness":4.0*scale
            }));
        }
        for (i, j) in [(2, 3), (3, 4), (4, 2)] {
            input["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("cycle-{i}-{j}"),"node_i":i,"node_j":j,"stiffness":scale
            }));
        }
    }
    let mut request: SolveHarmonicSpring1dRequest = serde_json::from_value(input).unwrap();
    if reversed {
        request.elements.reverse();
        for element in &mut request.elements {
            std::mem::swap(&mut element.node_i, &mut element.node_j);
        }
    }
    request
}

#[test]
fn harmonic_residual_enclosure_public_complex_response_matches_the_independent_ratio() {
    for dense in [false, true] {
        for sign in [-1.0, 1.0] {
            for power in [-20, 0, 50] {
                for reversed in [false, true] {
                    let input = model(dense, sign, power, reversed);
                    let result = solve_harmonic_spring_1d(&input).unwrap();
                    let response = &result.frequencies[0].nodes[1];
                    let expected = 0.4_f64.hypot(0.2);
                    assert!(
                        (response.displacement_amplitude / expected - 1.0).abs()
                            <= 4.0 * f64::EPSILON
                    );
                    let phase = (-sign * 0.2_f64).atan2(sign * 0.4).to_degrees();
                    assert!((response.displacement_phase_deg - phase).abs() <= 1e-12);
                    assert!(
                        (result.max_force / (0.5 * 2.0_f64.powi(power)) - 1.0).abs()
                            <= 4.0 * f64::EPSILON
                    );
                    assert_eq!(
                        serde_json::to_value(solve_harmonic_spring_1d_owned(input).unwrap())
                            .unwrap(),
                        serde_json::to_value(result).unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn harmonic_residual_enclosure_public_replay_cancellation_preserves_fresh_request_reuse() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    for dense in [false, true] {
        let input = model(dense, 1.0, 0, false);
        let before = serde_json::to_vec(&input).unwrap();
        let baseline = serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap();
        for mode in 0..3 {
            let control = SolverControl::default();
            let cancel = control.clone();
            let starts = Rc::new(Cell::new(0));
            let events = starts.clone();
            let progress = Rc::new(Cell::new(0));
            let observed = progress.clone();
            let n = input.nodes.len() as u64;
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == SolverStage::HarmonicResidual && p.completed_steps == 0 {
                        events.set(events.get() + 1);
                        if mode == 0 && events.get() == 2 {
                            cancel.request_cancel();
                        }
                    }
                    if mode == 1
                        && events.get() >= 2
                        && p.stage == SolverStage::HarmonicResidual
                        && p.completed_steps == n
                    {
                        observed.set(observed.get() + 1);
                        if observed.get() == 2 {
                            cancel.request_cancel();
                        }
                    }
                    if mode == 2
                        && p.stage == SolverStage::ConstraintValidate
                        && p.completed_steps == 2 * n
                    {
                        cancel.request_cancel();
                    }
                },
                || solve_harmonic_spring_1d(&input),
            )
            .unwrap_err();
            assert!(
                starts.get() >= 2 && control.was_interrupted() && error.contains("cancelled"),
                "{error}"
            );
            assert_eq!(serde_json::to_vec(&input).unwrap(), before);
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap(),
                baseline
            );
        }
    }
}

#[test]
fn harmonic_residual_enclosure_public_late_frequency_failure_cannot_return_a_prefix() {
    for dense in [false, true] {
        let mut input = model(dense, 1.0, 0, false);
        input.frequencies_hz.push(1e308);
        let error = solve_harmonic_spring_1d(&input).unwrap_err();
        assert!(
            error.contains("frequency 1") && error.contains("range"),
            "{error}"
        );
        assert_eq!(solve_harmonic_spring_1d_owned(input).unwrap_err(), error);
        assert!(solve_harmonic_spring_1d(&model(dense, 1.0, 0, false)).is_ok());
    }
}

fn parallel_model(dense: bool, sign: f64, reversed: bool) -> SolveHarmonicSpring1dRequest {
    let mut request = model(dense, sign, 0, false);
    let mut spring = request.elements.remove(0);
    spring.stiffness *= 2.0_f64.powi(-16);
    spring.damping *= 2.0_f64.powi(-16);
    for index in 0..(1 << 16) {
        let mut element = spring.clone();
        element.id = format!("parallel-{index}");
        request.elements.push(element);
    }
    if reversed {
        request.elements.reverse();
        for element in &mut request.elements {
            std::mem::swap(&mut element.node_i, &mut element.node_j);
        }
    }
    request
}

#[test]
fn harmonic_residual_many_parallel_public_response_and_force_match_the_equivalent_member() {
    for dense in [false, true] {
        for sign in [-1.0, 1.0] {
            for reversed in [false, true] {
                let input = parallel_model(dense, sign, reversed);
                let result = solve_harmonic_spring_1d(&input).unwrap();
                let response = &result.frequencies[0].nodes[1];
                assert!(
                    (response.displacement_amplitude / 0.4_f64.hypot(0.2) - 1.0).abs()
                        <= 4.0 * f64::EPSILON
                );
                let phase = (-sign * 0.2_f64).atan2(sign * 0.4).to_degrees();
                assert!((response.displacement_phase_deg - phase).abs() <= 1e-12);
                let forces = result.frequencies[0]
                    .elements
                    .iter()
                    .filter(|element| element.id.starts_with("parallel-"));
                let mut total = 0.0;
                for force in forces {
                    assert!((force.force_amplitude / 2.0_f64.powi(-17) - 1.0).abs() < 1e-14);
                    total += force.force_amplitude;
                }
                assert!((total / 0.5 - 1.0).abs() <= 1e-10);
                assert_eq!(
                    serde_json::to_vec(&solve_harmonic_spring_1d_owned(input).unwrap()).unwrap(),
                    serde_json::to_vec(&result).unwrap()
                );
            }
        }
    }
}

#[test]
fn harmonic_residual_many_parallel_cancellation_inside_replay_preserves_clean_reuse() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    for dense in [false, true] {
        let input = parallel_model(dense, 1.0, false);
        let before = serde_json::to_vec(&input).unwrap();
        let control = SolverControl::default();
        let cancel = control.clone();
        let starts = Rc::new(Cell::new(0));
        let observed = starts.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::HarmonicResidual {
                    if p.completed_steps == 0 {
                        observed.set(observed.get() + 1);
                    }
                    if observed.get() == 2 && p.completed_steps == 1024 {
                        cancel.request_cancel();
                    }
                }
            },
            || solve_harmonic_spring_1d(&input),
        )
        .unwrap_err();
        assert!(
            starts.get() == 2 && control.was_interrupted() && error.contains("cancelled"),
            "{error}"
        );
        assert_eq!(serde_json::to_vec(&input).unwrap(), before);
        let fresh = solve_harmonic_spring_1d(&input).unwrap();
        assert!((fresh.max_displacement / 0.4_f64.hypot(0.2) - 1.0).abs() <= 4.0 * f64::EPSILON);
        assert!((fresh.max_force / 2.0_f64.powi(-17) - 1.0).abs() <= 4.0 * f64::EPSILON);
    }
}
