use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dNodeInput, Frame3dElementInput, Frame3dNodeInput,
    SolveFrame2dRequest, SolveFrame3dRequest, SolveThermalFrame2dRequest,
    SolveThermalFrame3dRequest, ThermalFrame2dElementInput, ThermalFrame2dNodeInput,
    ThermalFrame3dElementInput, ThermalFrame3dNodeInput,
};
use kyuubiki_solver::{
    solve_frame_2d, solve_frame_3d, solve_thermal_frame_2d, solve_thermal_frame_3d,
};

#[test]
fn frame_2d_rejects_non_finite_node_coordinates_and_loads() {
    let mut request = frame_2d_request();
    request.nodes[1].x = f64::NAN;
    assert!(solve_frame_2d(&request).is_err());

    let mut request = frame_2d_request();
    request.nodes[1].load_y = f64::INFINITY;
    assert!(solve_frame_2d(&request).is_err());

    let mut request = frame_2d_request();
    request.nodes[1].moment_z = f64::NEG_INFINITY;
    assert!(solve_frame_2d(&request).is_err());
}

#[test]
fn thermal_frame_2d_rejects_non_finite_node_coordinates_loads_and_temperature() {
    let mut request = thermal_frame_2d_request();
    request.nodes[1].y = f64::NAN;
    assert!(solve_thermal_frame_2d(&request).is_err());

    let mut request = thermal_frame_2d_request();
    request.nodes[1].load_x = f64::INFINITY;
    assert!(solve_thermal_frame_2d(&request).is_err());

    let mut request = thermal_frame_2d_request();
    request.nodes[1].temperature_delta = f64::NEG_INFINITY;
    assert!(solve_thermal_frame_2d(&request).is_err());
}

#[test]
fn frame_3d_rejects_non_finite_node_coordinates_loads_and_moments() {
    let mut request = frame_3d_request();
    request.nodes[1].z = f64::NAN;
    assert!(solve_frame_3d(&request).is_err());

    let mut request = frame_3d_request();
    request.nodes[1].load_y = f64::INFINITY;
    assert!(solve_frame_3d(&request).is_err());

    let mut request = frame_3d_request();
    request.nodes[1].moment_z = f64::NEG_INFINITY;
    assert!(solve_frame_3d(&request).is_err());
}

#[test]
fn thermal_frame_3d_rejects_non_finite_node_coordinates_loads_moments_and_temperature() {
    let mut request = thermal_frame_3d_request();
    request.nodes[1].z = f64::NAN;
    assert!(solve_thermal_frame_3d(&request).is_err());

    let mut request = thermal_frame_3d_request();
    request.nodes[1].load_y = f64::INFINITY;
    assert!(solve_thermal_frame_3d(&request).is_err());

    let mut request = thermal_frame_3d_request();
    request.nodes[1].moment_z = f64::NEG_INFINITY;
    assert!(solve_thermal_frame_3d(&request).is_err());

    let mut request = thermal_frame_3d_request();
    request.nodes[1].temperature_delta = f64::NAN;
    assert!(solve_thermal_frame_3d(&request).is_err());
}

fn frame_2d_request() -> SolveFrame2dRequest {
    SolveFrame2dRequest {
        nodes: vec![
            frame_2d_node("fixed", 0.0, 0.0, true, true, true, 0.0, 0.0, 0.0),
            frame_2d_node("tip", 2.0, 0.0, false, false, false, 0.0, -1000.0, 0.0),
        ],
        elements: vec![Frame2dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
        }],
    }
}

fn thermal_frame_2d_request() -> SolveThermalFrame2dRequest {
    SolveThermalFrame2dRequest {
        nodes: vec![
            thermal_frame_2d_node("fixed", 0.0, 0.0, true, true, true, 40.0),
            thermal_frame_2d_node("restrained", 2.0, 0.0, true, true, true, 40.0),
        ],
        elements: vec![ThermalFrame2dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
            thermal_expansion: 12.0e-6,
            section_depth: 0.3,
            temperature_gradient_y: 0.0,
        }],
    }
}

fn frame_3d_request() -> SolveFrame3dRequest {
    SolveFrame3dRequest {
        nodes: vec![
            frame_3d_node(
                "fixed", 0.0, 0.0, 0.0, true, true, true, true, true, true, 0.0, 0.0, 0.0, 0.0,
                0.0, 0.0,
            ),
            frame_3d_node(
                "tip", 2.0, 0.0, 0.0, false, false, false, false, false, false, 0.0, -1000.0, 0.0,
                0.0, 0.0, 0.0,
            ),
        ],
        elements: vec![Frame3dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 8.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.6e-4,
        }],
    }
}

fn thermal_frame_3d_request() -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            thermal_frame_3d_node(
                "fixed", 0.0, 0.0, 0.0, true, true, true, true, true, true, 40.0,
            ),
            thermal_frame_3d_node(
                "restrained",
                2.0,
                0.0,
                0.0,
                true,
                true,
                true,
                true,
                true,
                true,
                40.0,
            ),
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "beam".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            shear_modulus: 80.0e9,
            torsion_constant: 5.0e-6,
            moment_of_inertia_y: 8.0e-6,
            moment_of_inertia_z: 6.0e-6,
            section_modulus_y: 1.6e-4,
            section_modulus_z: 1.2e-4,
            thermal_expansion: 12.0e-6,
            section_depth_y: 0.2,
            section_depth_z: 0.15,
            temperature_gradient_y: 30.0,
            temperature_gradient_z: 20.0,
        }],
    }
}

#[allow(clippy::too_many_arguments)]
fn frame_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    fix_rz: bool,
    load_x: f64,
    load_y: f64,
    moment_z: f64,
) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        fix_rz,
        load_x,
        load_y,
        moment_z,
    }
}

fn thermal_frame_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    fix_rz: bool,
    temperature_delta: f64,
) -> ThermalFrame2dNodeInput {
    ThermalFrame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        fix_rz,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}

#[allow(clippy::too_many_arguments)]
fn frame_3d_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    fix_rx: bool,
    fix_ry: bool,
    fix_rz: bool,
    load_x: f64,
    load_y: f64,
    load_z: f64,
    moment_x: f64,
    moment_y: f64,
    moment_z: f64,
) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        fix_rx,
        fix_ry,
        fix_rz,
        load_x,
        load_y,
        load_z,
        moment_x,
        moment_y,
        moment_z,
    }
}

#[allow(clippy::too_many_arguments)]
fn thermal_frame_3d_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    fix_rx: bool,
    fix_ry: bool,
    fix_rz: bool,
    temperature_delta: f64,
) -> ThermalFrame3dNodeInput {
    ThermalFrame3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        fix_rx,
        fix_ry,
        fix_rz,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}
