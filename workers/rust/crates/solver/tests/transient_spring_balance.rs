use kyuubiki_protocol::{
    SolveTransientSpring1dRequest, TransientSpring1dElementInput, TransientSpring1dNodeInput,
};
use kyuubiki_solver::solve_transient_spring_1d;
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn oscillator() -> SolveTransientSpring1dRequest {
    SolveTransientSpring1dRequest {
        nodes: vec![
            TransientSpring1dNodeInput {
                id: "base".into(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
                mass: 2.0,
                initial_displacement: 0.0,
                initial_velocity: 0.0,
            },
            TransientSpring1dNodeInput {
                id: "tip".into(),
                x: 1.0,
                fix_x: false,
                load_x: 10.0,
                mass: 2.0,
                initial_displacement: 0.1,
                initial_velocity: 0.0,
            },
        ],
        elements: vec![TransientSpring1dElementInput {
            id: "spring".into(),
            node_i: 0,
            node_j: 1,
            stiffness: 100.0,
            damping: 0.5,
        }],
        time_step: 0.01,
        steps: 20,
        history_stride: None,
    }
}

#[test]
fn static_preload_does_not_create_acceleration_when_the_time_step_shrinks() {
    for time_step in [0.01, 1.0e-6, 1.0e-10] {
        let mut input = oscillator();
        input.time_step = time_step;
        let result = solve_transient_spring_1d(&input).unwrap();
        let tip = &result.nodes[1];
        assert!(
            (tip.ux - 0.1).abs() < 1.0e-14,
            "dt={time_step}: u={}",
            tip.ux
        );
        assert!(tip.vx.abs() < 1.0e-10, "dt={time_step}: v={}", tip.vx);
        assert!(tip.ax.abs() < 1.0e-8, "dt={time_step}: a={}", tip.ax);
    }
}

#[test]
fn rigid_translation_retains_velocity_without_spurious_internal_dynamics() {
    let mut input = oscillator();
    input.time_step = 1.0e-7;
    input.steps = 8;
    for node in &mut input.nodes {
        node.fix_x = false;
        node.load_x = 0.0;
        node.initial_displacement = 1000.0;
        node.initial_velocity = 1.0;
    }
    let result = solve_transient_spring_1d(&input).unwrap();
    for node in &result.nodes {
        assert!((node.ux - (1000.0 + result.final_time)).abs() < 1.0e-9);
        assert!((node.vx - 1.0).abs() < 1.0e-9, "v={}", node.vx);
        assert!(node.ax.abs() < 1.0e-7, "a={}", node.ax);
    }
    assert!(result.elements[0].spring_force.abs() < 1.0e-9);
    assert!(result.elements[0].damping_force.abs() < 1.0e-9);
}

#[test]
fn sparse_history_cannot_conceal_an_unrepresentable_intermediate_energy() {
    let mut input = oscillator();
    input.nodes[1].mass = 1.0;
    input.nodes[1].initial_displacement = 0.0;
    input.nodes[1].load_x = 1.2e154;
    input.elements[0].stiffness = 1.0;
    input.elements[0].damping = 0.0;
    input.time_step = 0.1;
    input.steps = 63;
    // Near a full cycle the final energy is finite, but near half a cycle it is not.
    for stride in [None, Some(63), Some(100)] {
        input.history_stride = stride;
        let error =
            solve_transient_spring_1d(&input).expect_err("sampling must not skip validation");
        assert!(error.contains("energy"), "{error}");
    }
    assert!(solve_transient_spring_1d(&oscillator()).is_ok());
}

#[test]
fn invalid_total_duration_fails_before_entering_a_potentially_unbounded_run() {
    let mut input = oscillator();
    input.time_step = 1.0e300;
    input.steps = usize::MAX;
    input.history_stride = Some(usize::MAX);
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |_| cancel.request_cancel(),
        || solve_transient_spring_1d(&input),
    )
    .unwrap_err();
    assert!(
        error.contains("simulation time became non-finite"),
        "{error}"
    );
    assert!(
        !control.cancellation_requested(),
        "invalid duration must fail before numerical work"
    );
}

#[test]
fn undamped_average_acceleration_conserves_discrete_energy() {
    let mut input = oscillator();
    input.nodes[1].load_x = 0.0;
    input.elements[0].damping = 0.0;
    input.steps = 2000;
    let result = solve_transient_spring_1d(&input).unwrap();
    let initial = result.history[0].kinetic_energy + result.history[0].strain_energy;
    for step in &result.history {
        let energy = step.kinetic_energy + step.strain_energy;
        assert!(
            (energy / initial - 1.0).abs() < 5.0e-9,
            "step {}: {energy}",
            step.step
        );
    }
}

#[test]
fn damped_loaded_steps_balance_work_energy_and_dissipation() {
    let mut input = oscillator();
    input.nodes[1].load_x = -2.0;
    input.nodes[1].initial_velocity = 0.2;
    input.elements[0].damping = 3.0;
    input.steps = 400;
    let result = solve_transient_spring_1d(&input).unwrap();
    for steps in result.history.windows(2) {
        let [old, new] = steps else { unreachable!() };
        let energy_change =
            new.kinetic_energy + new.strain_energy - old.kinetic_energy - old.strain_energy;
        let midpoint_velocity = 0.5 * (old.velocities[1] + new.velocities[1]);
        let dissipation = input.time_step * input.elements[0].damping * midpoint_velocity.powi(2);
        let work = input.nodes[1].load_x * (new.displacements[1] - old.displacements[1]);
        assert!(
            (energy_change + dissipation - work).abs() < 1.0e-10,
            "step {}",
            new.step
        );
    }
}

#[test]
fn damped_response_converges_quadratically_to_an_independent_continuous_solution() {
    let mut input = oscillator();
    input.nodes[1].load_x = -2.0;
    input.nodes[1].initial_velocity = 0.2;
    input.elements[0].damping = 3.0;
    let mass = input.nodes[1].mass;
    let stiffness = input.elements[0].stiffness;
    let decay = input.elements[0].damping / (2.0 * mass);
    let frequency = (stiffness / mass - decay * decay).sqrt();
    let equilibrium = input.nodes[1].load_x / stiffness;
    let a = input.nodes[1].initial_displacement - equilibrium;
    let b = (input.nodes[1].initial_velocity + decay * a) / frequency;
    let time = 1.2;
    let (sin, cos) = (frequency * time).sin_cos();
    let envelope = (-decay * time).exp();
    let expected_u = equilibrium + envelope * (a * cos + b * sin);
    let expected_v =
        envelope * ((frequency * b - decay * a) * cos - (frequency * a + decay * b) * sin);
    let mut previous_error = f64::INFINITY;
    for steps in [30, 60, 120] {
        input.steps = steps;
        input.time_step = time / steps as f64;
        let result = solve_transient_spring_1d(&input).unwrap();
        let tip = &result.nodes[1];
        let error = (tip.ux - expected_u).hypot((tip.vx - expected_v) / frequency);
        assert!(
            error < 0.3 * previous_error,
            "steps={steps}, error={error}, previous={previous_error}"
        );
        previous_error = error;
    }
    assert!(previous_error < 3.0e-4, "{previous_error}");
}

fn chain() -> SolveTransientSpring1dRequest {
    let mut input = oscillator();
    for index in 2..4 {
        let mut node = input.nodes[1].clone();
        node.id = format!("n{index}");
        node.x = index as f64;
        input.nodes.push(node);
    }
    for (index, (mass, load, u, v)) in [
        (2.0, 0.0, 0.0, 0.0),
        (3.0, 0.4, 0.1, 0.05),
        (5.0, -0.1, 0.04, -0.04),
        (7.0, -0.3, -0.02, 0.1),
    ]
    .into_iter()
    .enumerate()
    {
        let node = &mut input.nodes[index];
        node.mass = mass;
        node.load_x = load;
        node.initial_displacement = u;
        node.initial_velocity = v;
    }
    input.elements = [(100.0, 0.5), (75.0, 2.0), (35.0, 1.0)]
        .into_iter()
        .enumerate()
        .map(
            |(index, (stiffness, damping))| TransientSpring1dElementInput {
                id: format!("s{index}"),
                node_i: index,
                node_j: index + 1,
                stiffness,
                damping,
            },
        )
        .collect();
    input.steps = 400;
    input
}

#[test]
fn heterogeneous_chain_preserves_work_balance_and_sampling_does_not_change_the_solve() {
    let mut input = chain();
    let full = solve_transient_spring_1d(&input).unwrap();
    for pair in full.history.windows(2) {
        let old = &pair[0];
        let new = &pair[1];
        let midpoint: Vec<_> = old
            .velocities
            .iter()
            .zip(&new.velocities)
            .map(|(a, b)| 0.5 * (a + b))
            .collect();
        let dissipation: f64 = input
            .elements
            .iter()
            .map(|e| {
                input.time_step * e.damping * (midpoint[e.node_j] - midpoint[e.node_i]).powi(2)
            })
            .sum();
        let work: f64 = input
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| n.load_x * (new.displacements[i] - old.displacements[i]))
            .sum();
        let energy_change =
            new.kinetic_energy + new.strain_energy - old.kinetic_energy - old.strain_energy;
        assert!(
            (energy_change + dissipation - work).abs() < 1.0e-11,
            "step {}",
            new.step
        );
        for (i, mid) in midpoint.into_iter().enumerate() {
            assert!(
                (new.displacements[i] - old.displacements[i] - input.time_step * mid).abs()
                    < 1.0e-15
            );
        }
    }
    input.history_stride = Some(37);
    let sampled = solve_transient_spring_1d(&input).unwrap();
    assert_eq!(sampled.max_displacement, full.max_displacement);
    assert_eq!(sampled.max_velocity, full.max_velocity);
    assert_eq!(
        serde_json::to_value(&sampled.nodes).unwrap(),
        serde_json::to_value(&full.nodes).unwrap()
    );
    for step in &sampled.history {
        assert_eq!(
            serde_json::to_value(step).unwrap(),
            serde_json::to_value(&full.history[step.step]).unwrap()
        );
    }
}

#[test]
fn restarting_from_final_displacement_and_velocity_matches_an_uninterrupted_run() {
    let mut input = chain();
    let full = solve_transient_spring_1d(&input).unwrap();
    input.steps = 137;
    let first = solve_transient_spring_1d(&input).unwrap();
    for (initial, final_node) in input.nodes.iter_mut().zip(&first.nodes) {
        initial.initial_displacement = final_node.ux;
        initial.initial_velocity = final_node.vx;
    }
    input.steps = 400 - 137;
    let resumed = solve_transient_spring_1d(&input).unwrap();
    assert_eq!(
        serde_json::to_value(&resumed.nodes).unwrap(),
        serde_json::to_value(&full.nodes).unwrap()
    );
}

#[test]
fn cancellation_interrupts_unsaved_steps_and_result_recovery_without_poisoning_replay() {
    let mut input = chain();
    input.steps = 20;
    input.history_stride = Some(20);
    let baseline = solve_transient_spring_1d(&input).unwrap();
    for stage in [
        SolverStage::TransientStep,
        SolverStage::TransientState,
        SolverStage::TransientEnergy,
        SolverStage::ResultNodes,
        SolverStage::ResultElements,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let step = Arc::new(AtomicUsize::new(0));
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::TransientStep {
                    step.store(point.completed_steps as usize, Ordering::Relaxed);
                }
                if step.load(Ordering::Relaxed) >= 3 && point.stage == stage {
                    cancel.request_cancel();
                }
            },
            || {
                let result = solve_transient_spring_1d(&input);
                assert!(
                    result.is_err(),
                    "{stage:?} must interrupt inside the solver"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        let replay = solve_transient_spring_1d(&input).unwrap();
        assert_eq!(
            serde_json::to_value(&replay).unwrap(),
            serde_json::to_value(&baseline).unwrap()
        );
    }
}
