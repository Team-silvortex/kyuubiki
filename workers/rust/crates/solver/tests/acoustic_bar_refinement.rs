use kyuubiki_protocol::{
    AcousticBar1dElementInput, AcousticBar1dNodeInput, SolveAcousticBar1dRequest,
};
use kyuubiki_solver::solve_acoustic_bar_1d;

const TOL: f64 = 1.0e-9;
const LENGTH: f64 = 2.0;
const LEFT_PRESSURE: f64 = 9.0;
const RIGHT_PRESSURE: f64 = 1.0;
const DENSITY: f64 = 1.2;
const BULK_MODULUS: f64 = 142_000.0;
const FREQUENCY_HZ: f64 = 100.0;

#[test]
fn fixed_pressure_linear_field_is_refinement_invariant() {
    let expected_gradient = (RIGHT_PRESSURE - LEFT_PRESSURE) / LENGTH;
    let omega = std::f64::consts::TAU * FREQUENCY_HZ;
    let expected_velocity = expected_gradient.abs() / (DENSITY * omega);
    for elements in [1_usize, 2, 4, 8, 16] {
        let result =
            solve_acoustic_bar_1d(&mesh(elements)).expect("refined acoustic field should solve");
        assert_eq!(result.elements.len(), elements);
        for node in &result.nodes {
            assert_close(node.pressure, LEFT_PRESSURE + expected_gradient * node.x);
        }
        for element in &result.elements {
            assert_close(element.pressure_gradient, expected_gradient);
            assert_close(element.particle_velocity, expected_velocity);
            assert_close(element.wave_number, omega / (BULK_MODULUS / DENSITY).sqrt());
        }
    }
}

fn mesh(count: usize) -> SolveAcousticBar1dRequest {
    let nodes = (0..=count)
        .map(|index| {
            let x = LENGTH * index as f64 / count as f64;
            AcousticBar1dNodeInput {
                id: format!("node-{index}"),
                x,
                fix_pressure: true,
                pressure: LEFT_PRESSURE + (RIGHT_PRESSURE - LEFT_PRESSURE) * x / LENGTH,
                volume_velocity_source: 0.0,
            }
        })
        .collect();
    let elements = (0..count)
        .map(|index| AcousticBar1dElementInput {
            id: format!("element-{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.1,
            density: DENSITY,
            bulk_modulus: BULK_MODULUS,
            damping_ratio: 0.02,
        })
        .collect();
    SolveAcousticBar1dRequest {
        frequency_hz: FREQUENCY_HZ,
        nodes,
        elements,
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}"
    );
}
