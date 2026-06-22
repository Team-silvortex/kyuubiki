use kyuubiki_protocol::{
    PlaneNodeInput, PlaneTriangleElementInput, SolvePlaneTriangle2dRequest,
    SolveThermalFrame2dRequest, SolveThermalFrame3dRequest, SolveThermalPlaneTriangle2dRequest,
    SolveThermalTruss3dRequest, ThermalFrame2dElementInput, ThermalFrame2dNodeInput,
    ThermalFrame3dElementInput, ThermalFrame3dNodeInput, ThermalPlaneNodeInput,
    ThermalPlaneTriangleElementInput, ThermalTruss3dElementInput, ThermalTruss3dNodeInput,
};
use kyuubiki_solver::{
    solve_plane_triangle_2d, solve_thermal_frame_2d, solve_thermal_frame_3d,
    solve_thermal_plane_triangle_2d, solve_thermal_truss_3d,
};

#[test]
fn valid_triangle_structural_operator_results_stay_finite() {
    let plane =
        solve_plane_triangle_2d(&plane_triangle_request()).expect("valid plane triangle solves");
    assert_finite("plane triangle max_displacement", plane.max_displacement);
    assert_finite("plane triangle max_stress", plane.max_stress);
    assert!(plane.max_displacement > 0.0);
    assert!(plane.max_stress > 0.0);
    for element in &plane.elements {
        assert_finite("plane triangle strain_x", element.strain_x);
        assert_finite("plane triangle stress_x", element.stress_x);
        assert_finite("plane triangle von_mises", element.von_mises);
    }

    let thermal = solve_thermal_plane_triangle_2d(&thermal_plane_triangle_request())
        .expect("valid thermal plane triangle solves");
    assert_finite(
        "thermal triangle max_displacement",
        thermal.max_displacement,
    );
    assert_finite("thermal triangle max_stress", thermal.max_stress);
    assert_finite(
        "thermal triangle max_temperature_delta",
        thermal.max_temperature_delta,
    );
    assert!(thermal.max_stress > 0.0);
    for element in &thermal.elements {
        assert_finite("thermal triangle thermal_strain", element.thermal_strain);
        assert_finite("thermal triangle stress_x", element.stress_x);
        assert_finite("thermal triangle von_mises", element.von_mises);
    }
}

#[test]
fn valid_thermal_frame_and_truss_operator_results_stay_finite() {
    let thermal_truss =
        solve_thermal_truss_3d(&thermal_truss_3d_request()).expect("valid thermal truss 3d solves");
    assert_finite(
        "thermal truss 3d max_displacement",
        thermal_truss.max_displacement,
    );
    assert_finite("thermal truss 3d max_stress", thermal_truss.max_stress);
    assert_finite(
        "thermal truss 3d max_axial_force",
        thermal_truss.max_axial_force,
    );
    assert_finite(
        "thermal truss 3d max_temperature_delta",
        thermal_truss.max_temperature_delta,
    );
    assert!(thermal_truss.max_stress > 0.0);
    for element in &thermal_truss.elements {
        assert_finite("thermal truss 3d thermal_strain", element.thermal_strain);
        assert_finite("thermal truss 3d stress", element.stress);
        assert_finite("thermal truss 3d axial_force", element.axial_force);
    }

    let frame_2d =
        solve_thermal_frame_2d(&thermal_frame_2d_request()).expect("valid thermal frame 2d solves");
    assert_finite(
        "thermal frame 2d max_displacement",
        frame_2d.max_displacement,
    );
    assert_finite("thermal frame 2d max_rotation", frame_2d.max_rotation);
    assert_finite("thermal frame 2d max_moment", frame_2d.max_moment);
    assert_finite("thermal frame 2d max_stress", frame_2d.max_stress);
    assert_finite(
        "thermal frame 2d max_temperature_delta",
        frame_2d.max_temperature_delta,
    );
    assert!(frame_2d.max_stress > 0.0);
    for element in &frame_2d.elements {
        assert_finite("thermal frame 2d thermal_strain", element.thermal_strain);
        assert_finite("thermal frame 2d moment_i", element.moment_i);
        assert_finite(
            "thermal frame 2d combined_stress",
            element.max_combined_stress,
        );
    }

    let frame_3d =
        solve_thermal_frame_3d(&thermal_frame_3d_request()).expect("valid thermal frame 3d solves");
    assert_finite(
        "thermal frame 3d max_displacement",
        frame_3d.max_displacement,
    );
    assert_finite("thermal frame 3d max_rotation", frame_3d.max_rotation);
    assert_finite("thermal frame 3d max_moment", frame_3d.max_moment);
    assert_finite("thermal frame 3d max_stress", frame_3d.max_stress);
    assert_finite(
        "thermal frame 3d max_temperature_delta",
        frame_3d.max_temperature_delta,
    );
    assert!(frame_3d.max_stress > 0.0);
    for element in &frame_3d.elements {
        assert_finite("thermal frame 3d thermal_strain", element.thermal_strain);
        assert_finite("thermal frame 3d moment_y_i", element.moment_y_i);
        assert_finite(
            "thermal frame 3d combined_stress",
            element.max_combined_stress,
        );
    }
}

fn plane_triangle_request() -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: vec![
            plane_node("n0", 0.0, 0.0, true, true, 0.0),
            plane_node("n1", 1.0, 0.0, false, true, 0.0),
            plane_node("n2", 1.0, 1.0, false, false, -1000.0),
            plane_node("n3", 0.0, 1.0, true, false, -1000.0),
        ],
        elements: vec![
            plane_triangle_element("p0", 0, 1, 2),
            plane_triangle_element("p1", 0, 2, 3),
        ],
    }
}

fn thermal_plane_triangle_request() -> SolveThermalPlaneTriangle2dRequest {
    SolveThermalPlaneTriangle2dRequest {
        nodes: vec![
            thermal_plane_node("n0", 0.0, 0.0),
            thermal_plane_node("n1", 1.0, 0.0),
            thermal_plane_node("n2", 1.0, 1.0),
            thermal_plane_node("n3", 0.0, 1.0),
        ],
        elements: vec![
            thermal_plane_triangle_element("tp0", 0, 1, 2),
            thermal_plane_triangle_element("tp1", 0, 2, 3),
        ],
    }
}

fn thermal_truss_3d_request() -> SolveThermalTruss3dRequest {
    SolveThermalTruss3dRequest {
        nodes: vec![
            thermal_truss_3d_node("n0", 0.0, 0.0, 0.0),
            thermal_truss_3d_node("n1", 1.0, 0.0, 0.0),
            thermal_truss_3d_node("n2", 0.0, 1.0, 0.0),
        ],
        elements: vec![
            thermal_truss_3d_element("tt3-0", 0, 1),
            thermal_truss_3d_element("tt3-1", 1, 2),
            thermal_truss_3d_element("tt3-2", 2, 0),
        ],
    }
}

fn thermal_frame_2d_request() -> SolveThermalFrame2dRequest {
    SolveThermalFrame2dRequest {
        nodes: vec![
            thermal_frame_2d_node("tf0", 0.0, 0.0, true, 0.0),
            thermal_frame_2d_node("tf1", 0.0, 3.0, false, 35.0),
            thermal_frame_2d_node("tf2", 4.0, 3.0, false, 35.0),
            thermal_frame_2d_node("tf3", 4.0, 0.0, true, 0.0),
        ],
        elements: vec![
            thermal_frame_2d_element("te0", 0, 1, 0.0),
            thermal_frame_2d_element("te1", 1, 2, 30.0),
            thermal_frame_2d_element("te2", 2, 3, 0.0),
        ],
    }
}

fn thermal_frame_3d_request() -> SolveThermalFrame3dRequest {
    SolveThermalFrame3dRequest {
        nodes: vec![
            thermal_frame_3d_node("n0", 0.0, 0.0, 0.0),
            thermal_frame_3d_node("n1", 2.0, 0.0, 0.0),
        ],
        elements: vec![ThermalFrame3dElementInput {
            id: "tf3-0".to_string(),
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

fn plane_node(id: &str, x: f64, y: f64, fix_x: bool, fix_y: bool, load_y: f64) -> PlaneNodeInput {
    PlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        load_x: 0.0,
        load_y,
    }
}

fn plane_triangle_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> PlaneTriangleElementInput {
    PlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: 0.02,
        youngs_modulus: 70.0e9,
        poisson_ratio: 0.33,
    }
}

fn thermal_plane_node(id: &str, x: f64, y: f64) -> ThermalPlaneNodeInput {
    ThermalPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: true,
        fix_y: true,
        load_x: 0.0,
        load_y: 0.0,
        temperature_delta: 40.0,
    }
}

fn thermal_plane_triangle_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> ThermalPlaneTriangleElementInput {
    ThermalPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: 0.02,
        youngs_modulus: 70.0e9,
        poisson_ratio: 0.33,
        thermal_expansion: 12.0e-6,
    }
}

fn thermal_truss_3d_node(id: &str, x: f64, y: f64, z: f64) -> ThermalTruss3dNodeInput {
    ThermalTruss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: true,
        fix_y: true,
        fix_z: true,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        temperature_delta: 40.0,
    }
}

fn thermal_truss_3d_element(id: &str, node_i: usize, node_j: usize) -> ThermalTruss3dElementInput {
    ThermalTruss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 210.0e9,
        thermal_expansion: 12.0e-6,
    }
}

fn thermal_frame_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fixed: bool,
    temperature_delta: f64,
) -> ThermalFrame2dNodeInput {
    ThermalFrame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x: fixed,
        fix_y: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y: 0.0,
        moment_z: 0.0,
        temperature_delta,
    }
}

fn thermal_frame_2d_element(
    id: &str,
    node_i: usize,
    node_j: usize,
    temperature_gradient_y: f64,
) -> ThermalFrame2dElementInput {
    ThermalFrame2dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.02,
        youngs_modulus: 210.0e9,
        moment_of_inertia: 0.00014,
        section_modulus: 0.0012,
        thermal_expansion: 12.0e-6,
        section_depth: 0.2,
        temperature_gradient_y,
    }
}

fn thermal_frame_3d_node(id: &str, x: f64, y: f64, z: f64) -> ThermalFrame3dNodeInput {
    ThermalFrame3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: true,
        fix_y: true,
        fix_z: true,
        fix_rx: true,
        fix_ry: true,
        fix_rz: true,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
        temperature_delta: 35.0,
    }
}

fn assert_finite(label: &str, value: f64) {
    assert!(value.is_finite(), "{label} must be finite, got {value}");
}
