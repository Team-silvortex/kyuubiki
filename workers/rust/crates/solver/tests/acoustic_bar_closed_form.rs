use kyuubiki_protocol::{
    AcousticBar1dElementInput, AcousticBar1dNodeInput, SolveAcousticBar1dRequest,
};
use kyuubiki_solver::solve_acoustic_bar_1d;

const TOL: f64 = 1.0e-9;

#[test]
fn acoustic_bar_1d_matches_closed_form_response_across_frequencies() {
    for frequency_hz in [50.0, 200.0] {
        let expected = expected_closed_form(frequency_hz);
        let result =
            solve_acoustic_bar_1d(&request(frequency_hz)).expect("closed-form acoustic case");

        assert_close(result.frequency_hz, frequency_hz);
        assert_close(result.angular_frequency, expected.omega);
        assert_close(result.nodes[0].pressure, expected.fixed_pressure);
        assert_close(result.nodes[1].pressure, expected.source_pressure);
        assert_close(result.max_pressure, expected.max_pressure);
        assert_close(result.max_particle_velocity, expected.particle_velocity);
        assert_close(result.max_acoustic_intensity, expected.acoustic_intensity);
        assert_close(result.total_damping_loss, expected.damping_loss);

        let element = &result.elements[0];
        assert_close(element.speed_of_sound, expected.speed_of_sound);
        assert_close(element.wave_number, expected.wave_number);
        assert_close(element.pressure_gradient, expected.pressure_gradient);
        assert_close(element.particle_velocity, expected.particle_velocity);
        assert_close(element.acoustic_intensity, expected.acoustic_intensity);
        assert_close(element.damping_loss, expected.damping_loss);
    }
}

#[test]
fn acoustic_bar_1d_reports_zero_loss_for_undamped_closed_form_case() {
    let mut request = request(125.0);
    request.elements[0].damping_ratio = 0.0;
    let result = solve_acoustic_bar_1d(&request).expect("undamped acoustic case");

    assert_close(result.elements[0].damping_loss, 0.0);
    assert_close(result.total_damping_loss, 0.0);
}

fn request(frequency_hz: f64) -> SolveAcousticBar1dRequest {
    SolveAcousticBar1dRequest {
        frequency_hz,
        nodes: vec![
            node("fixed", 0.0, true, 1.5, 0.0),
            node("source", 1.25, false, 0.0, 0.008),
        ],
        elements: vec![AcousticBar1dElementInput {
            id: "duct".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.08,
            density: 1.18,
            bulk_modulus: 141_000.0,
            damping_ratio: 0.015,
        }],
    }
}

fn node(
    id: &str,
    x: f64,
    fix_pressure: bool,
    pressure: f64,
    volume_velocity_source: f64,
) -> AcousticBar1dNodeInput {
    AcousticBar1dNodeInput {
        id: id.to_string(),
        x,
        fix_pressure,
        pressure,
        volume_velocity_source,
    }
}

fn expected_closed_form(frequency_hz: f64) -> ExpectedAcousticResponse {
    let request = request(frequency_hz);
    let element = &request.elements[0];
    let fixed_pressure = request.nodes[0].pressure;
    let source = request.nodes[1].volume_velocity_source;
    let length = request.nodes[element.node_j].x - request.nodes[element.node_i].x;
    let omega = 2.0 * std::f64::consts::PI * frequency_hz;
    let stiffness = element.area / (element.density * length);
    let mass = element.area * length / (6.0 * element.bulk_modulus);
    let dynamic = omega * omega * mass * (1.0 + element.damping_ratio);
    let k10 = -stiffness + dynamic;
    let k11 = stiffness + 2.0 * dynamic;
    let source_pressure = (source * omega - k10 * fixed_pressure) / k11;
    let speed_of_sound = f64::sqrt(element.bulk_modulus / element.density);
    let wave_number = omega / speed_of_sound;
    let pressure_gradient = (source_pressure - fixed_pressure) / length;
    let particle_velocity = pressure_gradient.abs() / (element.density * omega);
    let average_pressure = (fixed_pressure.abs() + source_pressure.abs()) / 2.0;
    let acoustic_intensity = average_pressure * particle_velocity / 2.0;
    let damping_loss = acoustic_intensity * element.damping_ratio * element.area * length;
    ExpectedAcousticResponse {
        omega,
        fixed_pressure,
        source_pressure,
        max_pressure: fixed_pressure.abs().max(source_pressure.abs()),
        speed_of_sound,
        wave_number,
        pressure_gradient,
        particle_velocity,
        acoustic_intensity,
        damping_loss,
    }
}

struct ExpectedAcousticResponse {
    omega: f64,
    fixed_pressure: f64,
    source_pressure: f64,
    max_pressure: f64,
    speed_of_sound: f64,
    wave_number: f64,
    pressure_gradient: f64,
    particle_velocity: f64,
    acoustic_intensity: f64,
    damping_loss: f64,
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
