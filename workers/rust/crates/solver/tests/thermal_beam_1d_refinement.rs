use kyuubiki_protocol::{
    SolveThermalBeam1dRequest, ThermalBeam1dElementInput, ThermalBeam1dNodeInput,
};
use kyuubiki_solver::solve_thermal_beam_1d;

const TOL: f64 = 1.0e-9;
const LENGTH: f64 = 2.4;
const YOUNGS_MODULUS: f64 = 210.0e9;
const MOMENT_OF_INERTIA: f64 = 0.00012;
const SECTION_MODULUS: f64 = 0.0011;
const THERMAL_EXPANSION: f64 = 12.0e-6;
const SECTION_DEPTH: f64 = 0.3;
const TEMPERATURE_GRADIENT_Y: f64 = 45.0;

#[test]
fn free_curvature_field_is_refinement_invariant() {
    let curvature = THERMAL_EXPANSION * TEMPERATURE_GRADIENT_Y / SECTION_DEPTH;
    let tip_rotation = curvature * LENGTH;
    let tip_displacement = 0.5 * curvature * LENGTH * LENGTH;

    for elements in [1_usize, 2, 4, 8, 16] {
        let result =
            solve_thermal_beam_1d(&mesh(elements)).expect("refined thermal beam should solve");
        assert_eq!(result.nodes.len(), elements + 1);
        assert_eq!(result.elements.len(), elements);
        assert_close(result.max_displacement, tip_displacement);
        assert_close(result.max_rotation, tip_rotation);
        assert_close(result.max_temperature_gradient, TEMPERATURE_GRADIENT_Y);
        assert_near_zero(result.max_moment, 1.0e-5);
        assert_near_zero(result.max_stress, 1.0e-3);
        assert_near_zero(result.total_strain_energy, 1.0e-9);

        for node in &result.nodes {
            assert_close(node.uy, 0.5 * curvature * node.x * node.x);
            assert_close(node.rz, curvature * node.x);
            assert_close(node.displacement_magnitude, node.uy.abs());
        }
        for element in &result.elements {
            assert_close(element.length, LENGTH / elements as f64);
            assert_close(element.temperature_gradient_y, TEMPERATURE_GRADIENT_Y);
            assert_close(element.thermal_curvature, curvature);
            assert_near_zero(element.shear_force_i, 1.0e-5);
            assert_near_zero(element.shear_force_j, 1.0e-5);
            assert_near_zero(element.moment_i, 1.0e-5);
            assert_near_zero(element.moment_j, 1.0e-5);
            assert_near_zero(element.max_bending_stress, 1.0e-3);
            assert_near_zero(element.strain_energy, 1.0e-9);
        }
    }
}

fn mesh(count: usize) -> SolveThermalBeam1dRequest {
    let nodes = (0..=count)
        .map(|index| ThermalBeam1dNodeInput {
            id: format!("node-{index}"),
            x: LENGTH * index as f64 / count as f64,
            fix_y: index == 0,
            fix_rz: index == 0,
            load_y: 0.0,
            moment_z: 0.0,
        })
        .collect();
    let elements = (0..count)
        .map(|index| ThermalBeam1dElementInput {
            id: format!("beam-{index}"),
            node_i: index,
            node_j: index + 1,
            youngs_modulus: YOUNGS_MODULUS,
            moment_of_inertia: MOMENT_OF_INERTIA,
            section_modulus: SECTION_MODULUS,
            thermal_expansion: THERMAL_EXPANSION,
            section_depth: SECTION_DEPTH,
            distributed_load_y: 0.0,
            temperature_gradient_y: TEMPERATURE_GRADIENT_Y,
        })
        .collect();
    SolveThermalBeam1dRequest { nodes, elements }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}",
    );
}

fn assert_near_zero(actual: f64, tolerance: f64) {
    assert!(
        actual.abs() <= tolerance,
        "expected {actual} to be within {tolerance} of zero",
    );
}
