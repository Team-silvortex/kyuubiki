use kyuubiki_protocol::{
    AcousticBar1dElementInput, AcousticBar1dNodeInput, SolveAcousticBar1dRequest,
};
use kyuubiki_solver::solve_acoustic_bar_1d;

const TOL: f64 = 1.0e-9;

#[test]
fn acoustic_bar_1d_matches_single_element_frequency_baseline() {
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
            AcousticBar1dNodeInput {
                id: "inlet".to_string(),
                x: 0.0,
                fix_pressure: true,
                pressure: fixed_pressure,
                volume_velocity_source: 0.0,
            },
            AcousticBar1dNodeInput {
                id: "source".to_string(),
                x: length,
                fix_pressure: false,
                pressure: 0.0,
                volume_velocity_source: source,
            },
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
    .expect("acoustic baseline should solve");

    let stiffness = area / (density * length);
    let mass = area * length / (6.0 * bulk_modulus);
    let dynamic = omega * omega * mass * (1.0 + damping_ratio);
    let k10 = -stiffness + dynamic;
    let k11 = stiffness + 2.0 * dynamic;
    let rhs_1 = source * omega;
    let expected_pressure = (rhs_1 - k10 * fixed_pressure) / k11;
    let expected_speed = (bulk_modulus / density).sqrt();
    let expected_wave_number = omega / expected_speed;
    let expected_gradient = (expected_pressure - fixed_pressure) / length;
    let expected_velocity = expected_gradient.abs() / (density * omega);
    let expected_average_pressure = (fixed_pressure.abs() + expected_pressure.abs()) / 2.0;
    let expected_intensity = expected_average_pressure * expected_velocity / 2.0;
    let expected_damping_loss = expected_intensity * damping_ratio * area * length;

    assert_close(result.nodes[0].pressure, fixed_pressure);
    assert_close(result.nodes[1].pressure, expected_pressure);
    assert_close(result.max_pressure, expected_pressure.abs().max(fixed_pressure));
    assert_close(result.angular_frequency, omega);
    assert_close(result.elements[0].speed_of_sound, expected_speed);
    assert_close(result.elements[0].wave_number, expected_wave_number);
    assert_close(result.elements[0].pressure_gradient, expected_gradient);
    assert_close(result.elements[0].particle_velocity, expected_velocity);
    assert_close(result.elements[0].acoustic_intensity, expected_intensity);
    assert_close(result.elements[0].damping_loss, expected_damping_loss);
    assert_close(result.total_damping_loss, expected_damping_loss);
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
