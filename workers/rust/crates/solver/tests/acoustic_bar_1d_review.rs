use kyuubiki_protocol::{
    AcousticBar1dElementInput, AcousticBar1dNodeInput, SolveAcousticBar1dRequest,
};
use kyuubiki_solver::solve_acoustic_bar_1d;

const TOL: f64 = 1.0e-9;

#[test]
fn acoustic_bar_1d_review_bundle_checks_frequency_response_velocity_intensity_and_loss() {
    let frequency_hz = 100.0;
    let omega = 2.0 * std::f64::consts::PI * frequency_hz;
    let length = 1.0;
    let area = 0.1;
    let density = 1.2;
    let bulk_modulus = 142_000.0;
    let damping_ratio = 0.02;
    let source = 0.01;
    let fixed_pressure = 1.0;

    let result = solve_acoustic_bar_1d(&SolveAcousticBar1dRequest {
        frequency_hz,
        nodes: vec![
            node("inlet", 0.0, true, fixed_pressure, 0.0),
            node("source", length, false, 0.0, source),
        ],
        elements: vec![AcousticBar1dElementInput {
            id: "duct".to_string(),
            node_i: 0,
            node_j: 1,
            area,
            density,
            bulk_modulus,
            damping_ratio,
        }],
    })
    .expect("review acoustic bar should solve");

    let stiffness = area / (density * length);
    let mass = area * length / (6.0 * bulk_modulus);
    let dynamic = omega * omega * mass * (1.0 + damping_ratio);
    let k10 = -stiffness + dynamic;
    let k11 = stiffness + 2.0 * dynamic;
    let expected_pressure = (source * omega - k10 * fixed_pressure) / k11;
    let expected_speed = (bulk_modulus / density).sqrt();
    let expected_wave_number = omega / expected_speed;
    let expected_gradient = (expected_pressure - fixed_pressure) / length;
    let expected_velocity = expected_gradient.abs() / (density * omega);
    let expected_average_pressure = (fixed_pressure.abs() + expected_pressure.abs()) / 2.0;
    let expected_intensity = expected_average_pressure * expected_velocity / 2.0;
    let expected_damping_loss = expected_intensity * damping_ratio * area * length;

    assert_eq!(result.nodes.len(), 2);
    assert_eq!(result.elements.len(), 1);
    assert_close(result.frequency_hz, frequency_hz);
    assert_close(result.angular_frequency, omega);
    assert_close(result.nodes[0].pressure, fixed_pressure);
    assert_close(result.nodes[1].pressure, expected_pressure);
    assert_close(
        result.max_pressure,
        expected_pressure.abs().max(fixed_pressure),
    );
    assert_close(result.total_damping_loss, expected_damping_loss);

    let element = &result.elements[0];
    assert_close(element.length, length);
    assert_close(element.speed_of_sound, expected_speed);
    assert_close(element.wave_number, expected_wave_number);
    assert_close(element.pressure_gradient, expected_gradient);
    assert_close(element.particle_velocity, expected_velocity);
    assert_close(element.acoustic_intensity, expected_intensity);
    assert_close(element.damping_loss, expected_damping_loss);
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

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
