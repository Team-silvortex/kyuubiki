use kyuubiki_protocol::SolveHarmonicSpring1dRequest;
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use serde_json::json;

fn model(dense: bool, power: i32, sign: f64, reversed: bool) -> SolveHarmonicSpring1dRequest {
    let d = 2.0_f64.powi(-27);
    let omega = (1.0 + d) * 2.0_f64.powi(power);
    let force = sign * (5.0 * 2.0_f64.powi(-54) - 2.0_f64.powi(-81));
    let mut input = json!({
        "nodes":[
            {"id":"root","x":0.0,"fix_x":true,"mass":1.0,"load_x":0.0},
            {"id":"tip","x":1.0,"fix_x":false,"mass":(1.0+d)*2.0_f64.powi(-2*power),"load_x":force}
        ],"elements":[
            {"id":"spring","node_i":0,"node_j":1,"stiffness":1.0+3.0*d+2.0_f64.powi(-51)}
        ],"frequencies_hz":[omega / std::f64::consts::TAU]
    });
    if dense {
        for node in 2..=4 {
            input["nodes"].as_array_mut().unwrap().push(json!({
                "id":format!("control-{node}"),"x":node as f64,"fix_x":false,"mass":2.0_f64.powi(-2*power),"load_x":0.0
            }));
            input["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("ground-{node}"),"node_i":0,"node_j":node,"stiffness":4.0
            }));
        }
        for (i, j) in [(2, 3), (3, 4), (4, 2)] {
            input["elements"].as_array_mut().unwrap().push(json!({
                "id":format!("cycle-{i}-{j}"),"node_i":i,"node_j":j,"stiffness":1.0
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
fn harmonic_inertia_precision_public_oscillator_matches_the_independent_cubic_identity() {
    for dense in [false, true] {
        for power in [-500, -200, 0, 200, 500] {
            for sign in [-1.0, 1.0] {
                for reversed in [false, true] {
                    let input = model(dense, power, sign, reversed);
                    let result = solve_harmonic_spring_1d(&input).unwrap_or_else(|error| {
                        panic!("dense={dense}, power={power}, reversed={reversed}: {error}")
                    });
                    let response = &result.frequencies[0].nodes[1];
                    assert_eq!(
                        response.displacement_amplitude, 1.0,
                        "dense={dense}, power={power}: {response:?}"
                    );
                    let spring = result.frequencies[0]
                        .elements
                        .iter()
                        .find(|e| e.id == "spring")
                        .unwrap();
                    assert_eq!(
                        spring.force_amplitude,
                        1.0 + 3.0 * 2.0_f64.powi(-27) + 2.0_f64.powi(-51)
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
fn harmonic_inertia_precision_cancelled_preparation_and_failed_sweeps_leave_requests_reusable() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    for dense in [false, true] {
        let input = model(dense, 0, 1.0, false);
        let before = serde_json::to_vec(&input).unwrap();
        let baseline = serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap();
        for (stage, completed, occurrence) in [
            (
                SolverStage::ConstraintReduce,
                (input.nodes.len() - 1) as u64,
                if dense { 1 } else { 2 },
            ),
            (
                SolverStage::ElementAssembly,
                input.elements.len() as u64 + 1,
                1,
            ),
            (
                SolverStage::ElementAssembly,
                input.elements.len() as u64 + 2,
                1,
            ),
            (
                SolverStage::ElementAssembly,
                input.elements.len() as u64 + 3,
                1,
            ),
            (SolverStage::HarmonicResidual, 0, 1),
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let hits = Rc::new(Cell::new(0));
            let observed = hits.clone();
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == stage && p.completed_steps == completed {
                        observed.set(observed.get() + 1);
                        if observed.get() == occurrence {
                            cancel.request_cancel();
                        }
                    }
                },
                || solve_harmonic_spring_1d(&input),
            )
            .unwrap_err();
            assert_eq!(hits.get(), occurrence);
            assert!(
                control.was_interrupted() && error.contains("cancelled"),
                "{error}"
            );
            assert_eq!(serde_json::to_vec(&input).unwrap(), before);
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap(),
                baseline
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
            baseline
        );
    }
}

#[test]
fn harmonic_inertia_precision_nonrepresentable_subnormal_is_rejected_before_factorization() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    for dense in [false, true] {
        let mut input = model(dense, 0, 1.0, false);
        input.nodes[1].mass = f64::from_bits(1);
        let entered = Rc::new(Cell::new(false));
        let observed = entered.clone();
        let error = with_solver_observer(
            &SolverControl::default(),
            move |p| {
                if matches!(
                    p.stage,
                    SolverStage::LinearPrepare
                        | SolverStage::DenseFactor
                        | SolverStage::TridiagonalFactor
                ) {
                    observed.set(true);
                }
            },
            || solve_harmonic_spring_1d(&input),
        )
        .unwrap_err();
        assert!(!entered.get());
        assert!(
            error.contains("node 1 inertia coefficient") && error.contains("lose numerical range"),
            "{error}"
        );
        assert_eq!(
            solve_harmonic_spring_1d(&model(dense, 0, 1.0, false))
                .unwrap()
                .max_displacement,
            1.0
        );
    }
}

#[test]
fn harmonic_inertia_precision_below_quantum_tails_do_not_reject_normal_coefficients() {
    for dense in [false, true] {
        let mut input = model(dense, 0, 1.0, false);
        let d = 2.0_f64.powi(-27);
        let scale = 2.0_f64.powi(-1000);
        input.nodes[1].mass *= scale;
        input.elements[0].stiffness = 2.0 * scale;
        // Correct rounding of (2 - (1+d)^3) * scale; the cubic tail
        // lies below the physical f64 quantum, not below the row's scale.
        input.nodes[1].load_x = (1.0 - 3.0 * d - f64::EPSILON) * scale;
        let result = solve_harmonic_spring_1d(&input).unwrap();
        assert_eq!(
            result.frequencies[0].nodes[1]
                .displacement_amplitude
                .to_bits(),
            1.0_f64.to_bits()
        );
    }
}

#[test]
fn harmonic_inertia_precision_extreme_cross_scale_residual_enclosure_is_certified() {
    for dense in [false, true] {
        let mut input = model(dense, 0, 1.0, false);
        input.nodes[1].mass *= 2.0_f64.powi(-1000);
        let expected = input.nodes[1].load_x / input.elements[0].stiffness;
        let result = solve_harmonic_spring_1d(&input).unwrap();
        assert_eq!(
            result.frequencies[0].nodes[1]
                .displacement_amplitude
                .to_bits(),
            expected.to_bits()
        );
        assert_eq!(
            solve_harmonic_spring_1d(&model(dense, 0, 1.0, false))
                .unwrap()
                .max_displacement,
            1.0
        );
    }
}
