use kyuubiki_protocol::{
    SolveThermalFrame2dRequest, SolveThermalFrame3dRequest, ThermalFrame2dElementInput,
    ThermalFrame2dNodeInput, ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
};
use kyuubiki_solver::{solve_thermal_frame_2d, solve_thermal_frame_3d};

const LENGTH: f64 = 2.0;
const ALPHA: f64 = 11.5e-6;
const TEMPERATURE_0: f64 = 18.0;
const TEMPERATURE_2: f64 = 7.0;
const GRADIENT_Y_0: f64 = 9.0;
const GRADIENT_Y_2: f64 = 4.0;
const GRADIENT_Z_0: f64 = 6.0;
const GRADIENT_Z_2: f64 = 3.0;
const DEPTH_Y: f64 = 0.22;
const DEPTH_Z: f64 = 0.18;

#[test]
fn thermal_frame_2d_quadratic_field_converges_at_second_order() {
    let expected = expected_2d_response();
    let mut axial_errors = Vec::new();
    let mut displacement_errors = Vec::new();
    let mut rotation_errors = Vec::new();

    for element_count in [1_usize, 2, 4, 8, 16] {
        let result = solve_thermal_frame_2d(&thermal_frame_2d_request(element_count))
            .expect("refined thermal frame 2d should solve");
        let tip = result
            .nodes
            .last()
            .expect("thermal frame should have a tip");

        assert_eq!(result.elements.len(), element_count);
        axial_errors.push((tip.ux - expected.axial).abs());
        displacement_errors.push((tip.uy - expected.transverse_y).abs());
        rotation_errors.push((tip.rz - expected.rotation_z).abs());
    }

    assert_second_order(&axial_errors, "2d axial thermal displacement");
    assert_second_order(&displacement_errors, "2d transverse thermal displacement");
    assert_second_order(&rotation_errors, "2d thermal rotation");
}

#[test]
fn thermal_frame_3d_quadratic_fields_converge_at_second_order() {
    let expected = expected_3d_response();
    let mut axial_errors = Vec::new();
    let mut y_displacement_errors = Vec::new();
    let mut z_displacement_errors = Vec::new();
    let mut y_rotation_errors = Vec::new();
    let mut z_rotation_errors = Vec::new();

    for element_count in [1_usize, 2, 4, 8, 16] {
        let result = solve_thermal_frame_3d(&thermal_frame_3d_request(element_count))
            .expect("refined thermal frame 3d should solve");
        let tip = result
            .nodes
            .last()
            .expect("thermal frame should have a tip");

        assert_eq!(result.elements.len(), element_count);
        axial_errors.push((tip.ux - expected.axial).abs());
        y_displacement_errors.push((tip.uy - expected.transverse_y).abs());
        z_displacement_errors.push((tip.uz - expected.transverse_z).abs());
        y_rotation_errors.push((tip.ry - expected.rotation_y).abs());
        z_rotation_errors.push((tip.rz - expected.rotation_z).abs());
    }

    assert_second_order(&axial_errors, "3d axial thermal displacement");
    assert_second_order(&y_displacement_errors, "3d local-y thermal displacement");
    assert_second_order(&z_displacement_errors, "3d local-z thermal displacement");
    assert_second_order(&y_rotation_errors, "3d local-y thermal rotation");
    assert_second_order(&z_rotation_errors, "3d local-z thermal rotation");
}

#[derive(Clone, Copy)]
struct ThermalResponse {
    axial: f64,
    transverse_y: f64,
    transverse_z: f64,
    rotation_y: f64,
    rotation_z: f64,
}

fn expected_2d_response() -> ThermalResponse {
    let axial = ALPHA * (TEMPERATURE_0 * LENGTH + TEMPERATURE_2 * LENGTH.powi(3) / 3.0);
    let curvature_scale = ALPHA / DEPTH_Y;
    ThermalResponse {
        axial,
        transverse_y: curvature_scale
            * (GRADIENT_Y_0 * LENGTH.powi(2) / 2.0 + GRADIENT_Y_2 * LENGTH.powi(4) / 12.0),
        transverse_z: 0.0,
        rotation_y: 0.0,
        rotation_z: curvature_scale * (GRADIENT_Y_0 * LENGTH + GRADIENT_Y_2 * LENGTH.powi(3) / 3.0),
    }
}

fn expected_3d_response() -> ThermalResponse {
    let mut expected = expected_2d_response();
    let curvature_scale = ALPHA / DEPTH_Z;
    expected.transverse_z = -curvature_scale
        * (GRADIENT_Z_0 * LENGTH.powi(2) / 2.0 + GRADIENT_Z_2 * LENGTH.powi(4) / 12.0);
    expected.rotation_y =
        curvature_scale * (GRADIENT_Z_0 * LENGTH + GRADIENT_Z_2 * LENGTH.powi(3) / 3.0);
    expected
}

fn thermal_frame_2d_request(element_count: usize) -> SolveThermalFrame2dRequest {
    let nodes = (0..=element_count)
        .map(|index| {
            let x = LENGTH * index as f64 / element_count as f64;
            ThermalFrame2dNodeInput {
                id: format!("n{index}"),
                x,
                y: 0.0,
                fix_x: index == 0,
                fix_y: index == 0,
                fix_rz: index == 0,
                load_x: 0.0,
                load_y: 0.0,
                moment_z: 0.0,
                temperature_delta: temperature(x),
            }
        })
        .collect();
    let elements = (0..element_count)
        .map(|index| {
            let midpoint = LENGTH * (index as f64 + 0.5) / element_count as f64;
            ThermalFrame2dElementInput {
                id: format!("e{index}"),
                node_i: index,
                node_j: index + 1,
                area: 0.018,
                youngs_modulus: 208.0e9,
                moment_of_inertia: 6.2e-6,
                section_modulus: 1.2e-4,
                thermal_expansion: ALPHA,
                section_depth: DEPTH_Y,
                temperature_gradient_y: gradient_y(midpoint),
            }
        })
        .collect();
    SolveThermalFrame2dRequest { nodes, elements }
}

fn thermal_frame_3d_request(element_count: usize) -> SolveThermalFrame3dRequest {
    let nodes = (0..=element_count)
        .map(|index| {
            let x = LENGTH * index as f64 / element_count as f64;
            ThermalFrame3dNodeInput {
                id: format!("n{index}"),
                x,
                y: 0.0,
                z: 0.0,
                fix_x: index == 0,
                fix_y: index == 0,
                fix_z: index == 0,
                fix_rx: index == 0,
                fix_ry: index == 0,
                fix_rz: index == 0,
                load_x: 0.0,
                load_y: 0.0,
                load_z: 0.0,
                moment_x: 0.0,
                moment_y: 0.0,
                moment_z: 0.0,
                temperature_delta: temperature(x),
            }
        })
        .collect();
    let elements = (0..element_count)
        .map(|index| {
            let midpoint = LENGTH * (index as f64 + 0.5) / element_count as f64;
            ThermalFrame3dElementInput {
                id: format!("e{index}"),
                node_i: index,
                node_j: index + 1,
                local_y_axis: Some([0.0, 1.0, 0.0]),
                area: 0.018,
                youngs_modulus: 208.0e9,
                shear_modulus: 80.0e9,
                torsion_constant: 4.8e-6,
                moment_of_inertia_y: 7.0e-6,
                moment_of_inertia_z: 5.3e-6,
                section_modulus_y: 1.4e-4,
                section_modulus_z: 1.1e-4,
                thermal_expansion: ALPHA,
                section_depth_y: DEPTH_Y,
                section_depth_z: DEPTH_Z,
                temperature_gradient_y: gradient_y(midpoint),
                temperature_gradient_z: gradient_z(midpoint),
            }
        })
        .collect();
    SolveThermalFrame3dRequest {
        nodes,
        elements,
        directional_springs: Vec::new(),
        directional_rotational_springs: Vec::new(),
        directional_constraints: Vec::new(),
        directional_rotational_constraints: Vec::new(),
    }
}

fn temperature(x: f64) -> f64 {
    TEMPERATURE_0 + TEMPERATURE_2 * x * x
}

fn gradient_y(x: f64) -> f64 {
    GRADIENT_Y_0 + GRADIENT_Y_2 * x * x
}

fn gradient_z(x: f64) -> f64 {
    GRADIENT_Z_0 + GRADIENT_Z_2 * x * x
}

fn assert_second_order(errors: &[f64], label: &str) {
    for pair in errors.windows(2) {
        assert!(
            pair[1] < pair[0],
            "{label} did not improve under refinement: {errors:?}",
        );
        let ratio = pair[0] / pair[1];
        assert!(
            ratio > 3.5,
            "{label} expected near-second-order convergence, ratio={ratio}: {errors:?}",
        );
    }
}
