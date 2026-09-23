use kyuubiki_protocol::{
    SolveHarmonicSpring1dRequest, TransientSpring1dElementInput, TransientSpring1dNodeInput,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_harmonic_spring_1d_owned};
use std::f64::consts::TAU;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn oscillator() -> SolveHarmonicSpring1dRequest {
    SolveHarmonicSpring1dRequest {
        nodes: vec![
            TransientSpring1dNodeInput {
                id: "base".into(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
                mass: 1.0,
                initial_displacement: 0.0,
                initial_velocity: 0.0,
            },
            TransientSpring1dNodeInput {
                id: "tip".into(),
                x: 1.0,
                fix_x: false,
                load_x: 1.0,
                mass: 1.0,
                initial_displacement: 0.0,
                initial_velocity: 0.0,
            },
        ],
        elements: vec![TransientSpring1dElementInput {
            id: "spring".into(),
            node_i: 0,
            node_j: 1,
            stiffness: 2.0,
            damping: 0.5,
        }],
        frequencies_hz: vec![0.0, 1.0 / TAU, 2.0 / TAU],
    }
}

#[test]
fn finite_displacement_cannot_publish_infinite_acceleration() {
    let mut input = oscillator();
    input.frequencies_hz = vec![1.0e100 / TAU];
    input.nodes[1].mass = 1.0e-200;
    input.nodes[1].load_x = 1.0e150;
    input.elements[0].damping = 0.0;
    let error = solve_harmonic_spring_1d(&input).expect_err("acceleration exceeds f64 range");
    assert!(
        error.contains("acceleration") && error.contains("node 1"),
        "{error}"
    );
    assert!(error.contains("frequency 0"), "{error}");
    assert!(solve_harmonic_spring_1d(&oscillator()).is_ok());
}

#[test]
fn finite_response_is_not_rejected_by_overflow_in_the_residual_denominator() {
    let mut input = oscillator();
    input.frequencies_hz = vec![1.0 / TAU];
    input.nodes[1].mass = 1.0e-300;
    input.nodes[1].load_x = 1.0e8;
    input.elements[0].stiffness = 2.0e-300;
    input.elements[0].damping = 0.0;
    let result = solve_harmonic_spring_1d(&input).unwrap();
    let frame = &result.frequencies[0];
    for value in [
        frame.max_displacement,
        frame.max_velocity,
        frame.max_acceleration,
    ] {
        assert!(value.is_finite());
        assert!((value / 1.0e308 - 1.0).abs() < 1.0e-12, "{value}");
    }
    assert!((frame.max_force / 2.0e8 - 1.0).abs() < 1.0e-12);
}

#[test]
fn harmonic_numerical_work_observes_cancellation_before_returning_success() {
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |_| cancel.request_cancel(),
        || {
            let result = solve_harmonic_spring_1d(&oscillator());
            assert!(
                result.is_err(),
                "cancellation must reach the solver, not only its wrapper"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancelled"), "{error}");
    assert!(control.was_interrupted());
    assert!(solve_harmonic_spring_1d(&oscillator()).is_ok());
}

#[test]
fn finite_node_response_cannot_publish_infinite_element_force() {
    let mut input = oscillator();
    input.frequencies_hz = vec![1.0 / TAU];
    input.nodes[1].load_x = 1.0e308;
    input.elements[0].damping = 0.0;
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("element 0") && error.contains("force"),
        "{error}"
    );
}

#[test]
fn finite_complex_coefficients_need_not_have_a_representable_norm() {
    let mut input = oscillator();
    input.frequencies_hz = vec![1.0 / TAU];
    input.elements[0].stiffness = 1.3e308;
    input.elements[0].damping = 1.3e308;
    let result = solve_harmonic_spring_1d(&input).unwrap();
    let frame = &result.frequencies[0];
    let expected = (1.0 / 1.3e308) / 2.0_f64.sqrt();
    assert!((frame.max_displacement / expected - 1.0).abs() < 1.0e-12);
    assert!((frame.max_force - 1.0).abs() < 1.0e-12);
    assert!((frame.nodes[1].displacement_phase_deg + 45.0).abs() < 1.0e-10);
}

#[test]
fn damped_resonance_retains_amplitude_and_phase_as_damping_shrinks() {
    for damping in [1.0, 1.0e-4, 1.0e-12] {
        let mut input = oscillator();
        input.elements[0].stiffness = 1.0;
        input.elements[0].damping = damping;
        input.frequencies_hz = vec![0.0, 1.0 / TAU];
        let result = solve_harmonic_spring_1d(&input).unwrap();
        let frame = &result.frequencies[1];
        assert!((frame.max_displacement * damping - 1.0).abs() < 1.0e-12);
        assert!((frame.nodes[1].displacement_phase_deg + 90.0).abs() < 1.0e-10);
        assert!((frame.max_force * damping / 1.0_f64.hypot(damping) - 1.0).abs() < 1.0e-12);
        assert_eq!(result.frequencies[0].max_velocity, 0.0);
        assert_eq!(result.frequencies[0].max_acceleration, 0.0);
    }
}

#[test]
fn singular_middle_frequency_reports_its_index_and_allows_a_fresh_damped_run() {
    let mut input = oscillator();
    input.elements[0].stiffness = 1.0;
    input.elements[0].damping = 0.0;
    let error = solve_harmonic_spring_1d(&input).unwrap_err();
    assert!(
        error.contains("frequency 1") && error.contains("singular"),
        "{error}"
    );
    input.elements[0].damping = 0.01;
    assert_eq!(
        solve_harmonic_spring_1d(&input).unwrap().frequencies.len(),
        3
    );
}

fn coupled_network(count: usize) -> SolveHarmonicSpring1dRequest {
    let mut input = oscillator();
    input.nodes.truncate(1);
    input.elements.clear();
    for index in 1..=count {
        let mut node = oscillator().nodes[1].clone();
        node.id = format!("node-{index}");
        node.x = index as f64;
        node.load_x = if index == 1 { 1.0 } else { 0.0 };
        input.nodes.push(node);
        input.elements.push(TransientSpring1dElementInput {
            id: format!("ground-{index}"),
            node_i: 0,
            node_j: index,
            stiffness: 2.0,
            damping: 0.25,
        });
    }
    let mut edges = (1..count)
        .map(|index| (index, index + 1))
        .collect::<Vec<_>>();
    if count == 3 {
        edges.push((3, 1));
    }
    for (node_i, node_j) in edges {
        input.elements.push(TransientSpring1dElementInput {
            id: format!("link-{node_i}-{node_j}"),
            node_i,
            node_j,
            stiffness: 3.0,
            damping: 0.1,
        });
    }
    input.frequencies_hz = [0.0, 1.0, 2.0_f64.sqrt(), 11.0_f64.sqrt(), 4.0]
        .map(|omega| omega / TAU)
        .to_vec();
    input
}

fn reciprocal(real: f64, imaginary: f64) -> (f64, f64) {
    let denominator = real * real + imaginary * imaginary;
    (real / denominator, -imaginary / denominator)
}

fn check_network_reference(count: usize) {
    let input = coupled_network(count);
    let result = solve_harmonic_spring_1d(&input).unwrap();
    for frame in &result.frequencies {
        let omega = frame.angular_frequency;
        let n = count as f64;
        // The two-node link and three-node cycle have Laplacian eigenvalues 0 and n.
        let common = reciprocal(2.0 - omega * omega, 0.25 * omega);
        let relative = reciprocal(2.0 + 3.0 * n - omega * omega, (0.25 + 0.1 * n) * omega);
        for node in frame.nodes.iter().skip(1) {
            let weight = if node.index == 1 { n - 1.0 } else { -1.0 };
            let expected = (
                (common.0 + weight * relative.0) / n,
                (common.1 + weight * relative.1) / n,
            );
            let phase = node.displacement_phase_deg.to_radians();
            let actual = (
                node.displacement_amplitude * phase.cos(),
                node.displacement_amplitude * phase.sin(),
            );
            assert!((actual.0 - expected.0).hypot(actual.1 - expected.1) < 1.0e-11);
        }
        let input_power = frame
            .nodes
            .iter()
            .map(|node| {
                -0.5 * omega
                    * input.nodes[node.index].load_x
                    * node.displacement_amplitude
                    * node.displacement_phase_deg.to_radians().sin()
            })
            .sum::<f64>();
        let damping_power = frame
            .elements
            .iter()
            .map(|element| {
                0.5 * input.elements[element.index].damping
                    * (omega * element.extension_amplitude).powi(2)
            })
            .sum::<f64>();
        assert!(
            (input_power - damping_power).abs() < 1.0e-11,
            "omega={omega}: {input_power} vs {damping_power}"
        );
    }
}

#[test]
fn path_response_matches_modal_decomposition_and_average_power_balance() {
    check_network_reference(2);
}

#[test]
fn dense_cycle_response_matches_modal_decomposition_and_average_power_balance() {
    check_network_reference(3);
}

#[test]
fn independent_rows_with_four_hundred_decades_of_contrast_preserve_response() {
    let mut input = oscillator();
    input.nodes.push(input.nodes[1].clone());
    input.nodes[2].id = "stiff-tip".into();
    input.nodes[2].x = 2.0;
    input.elements.push(input.elements[0].clone());
    input.elements[1].node_j = 2;
    input.elements[1].id = "stiff-spring".into();
    for (index, scale) in [(1, 1.0e-200), (2, 1.0e200)] {
        input.nodes[index].mass *= scale;
        input.nodes[index].load_x *= scale;
        input.elements[index - 1].stiffness *= scale;
        input.elements[index - 1].damping *= scale;
    }
    let baseline = solve_harmonic_spring_1d(&oscillator()).unwrap();
    let result = solve_harmonic_spring_1d(&input).unwrap();
    for (frame, expected) in result.frequencies.iter().zip(&baseline.frequencies) {
        for node in frame.nodes.iter().skip(1) {
            assert!(
                (node.displacement_amplitude / expected.max_displacement - 1.0).abs() < 1.0e-12
            );
        }
    }
}

#[test]
fn factor_residual_and_result_stages_can_cancel_and_replay_without_shared_state() {
    for (count, stages) in [
        (
            2,
            vec![
                SolverStage::TridiagonalFactor,
                SolverStage::TridiagonalSubstitution,
            ],
        ),
        (
            3,
            vec![
                SolverStage::DenseFactor,
                SolverStage::DenseSubstitution,
                SolverStage::HarmonicResidual,
                SolverStage::ResultNodes,
                SolverStage::ResultElements,
                SolverStage::ResultTotals,
            ],
        ),
    ] {
        let input = coupled_network(count);
        let baseline = serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap();
        for stage in stages {
            let control = SolverControl::default();
            let cancel = control.clone();
            let error = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == stage && point.completed_steps > 0 {
                        cancel.request_cancel();
                    }
                },
                || {
                    let result = solve_harmonic_spring_1d(&input);
                    assert!(result.is_err(), "cancel was ignored at {stage:?}");
                    result
                },
            )
            .unwrap_err();
            assert!(error.contains("cancelled"), "{stage:?}: {error}");
            assert_eq!(control.last_checkpoint().unwrap().stage, stage);
            assert_eq!(
                serde_json::to_value(solve_harmonic_spring_1d(&input).unwrap()).unwrap(),
                baseline
            );
        }
    }
}

#[test]
fn cancelling_a_later_frequency_discards_the_already_computed_frames() {
    let input = oscillator();
    let control = SolverControl::default();
    let cancel = control.clone();
    let completed = Arc::new(AtomicUsize::new(0));
    let observed = completed.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ResultElements && point.completed_steps == 1 {
                observed.fetch_add(1, Ordering::Relaxed);
            }
            if point.stage == SolverStage::HarmonicSweep && point.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || solve_harmonic_spring_1d(&input),
    )
    .unwrap_err();
    assert!(error.contains("harmonic_sweep"), "{error}");
    assert_eq!(completed.load(Ordering::Relaxed), 1);
    assert_eq!(
        solve_harmonic_spring_1d(&input).unwrap().frequencies.len(),
        3
    );
}

#[test]
fn input_order_duplicates_and_owned_execution_preserve_the_sampled_peak() {
    let mut input = oscillator();
    input.frequencies_hz = vec![2.0 / TAU, 0.0, 1.0 / TAU, 2.0 / TAU];
    let result = solve_harmonic_spring_1d(&input).unwrap();
    assert_eq!(
        result
            .frequencies
            .iter()
            .map(|frame| frame.frequency_hz)
            .collect::<Vec<_>>(),
        input.frequencies_hz
    );
    assert_eq!(result.peak_frequency_hz, 1.0 / TAU);
    let owned = solve_harmonic_spring_1d_owned(input).unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::to_value(owned).unwrap()
    );
}
