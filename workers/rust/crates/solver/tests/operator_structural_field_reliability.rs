use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneTriangleElementInput, Frame2dElementInput,
    Frame2dNodeInput, Frame3dElementInput, Frame3dNodeInput, HeatPlaneNodeInput,
    HeatPlaneTriangleElementInput, SolveElectrostaticPlaneTriangle2dRequest, SolveFrame2dRequest,
    SolveFrame3dRequest, SolveHeatPlaneTriangle2dRequest, SolveTruss3dRequest, Truss3dElementInput,
    Truss3dNodeInput,
};
use kyuubiki_solver::{
    solve_electrostatic_plane_triangle_2d, solve_frame_2d, solve_frame_3d,
    solve_heat_plane_triangle_2d, solve_truss_3d,
};

#[test]
fn valid_triangle_field_operator_results_stay_finite() {
    let heat = solve_heat_plane_triangle_2d(&heat_triangle_request())
        .expect("valid heat triangle should solve");
    assert_finite("heat triangle max_temperature", heat.max_temperature);
    assert_finite("heat triangle max_heat_flux", heat.max_heat_flux);
    assert!(heat.max_heat_flux > 0.0);
    for element in &heat.elements {
        assert_finite("heat triangle gradient_x", element.temperature_gradient_x);
        assert_finite("heat triangle gradient_y", element.temperature_gradient_y);
        assert_finite("heat triangle flux", element.heat_flux_magnitude);
    }

    let electro = solve_electrostatic_plane_triangle_2d(&electro_triangle_request())
        .expect("valid electrostatic triangle should solve");
    assert_finite("electro triangle max_potential", electro.max_potential);
    assert_finite(
        "electro triangle max_electric_field",
        electro.max_electric_field,
    );
    assert_finite(
        "electro triangle max_flux_density",
        electro.max_flux_density,
    );
    assert!(electro.max_electric_field > 0.0);
    for element in &electro.elements {
        assert_finite("electro triangle gradient_x", element.potential_gradient_x);
        assert_finite("electro triangle gradient_y", element.potential_gradient_y);
        assert_finite("electro triangle field", element.electric_field_magnitude);
    }
}

#[test]
fn valid_3d_truss_and_frame_operator_results_stay_finite() {
    let truss = solve_truss_3d(&truss_3d_request()).expect("valid truss 3d should solve");
    assert_finite("truss 3d max_displacement", truss.max_displacement);
    assert_finite("truss 3d max_stress", truss.max_stress);
    assert!(truss.max_displacement > 0.0);
    assert!(truss.max_stress > 0.0);
    for element in &truss.elements {
        assert_finite("truss 3d strain", element.strain);
        assert_finite("truss 3d stress", element.stress);
        assert_finite("truss 3d axial_force", element.axial_force);
    }

    let frame_2d = solve_frame_2d(&frame_2d_request()).expect("valid frame 2d should solve");
    assert_finite("frame 2d max_displacement", frame_2d.max_displacement);
    assert_finite("frame 2d max_rotation", frame_2d.max_rotation);
    assert_finite("frame 2d max_moment", frame_2d.max_moment);
    assert_finite("frame 2d max_stress", frame_2d.max_stress);
    assert!(frame_2d.max_displacement > 0.0);
    assert!(frame_2d.max_stress > 0.0);
    for element in &frame_2d.elements {
        assert_finite("frame 2d moment_i", element.moment_i);
        assert_finite("frame 2d combined_stress", element.max_combined_stress);
    }

    let frame_3d = solve_frame_3d(&frame_3d_request()).expect("valid frame 3d should solve");
    assert_finite("frame 3d max_displacement", frame_3d.max_displacement);
    assert_finite("frame 3d max_rotation", frame_3d.max_rotation);
    assert_finite("frame 3d max_moment", frame_3d.max_moment);
    assert_finite("frame 3d max_stress", frame_3d.max_stress);
    assert!(frame_3d.max_displacement > 0.0);
    assert!(frame_3d.max_stress > 0.0);
    for element in &frame_3d.elements {
        assert_finite("frame 3d moment_z_i", element.moment_z_i);
        assert_finite("frame 3d combined_stress", element.max_combined_stress);
    }
}

fn heat_triangle_request() -> SolveHeatPlaneTriangle2dRequest {
    SolveHeatPlaneTriangle2dRequest {
        nodes: vec![
            heat_node("h0", 0.0, 0.0, true, 100.0),
            heat_node("h1", 1.0, 0.0, false, 0.0),
            heat_node("h2", 1.0, 1.0, true, 20.0),
            heat_node("h3", 0.0, 1.0, true, 20.0),
        ],
        elements: vec![
            HeatPlaneTriangleElementInput {
                id: "hp0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                thickness: 0.02,
                conductivity: 45.0,
            },
            HeatPlaneTriangleElementInput {
                id: "hp1".to_string(),
                node_i: 0,
                node_j: 2,
                node_k: 3,
                thickness: 0.02,
                conductivity: 45.0,
            },
        ],
    }
}

fn electro_triangle_request() -> SolveElectrostaticPlaneTriangle2dRequest {
    SolveElectrostaticPlaneTriangle2dRequest {
        nodes: vec![
            electro_node("e0", 0.0, 0.0, true, 12.0),
            electro_node("e1", 1.0, 0.0, true, 4.0),
            electro_node("e2", 0.0, 1.0, true, 12.0),
        ],
        elements: vec![ElectrostaticPlaneTriangleElementInput {
            id: "ep0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: 0.1,
            permittivity: 3.0,
        }],
    }
}

fn truss_3d_request() -> SolveTruss3dRequest {
    SolveTruss3dRequest {
        nodes: vec![
            truss_3d_node("b0", 0.0, 0.0, 0.0, true, true, true, 0.0),
            truss_3d_node("b1", 1.2, 0.0, 0.0, true, true, true, 0.0),
            truss_3d_node("b2", 0.0, 1.2, 0.0, true, true, true, 0.0),
            truss_3d_node("top", 0.35, 0.35, 1.0, false, false, false, -1600.0),
        ],
        elements: vec![
            truss_3d_element("e0", 0, 1),
            truss_3d_element("e1", 1, 2),
            truss_3d_element("e2", 2, 0),
            truss_3d_element("e3", 0, 3),
            truss_3d_element("e4", 1, 3),
            truss_3d_element("e5", 2, 3),
        ],
    }
}

fn frame_2d_request() -> SolveFrame2dRequest {
    SolveFrame2dRequest {
        nodes: vec![
            frame_2d_node("n0", 0.0, 0.0, true, true, true, 0.0),
            frame_2d_node("n1", 2.0, 0.0, false, false, false, -1000.0),
        ],
        elements: vec![Frame2dElementInput {
            id: "f0".to_string(),
            node_i: 0,
            node_j: 1,
            area: 0.02,
            youngs_modulus: 210.0e9,
            moment_of_inertia: 8.0e-6,
            section_modulus: 1.6e-4,
        }],
    }
}

fn frame_3d_request() -> SolveFrame3dRequest {
    SolveFrame3dRequest {
        nodes: vec![
            frame_3d_node("n0", 0.0, 0.0, 0.0, true, 0.0),
            frame_3d_node("n1", 2.0, 0.0, 0.0, false, -1000.0),
        ],
        elements: vec![Frame3dElementInput {
            id: "f0".to_string(),
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

fn heat_node(
    id: &str,
    x: f64,
    y: f64,
    fix_temperature: bool,
    temperature: f64,
) -> HeatPlaneNodeInput {
    HeatPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_temperature,
        temperature,
        heat_load: 0.0,
    }
}

fn electro_node(
    id: &str,
    x: f64,
    y: f64,
    fix_potential: bool,
    potential: f64,
) -> ElectrostaticPlaneNodeInput {
    ElectrostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_potential,
        potential,
        charge_density: 0.0,
    }
}

#[allow(clippy::too_many_arguments)]
fn truss_3d_node(
    id: &str,
    x: f64,
    y: f64,
    z: f64,
    fix_x: bool,
    fix_y: bool,
    fix_z: bool,
    load_z: f64,
) -> Truss3dNodeInput {
    Truss3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x,
        fix_y,
        fix_z,
        load_x: 0.0,
        load_y: 0.0,
        load_z,
    }
}

fn truss_3d_element(id: &str, node_i: usize, node_j: usize) -> Truss3dElementInput {
    Truss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
    }
}

fn frame_2d_node(
    id: &str,
    x: f64,
    y: f64,
    fix_x: bool,
    fix_y: bool,
    fix_rz: bool,
    load_y: f64,
) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_x,
        fix_y,
        fix_rz,
        load_x: 0.0,
        load_y,
        moment_z: 0.0,
    }
}

fn frame_3d_node(id: &str, x: f64, y: f64, z: f64, fixed: bool, load_y: f64) -> Frame3dNodeInput {
    Frame3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        fix_rx: fixed,
        fix_ry: fixed,
        fix_rz: fixed,
        load_x: 0.0,
        load_y,
        load_z: 0.0,
        moment_x: 0.0,
        moment_y: 0.0,
        moment_z: 0.0,
    }
}

fn assert_finite(label: &str, value: f64) {
    assert!(value.is_finite(), "{label} must be finite, got {value}");
}
