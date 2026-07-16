use super::common::assert_close;
use kyuubiki_protocol::{
    SolveHarmonicSpring1dRequest, SolveTransientSpring1dRequest, TransientSpring1dElementInput,
    TransientSpring1dNodeInput,
};
use kyuubiki_solver::{solve_harmonic_spring_1d, solve_transient_spring_1d};

const BETA: f64 = 0.25;
const GAMMA: f64 = 0.5;

#[test]
fn transient_spring_1d_matches_newmark_single_step_closed_form() {
    for case in [
        TransientCase {
            mass: 2.0,
            stiffness: 100.0,
            damping: 0.0,
            load: 0.0,
            initial_displacement: 0.1,
            initial_velocity: 0.0,
            time_step: 0.01,
        },
        TransientCase {
            mass: 2.0,
            stiffness: 100.0,
            damping: 0.5,
            load: 10.0,
            initial_displacement: 0.02,
            initial_velocity: -0.03,
            time_step: 0.02,
        },
    ] {
        let result = solve_transient_spring_1d(&transient_request(case))
            .expect("single-step transient spring should solve");
        let expected = newmark_single_step(case);
        let tip = &result.nodes[1];
        let element = &result.elements[0];
        let initial = &result.history[0];
        let final_step = &result.history[1];

        assert_eq!(result.history.len(), 2);
        assert_close(result.final_time, case.time_step, "transient final time");
        assert_close(
            initial.displacements[1],
            case.initial_displacement,
            "transient initial displacement",
        );
        assert_close(
            initial.velocities[1],
            case.initial_velocity,
            "transient initial velocity",
        );
        assert_close(tip.ux, expected.displacement, "transient tip displacement");
        assert_close(tip.vx, expected.velocity, "transient tip velocity");
        assert_close(tip.ax, expected.acceleration, "transient tip acceleration");
        assert_close(
            final_step.displacements[1],
            expected.displacement,
            "transient history displacement",
        );
        assert_close(
            final_step.velocities[1],
            expected.velocity,
            "transient history velocity",
        );
        assert_close(
            element.extension,
            expected.displacement,
            "transient extension",
        );
        assert_close(
            element.relative_velocity,
            expected.velocity,
            "transient relative velocity",
        );
        assert_close(
            element.spring_force,
            case.stiffness * expected.displacement,
            "transient spring force",
        );
        assert_close(
            element.damping_force,
            case.damping * expected.velocity,
            "transient damping force",
        );
        assert_close(
            final_step.kinetic_energy,
            0.5 * case.mass * expected.velocity.powi(2),
            "transient kinetic energy",
        );
        assert_close(
            final_step.strain_energy,
            0.5 * case.stiffness * expected.displacement.powi(2),
            "transient strain energy",
        );
    }
}

#[test]
fn harmonic_spring_1d_matches_single_dof_frequency_response_closed_form() {
    let case = HarmonicCase {
        mass: 2.0,
        stiffness: 1000.0,
        damping: 2.0,
        load: 50.0,
    };
    let frequencies = [0.0, 1.0, 2.5, 4.0];
    let result = solve_harmonic_spring_1d(&harmonic_request(case, &frequencies))
        .expect("harmonic spring frequency sweep should solve");
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
        .expect("frequency list should be non-empty");

    assert_eq!(result.frequencies.len(), frequencies.len());
    assert_close(
        result.peak_frequency_hz,
        expected_peak.0,
        "harmonic peak frequency",
    );

    for (frequency_result, &frequency_hz) in result.frequencies.iter().zip(frequencies.iter()) {
        let expected = harmonic_closed_form(case, frequency_hz);
        let tip = &frequency_result.nodes[1];
        let element = &frequency_result.elements[0];

        assert_close(
            frequency_result.frequency_hz,
            frequency_hz,
            "harmonic frequency",
        );
        assert_close(
            frequency_result.angular_frequency,
            expected.angular_frequency,
            "harmonic angular frequency",
        );
        assert_close(
            tip.displacement_amplitude,
            expected.displacement_amplitude,
            "harmonic displacement amplitude",
        );
        assert_close(
            tip.velocity_amplitude,
            expected.velocity_amplitude,
            "harmonic velocity amplitude",
        );
        assert_close(
            tip.acceleration_amplitude,
            expected.acceleration_amplitude,
            "harmonic acceleration amplitude",
        );
        assert_close(
            element.extension_amplitude,
            expected.displacement_amplitude,
            "harmonic extension amplitude",
        );
        assert_close(
            element.force_amplitude,
            expected.force_amplitude,
            "harmonic force amplitude",
        );
        assert_close(
            frequency_result.max_displacement,
            expected.displacement_amplitude,
            "harmonic max displacement",
        );
        assert_close(
            frequency_result.max_velocity,
            expected.velocity_amplitude,
            "harmonic max velocity",
        );
        assert_close(
            frequency_result.max_acceleration,
            expected.acceleration_amplitude,
            "harmonic max acceleration",
        );
        assert_close(
            frequency_result.max_force,
            expected.force_amplitude,
            "harmonic max force",
        );
    }
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
struct NewmarkExpected {
    displacement: f64,
    velocity: f64,
    acceleration: f64,
}

#[derive(Clone, Copy)]
struct HarmonicCase {
    mass: f64,
    stiffness: f64,
    damping: f64,
    load: f64,
}

struct HarmonicExpected {
    angular_frequency: f64,
    displacement_amplitude: f64,
    velocity_amplitude: f64,
    acceleration_amplitude: f64,
    force_amplitude: f64,
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

fn transient_request(case: TransientCase) -> SolveTransientSpring1dRequest {
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
        elements: vec![TransientSpring1dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: case.stiffness,
            damping: case.damping,
        }],
        time_step: case.time_step,
        steps: 1,
    }
}

fn harmonic_request(case: HarmonicCase, frequencies_hz: &[f64]) -> SolveHarmonicSpring1dRequest {
    SolveHarmonicSpring1dRequest {
        nodes: vec![
            dynamic_node("fixed", 0.0, true, 0.0, 1.0, 0.0, 0.0),
            dynamic_node("tip", 1.0, false, case.load, case.mass, 0.0, 0.0),
        ],
        elements: vec![TransientSpring1dElementInput {
            id: "s0".to_string(),
            node_i: 0,
            node_j: 1,
            stiffness: case.stiffness,
            damping: case.damping,
        }],
        frequencies_hz: frequencies_hz.to_vec(),
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
