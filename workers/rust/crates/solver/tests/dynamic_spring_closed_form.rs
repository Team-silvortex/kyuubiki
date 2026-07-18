use kyuubiki_protocol::{
    SolveHarmonicSpring1dRequest, SolveTransientSpring1dRequest, TransientSpring1dElementInput,
    TransientSpring1dNodeInput,
};
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_transient_spring_1d};

const TOL: f64 = 1.0e-10;
const BETA: f64 = 0.25;
const GAMMA: f64 = 0.5;

#[test]
fn transient_spring_1d_matches_newmark_step_and_load_scaling() {
    let baseline_case = TransientCase {
        mass: 2.0,
        stiffness: 120.0,
        damping: 0.6,
        load: 15.0,
        initial_displacement: 0.015,
        initial_velocity: -0.02,
        time_step: 0.02,
    };
    let baseline =
        solve_transient_spring_1d(&transient_request(baseline_case)).expect("baseline transient");
    assert_transient_response(&baseline, newmark_single_step(baseline_case), baseline_case);

    let load_scale = 2.4;
    let loaded_case = TransientCase {
        load: baseline_case.load * load_scale,
        initial_displacement: baseline_case.initial_displacement * load_scale,
        initial_velocity: baseline_case.initial_velocity * load_scale,
        ..baseline_case
    };
    let loaded =
        solve_transient_spring_1d(&transient_request(loaded_case)).expect("load-scaled transient");
    assert_transient_response(&loaded, newmark_single_step(loaded_case), loaded_case);
    assert_close(loaded.nodes[1].ux / baseline.nodes[1].ux, load_scale);
    assert_close(loaded.nodes[1].vx / baseline.nodes[1].vx, load_scale);
    assert_close(loaded.nodes[1].ax / baseline.nodes[1].ax, load_scale);
    assert_close(
        loaded.history[1].kinetic_energy / baseline.history[1].kinetic_energy,
        load_scale * load_scale,
    );
    assert_close(
        loaded.history[1].strain_energy / baseline.history[1].strain_energy,
        load_scale * load_scale,
    );
}

#[test]
fn transient_spring_1d_refines_toward_undamped_free_vibration_reference() {
    let base_case = TransientCase {
        mass: 2.0,
        stiffness: 128.0,
        damping: 0.0,
        load: 0.0,
        initial_displacement: 0.1,
        initial_velocity: 0.0,
        time_step: 0.02,
    };
    let final_time = 0.1;
    let mut previous_error = f64::INFINITY;

    for time_step in [0.02, 0.01, 0.005] {
        let case = TransientCase {
            time_step,
            ..base_case
        };
        let steps = (final_time / time_step) as usize;
        let result = solve_transient_spring_1d(&transient_request_with_steps(case, steps))
            .expect("refined free-vibration transient should solve");
        let expected = undamped_free_vibration(case, final_time);
        let displacement_error = (result.nodes[1].ux - expected.displacement).abs();
        let velocity_error = (result.nodes[1].vx - expected.velocity).abs();

        assert_close(result.final_time, final_time);
        assert!(
            displacement_error < previous_error,
            "smaller time step should reduce displacement error: {displacement_error} >= {previous_error}"
        );
        assert!(
            velocity_error.is_finite(),
            "velocity error should remain finite under refinement"
        );
        previous_error = displacement_error;
    }
}

#[test]
fn harmonic_spring_1d_matches_frequency_response_and_load_scaling() {
    let baseline_case = HarmonicCase {
        mass: 2.0,
        stiffness: 1_000.0,
        damping: 2.0,
        load: 40.0,
    };
    let frequencies = [0.0, 1.0, 2.5, 4.0];
    let baseline = solve_harmonic_spring_1d(&harmonic_request(baseline_case, &frequencies))
        .expect("baseline harmonic response");
    assert_harmonic_response(&baseline, baseline_case, &frequencies);

    let load_scale = 2.25;
    let loaded_case = HarmonicCase {
        load: baseline_case.load * load_scale,
        ..baseline_case
    };
    let loaded = solve_harmonic_spring_1d(&harmonic_request(loaded_case, &frequencies))
        .expect("load-scaled harmonic response");
    assert_harmonic_response(&loaded, loaded_case, &frequencies);
    assert_close(loaded.peak_frequency_hz, baseline.peak_frequency_hz);
    assert_close(
        loaded.max_displacement / baseline.max_displacement,
        load_scale,
    );
    assert_close(loaded.max_velocity / baseline.max_velocity, load_scale);
    assert_close(
        loaded.max_acceleration / baseline.max_acceleration,
        load_scale,
    );
    assert_close(loaded.max_force / baseline.max_force, load_scale);

    let damping_scale = 3.0;
    let damped_case = HarmonicCase {
        damping: baseline_case.damping * damping_scale,
        ..baseline_case
    };
    let damped = solve_harmonic_spring_1d(&harmonic_request(damped_case, &frequencies))
        .expect("damping-scaled harmonic response");
    assert_harmonic_response(&damped, damped_case, &frequencies);
    let near_resonance = frequencies
        .iter()
        .position(|frequency_hz| (*frequency_hz - 4.0).abs() <= f64::EPSILON)
        .expect("near-resonance frequency should be present");
    assert!(
        damped.frequencies[near_resonance].max_displacement
            < baseline.frequencies[near_resonance].max_displacement,
        "larger damping should reduce the retained near-resonance displacement"
    );
    assert_close(
        damped.frequencies[near_resonance].elements[0].force_amplitude,
        harmonic_closed_form(damped_case, frequencies[near_resonance]).force_amplitude,
    );
}

#[derive(Clone, Copy)]
struct TransientCase {
    mass: f64,
    stiffness: f64,
    damping: f64,
    load: f64,
    initial_displacement: f64,
    initial_velocity: f64,
    time_step: f64,
}

#[derive(Clone, Copy)]
struct HarmonicCase {
    mass: f64,
    stiffness: f64,
    damping: f64,
    load: f64,
}

struct NewmarkExpected {
    displacement: f64,
    velocity: f64,
    acceleration: f64,
}

struct HarmonicExpected {
    angular_frequency: f64,
    displacement_amplitude: f64,
    velocity_amplitude: f64,
    acceleration_amplitude: f64,
    force_amplitude: f64,
}

struct ContinuousExpected {
    displacement: f64,
    velocity: f64,
}

fn assert_transient_response(
    result: &kyuubiki_protocol::SolveTransientSpring1dResult,
    expected: NewmarkExpected,
    case: TransientCase,
) {
    let final_step = &result.history[1];
    let tip = &result.nodes[1];
    let element = &result.elements[0];

    assert_eq!(result.history.len(), 2);
    assert_close(result.final_time, case.time_step);
    assert_close(tip.ux, expected.displacement);
    assert_close(tip.vx, expected.velocity);
    assert_close(tip.ax, expected.acceleration);
    assert_close(final_step.displacements[1], expected.displacement);
    assert_close(final_step.velocities[1], expected.velocity);
    assert_close(element.extension, expected.displacement);
    assert_close(element.relative_velocity, expected.velocity);
    assert_close(element.spring_force, case.stiffness * expected.displacement);
    assert_close(element.damping_force, case.damping * expected.velocity);
    assert_close(
        final_step.kinetic_energy,
        0.5 * case.mass * expected.velocity.powi(2),
    );
    assert_close(
        final_step.strain_energy,
        0.5 * case.stiffness * expected.displacement.powi(2),
    );
    assert_transient_summary(result);
}

fn assert_harmonic_response(
    result: &kyuubiki_protocol::SolveHarmonicSpring1dResult,
    case: HarmonicCase,
    frequencies: &[f64],
) {
    let expected_peak = frequencies
        .iter()
        .copied()
        .map(|frequency_hz| {
            (
                frequency_hz,
                harmonic_closed_form(case, frequency_hz).displacement_amplitude,
            )
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .expect("frequency list should not be empty");

    assert_eq!(result.frequencies.len(), frequencies.len());
    assert_close(result.peak_frequency_hz, expected_peak.0);

    for (frequency_result, &frequency_hz) in result.frequencies.iter().zip(frequencies.iter()) {
        let expected = harmonic_closed_form(case, frequency_hz);
        let tip = &frequency_result.nodes[1];
        let element = &frequency_result.elements[0];

        assert_close(frequency_result.frequency_hz, frequency_hz);
        assert_close(
            frequency_result.angular_frequency,
            expected.angular_frequency,
        );
        assert_close(tip.displacement_amplitude, expected.displacement_amplitude);
        assert_close(tip.velocity_amplitude, expected.velocity_amplitude);
        assert_close(tip.acceleration_amplitude, expected.acceleration_amplitude);
        assert_close(element.extension_amplitude, expected.displacement_amplitude);
        assert_close(element.force_amplitude, expected.force_amplitude);
    }
    assert_harmonic_summary(result);
}

fn assert_transient_summary(result: &kyuubiki_protocol::SolveTransientSpring1dResult) {
    assert_eq!(result.nodes.len(), result.input.nodes.len());
    assert_eq!(result.elements.len(), result.input.elements.len());
    let final_step = result.history.last().expect("history includes final step");
    assert_close(
        result.final_time,
        result.input.steps as f64 * result.input.time_step,
    );
    assert_close(
        result.max_displacement,
        result
            .history
            .iter()
            .map(|step| step.max_displacement)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_velocity,
        result
            .history
            .iter()
            .map(|step| step.max_velocity)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_force,
        result
            .elements
            .iter()
            .map(|element| (element.spring_force + element.damping_force).abs())
            .fold(0.0_f64, f64::max),
    );

    for (node, (&displacement, &velocity)) in result.nodes.iter().zip(
        final_step
            .displacements
            .iter()
            .zip(final_step.velocities.iter()),
    ) {
        let input = &result.input.nodes[node.index];
        assert_eq!(node.id, input.id);
        assert_close(node.x, input.x);
        assert_close(node.ux, displacement);
        assert_close(node.vx, velocity);
    }

    let initial_step = result
        .history
        .first()
        .expect("history includes initial step");
    assert_eq!(initial_step.step, 0);
    assert_close(initial_step.time, 0.0);
    for (node_input, (&displacement, &velocity)) in result.input.nodes.iter().zip(
        initial_step
            .displacements
            .iter()
            .zip(initial_step.velocities.iter()),
    ) {
        assert_close(displacement, node_input.initial_displacement);
        assert_close(velocity, node_input.initial_velocity);
    }

    for (expected_step, step) in result.history.iter().enumerate() {
        assert_eq!(step.step, expected_step);
        assert_eq!(step.displacements.len(), result.input.nodes.len());
        assert_eq!(step.velocities.len(), result.input.nodes.len());
        assert_close(step.time, step.step as f64 * result.input.time_step);
        assert_close(
            step.max_displacement,
            step.displacements
                .iter()
                .map(|value| value.abs())
                .fold(0.0_f64, f64::max),
        );
        assert_close(
            step.max_velocity,
            step.velocities
                .iter()
                .map(|value| value.abs())
                .fold(0.0_f64, f64::max),
        );
        assert_close(
            step.kinetic_energy,
            kinetic_energy(&result.input, &step.velocities),
        );
        assert_close(
            step.strain_energy,
            strain_energy(&result.input, &step.displacements),
        );
    }

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        let input = &result.input.elements[element.index];
        let left = &result.nodes[element.node_i];
        let right = &result.nodes[element.node_j];
        assert_close(element.extension, right.ux - left.ux);
        assert_close(element.relative_velocity, right.vx - left.vx);
        assert_close(element.spring_force, input.stiffness * element.extension);
        assert_close(
            element.damping_force,
            input.damping * element.relative_velocity,
        );
    }
}

fn assert_harmonic_summary(result: &kyuubiki_protocol::SolveHarmonicSpring1dResult) {
    assert_eq!(result.frequencies.len(), result.input.frequencies_hz.len());
    assert_close(
        result.max_displacement,
        result
            .frequencies
            .iter()
            .map(|frequency| frequency.max_displacement)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_velocity,
        result
            .frequencies
            .iter()
            .map(|frequency| frequency.max_velocity)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_acceleration,
        result
            .frequencies
            .iter()
            .map(|frequency| frequency.max_acceleration)
            .fold(0.0_f64, f64::max),
    );
    assert_close(
        result.max_force,
        result
            .frequencies
            .iter()
            .map(|frequency| frequency.max_force)
            .fold(0.0_f64, f64::max),
    );

    let peak = result
        .frequencies
        .iter()
        .max_by(|left, right| left.max_displacement.total_cmp(&right.max_displacement))
        .expect("frequency list should not be empty");
    assert_close(result.peak_frequency_hz, peak.frequency_hz);

    for (index, frequency) in result.frequencies.iter().enumerate() {
        assert_eq!(frequency.nodes.len(), result.input.nodes.len());
        assert_eq!(frequency.elements.len(), result.input.elements.len());
        assert_close(frequency.frequency_hz, result.input.frequencies_hz[index]);
        assert_close(
            frequency.angular_frequency,
            std::f64::consts::TAU * frequency.frequency_hz,
        );
        assert_close(
            frequency.max_displacement,
            frequency
                .nodes
                .iter()
                .map(|node| node.displacement_amplitude)
                .fold(0.0_f64, f64::max),
        );
        assert_close(
            frequency.max_velocity,
            frequency
                .nodes
                .iter()
                .map(|node| node.velocity_amplitude)
                .fold(0.0_f64, f64::max),
        );
        assert_close(
            frequency.max_acceleration,
            frequency
                .nodes
                .iter()
                .map(|node| node.acceleration_amplitude)
                .fold(0.0_f64, f64::max),
        );
        assert_close(
            frequency.max_force,
            frequency
                .elements
                .iter()
                .map(|element| element.force_amplitude)
                .fold(0.0_f64, f64::max),
        );

        for (node_index, node) in frequency.nodes.iter().enumerate() {
            assert_eq!(node.index, node_index);
            let input = &result.input.nodes[node.index];
            assert_eq!(node.id, input.id);
            if input.fix_x {
                assert_close(node.displacement_amplitude, 0.0);
                assert_close(node.displacement_phase_deg, 0.0);
            }
            assert_close(
                node.velocity_amplitude,
                frequency.angular_frequency * node.displacement_amplitude,
            );
            assert_close(
                node.acceleration_amplitude,
                frequency.angular_frequency.powi(2) * node.displacement_amplitude,
            );
        }

        for (element_index, element) in frequency.elements.iter().enumerate() {
            assert_eq!(element.index, element_index);
        }
    }
}

fn kinetic_energy(request: &SolveTransientSpring1dRequest, velocities: &[f64]) -> f64 {
    0.5 * request
        .nodes
        .iter()
        .zip(velocities.iter())
        .map(|(node, velocity)| node.mass * velocity * velocity)
        .sum::<f64>()
}

fn strain_energy(request: &SolveTransientSpring1dRequest, displacements: &[f64]) -> f64 {
    request
        .elements
        .iter()
        .map(|element| {
            let extension = displacements[element.node_j] - displacements[element.node_i];
            0.5 * element.stiffness * extension * extension
        })
        .sum::<f64>()
}

fn newmark_single_step(case: TransientCase) -> NewmarkExpected {
    let dt = case.time_step;
    let initial_acceleration = (case.load
        - case.stiffness * case.initial_displacement
        - case.damping * case.initial_velocity)
        / case.mass;
    let a0 = 1.0 / (BETA * dt * dt);
    let a1 = GAMMA / (BETA * dt);
    let a2 = 1.0 / (BETA * dt);
    let a3 = 1.0 / (2.0 * BETA) - 1.0;
    let a4 = GAMMA / BETA - 1.0;
    let a5 = dt * (GAMMA / (2.0 * BETA) - 1.0);
    let effective = case.stiffness + a1 * case.damping + a0 * case.mass;
    let rhs = case.load
        + case.mass
            * (a0 * case.initial_displacement
                + a2 * case.initial_velocity
                + a3 * initial_acceleration)
        + case.damping
            * (a1 * case.initial_displacement
                + a4 * case.initial_velocity
                + a5 * initial_acceleration);
    let displacement = rhs / effective;
    let acceleration = a0 * (displacement - case.initial_displacement)
        - a2 * case.initial_velocity
        - a3 * initial_acceleration;
    let velocity =
        case.initial_velocity + dt * ((1.0 - GAMMA) * initial_acceleration + GAMMA * acceleration);

    NewmarkExpected {
        displacement,
        velocity,
        acceleration,
    }
}

fn harmonic_closed_form(case: HarmonicCase, frequency_hz: f64) -> HarmonicExpected {
    let angular_frequency = std::f64::consts::TAU * frequency_hz;
    let real = case.stiffness - angular_frequency.powi(2) * case.mass;
    let imag = angular_frequency * case.damping;
    let dynamic_stiffness = (real * real + imag * imag).sqrt();
    let displacement_amplitude = case.load / dynamic_stiffness;
    let velocity_amplitude = angular_frequency * displacement_amplitude;
    let acceleration_amplitude = angular_frequency.powi(2) * displacement_amplitude;
    let force_amplitude = displacement_amplitude * (case.stiffness.powi(2) + imag.powi(2)).sqrt();

    HarmonicExpected {
        angular_frequency,
        displacement_amplitude,
        velocity_amplitude,
        acceleration_amplitude,
        force_amplitude,
    }
}

fn undamped_free_vibration(case: TransientCase, time: f64) -> ContinuousExpected {
    let omega = (case.stiffness / case.mass).sqrt();
    ContinuousExpected {
        displacement: case.initial_displacement * (omega * time).cos()
            + case.initial_velocity / omega * (omega * time).sin(),
        velocity: -case.initial_displacement * omega * (omega * time).sin()
            + case.initial_velocity * (omega * time).cos(),
    }
}

fn transient_request(case: TransientCase) -> SolveTransientSpring1dRequest {
    transient_request_with_steps(case, 1)
}

fn transient_request_with_steps(
    case: TransientCase,
    steps: usize,
) -> SolveTransientSpring1dRequest {
    SolveTransientSpring1dRequest {
        nodes: vec![
            dynamic_node("fixed", 0.0, true, 0.0, 1.0, 0.0, 0.0),
            dynamic_node(
                "tip",
                1.0,
                false,
                case.load,
                case.mass,
                case.initial_displacement,
                case.initial_velocity,
            ),
        ],
        elements: vec![dynamic_element(case.stiffness, case.damping)],
        time_step: case.time_step,
        steps,
    }
}

fn harmonic_request(case: HarmonicCase, frequencies_hz: &[f64]) -> SolveHarmonicSpring1dRequest {
    SolveHarmonicSpring1dRequest {
        nodes: vec![
            dynamic_node("fixed", 0.0, true, 0.0, 1.0, 0.0, 0.0),
            dynamic_node("tip", 1.0, false, case.load, case.mass, 0.0, 0.0),
        ],
        elements: vec![dynamic_element(case.stiffness, case.damping)],
        frequencies_hz: frequencies_hz.to_vec(),
    }
}

fn dynamic_element(stiffness: f64, damping: f64) -> TransientSpring1dElementInput {
    TransientSpring1dElementInput {
        id: "s0".to_string(),
        node_i: 0,
        node_j: 1,
        stiffness,
        damping,
    }
}

fn dynamic_node(
    id: &str,
    x: f64,
    fix_x: bool,
    load_x: f64,
    mass: f64,
    initial_displacement: f64,
    initial_velocity: f64,
) -> TransientSpring1dNodeInput {
    TransientSpring1dNodeInput {
        id: id.to_string(),
        x,
        fix_x,
        load_x,
        mass,
        initial_displacement,
        initial_velocity,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
