use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
    ElectrostaticPlaneTriangleElementInput, MagnetostaticPlaneNodeInput,
    MagnetostaticPlaneQuadElementInput, MagnetostaticPlaneTriangleElementInput,
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest,
    SolveMagnetostaticPlaneQuad2dRequest, SolveMagnetostaticPlaneTriangle2dRequest,
};
use kyuubiki_solver::{
    solve_electrostatic_plane_quad_2d, solve_electrostatic_plane_triangle_2d,
    solve_magnetostatic_plane_quad_2d, solve_magnetostatic_plane_triangle_2d,
};

const TOL: f64 = 1.0e-10;
const EPSILON: f64 = 3.0;
const MU: f64 = 4.0e-7 * std::f64::consts::PI;
const THICKNESS: f64 = 0.1;
const EXPECTED_ELECTRIC_ENERGY_DENSITY: f64 = 96.0;
const EXPECTED_MAGNETIC_VECTOR_POTENTIAL: f64 = 0.000_125_663_706_143_591_7;
const EXPECTED_MAGNETIC_ENERGY_DENSITY: f64 = 0.006_283_185_307_179_585;

#[test]
fn electrostatic_plane_orientation_regression_preserves_energy_and_rotates_field() {
    let triangle = solve_electrostatic_plane_triangle_2d(&electrostatic_triangle_y_request())
        .expect("rotated electrostatic triangle should solve");
    let quad = solve_electrostatic_plane_quad_2d(&electrostatic_quad_y_request())
        .expect("rotated electrostatic quad should solve");

    let triangle_element = &triangle.elements[0];
    assert_close(triangle_element.potential_gradient_x, 0.0);
    assert_close(triangle_element.potential_gradient_y, -8.0);
    assert_close(triangle_element.electric_field_x, 0.0);
    assert_close(triangle_element.electric_field_y, 8.0);
    assert_close(triangle_element.electric_flux_density_x, 0.0);
    assert_close(triangle_element.electric_flux_density_y, 24.0);
    assert_close(
        triangle_element.electric_energy_density,
        EXPECTED_ELECTRIC_ENERGY_DENSITY,
    );
    assert_close(triangle_element.stored_energy, 4.8);

    let quad_element = &quad.elements[0];
    assert_close(quad_element.potential_gradient_x, 0.0);
    assert_close(quad_element.potential_gradient_y, -8.0);
    assert_close(quad_element.electric_field_x, 0.0);
    assert_close(quad_element.electric_field_y, 8.0);
    assert_close(quad_element.electric_flux_density_x, 0.0);
    assert_close(quad_element.electric_flux_density_y, 24.0);
    assert_close(
        quad_element.electric_energy_density,
        EXPECTED_ELECTRIC_ENERGY_DENSITY,
    );
    assert_close(quad_element.stored_energy, 9.6);
}

#[test]
fn electrostatic_plane_thickness_scales_energy_without_changing_field() {
    let baseline = solve_electrostatic_plane_quad_2d(&electrostatic_quad_y_request())
        .expect("baseline electrostatic quad should solve");
    let thick = solve_electrostatic_plane_quad_2d(&electrostatic_quad_y_request_with_thickness(
        THICKNESS * 2.5,
    ))
    .expect("thicker electrostatic quad should solve");

    let baseline_element = &baseline.elements[0];
    let thick_element = &thick.elements[0];
    assert_close(
        thick_element.potential_gradient_y,
        baseline_element.potential_gradient_y,
    );
    assert_close(
        thick_element.electric_field_y,
        baseline_element.electric_field_y,
    );
    assert_close(
        thick_element.electric_flux_density_y,
        baseline_element.electric_flux_density_y,
    );
    assert_close(
        thick_element.electric_energy_density,
        baseline_element.electric_energy_density,
    );
    assert_close(
        thick_element.stored_energy / baseline_element.stored_energy,
        2.5,
    );
}

#[test]
fn magnetostatic_plane_orientation_regression_preserves_energy_and_rotates_flux() {
    let triangle = solve_magnetostatic_plane_triangle_2d(&magnetostatic_triangle_x_request())
        .expect("rotated magnetostatic triangle should solve");
    let quad = solve_magnetostatic_plane_quad_2d(&magnetostatic_quad_x_request())
        .expect("rotated magnetostatic quad should solve");

    let triangle_element = &triangle.elements[0];
    assert_close(
        triangle.max_vector_potential,
        EXPECTED_MAGNETIC_VECTOR_POTENTIAL,
    );
    assert_close(
        triangle_element.vector_potential_gradient_x,
        EXPECTED_MAGNETIC_VECTOR_POTENTIAL,
    );
    assert_close(triangle_element.vector_potential_gradient_y, 0.0);
    assert_close(triangle_element.magnetic_flux_density_x, 0.0);
    assert_close(
        triangle_element.magnetic_flux_density_y,
        -EXPECTED_MAGNETIC_VECTOR_POTENTIAL,
    );
    assert_close(triangle_element.magnetic_field_strength_x, 0.0);
    assert_close(triangle_element.magnetic_field_strength_y, -100.0);
    assert_close(
        triangle_element.magnetic_energy_density,
        EXPECTED_MAGNETIC_ENERGY_DENSITY,
    );
    assert_close(triangle_element.stored_energy, 0.000_314_159_265_358_979_25);

    let quad_element = &quad.elements[0];
    assert_close(
        quad.max_vector_potential,
        EXPECTED_MAGNETIC_VECTOR_POTENTIAL,
    );
    assert_close(
        quad_element.vector_potential_gradient_x,
        EXPECTED_MAGNETIC_VECTOR_POTENTIAL,
    );
    assert_close(quad_element.vector_potential_gradient_y, 0.0);
    assert_close(quad_element.magnetic_flux_density_x, 0.0);
    assert_close(
        quad_element.magnetic_flux_density_y,
        -EXPECTED_MAGNETIC_VECTOR_POTENTIAL,
    );
    assert_close(quad_element.magnetic_field_strength_x, 0.0);
    assert_close(quad_element.magnetic_field_strength_y, -100.0);
    assert_close(
        quad_element.magnetic_energy_density,
        EXPECTED_MAGNETIC_ENERGY_DENSITY,
    );
    assert_close(quad_element.stored_energy, 0.000_628_318_530_717_958_5);
}

#[test]
fn magnetostatic_plane_current_source_scales_with_inverse_thickness() {
    let baseline = solve_magnetostatic_plane_quad_2d(&magnetostatic_quad_x_request())
        .expect("baseline magnetostatic quad should solve");
    let thick = solve_magnetostatic_plane_quad_2d(&magnetostatic_quad_x_request_with_thickness(
        THICKNESS * 2.5,
    ))
    .expect("thicker magnetostatic quad should solve");

    let baseline_element = &baseline.elements[0];
    let thick_element = &thick.elements[0];
    let inverse_thickness_scale = 1.0 / 2.5;
    assert_close(
        thick_element.vector_potential_gradient_x / baseline_element.vector_potential_gradient_x,
        inverse_thickness_scale,
    );
    assert_close(
        thick_element.magnetic_flux_density_y / baseline_element.magnetic_flux_density_y,
        inverse_thickness_scale,
    );
    assert_close(
        thick_element.magnetic_field_strength_y / baseline_element.magnetic_field_strength_y,
        inverse_thickness_scale,
    );
    assert_close(
        thick_element.magnetic_energy_density / baseline_element.magnetic_energy_density,
        inverse_thickness_scale * inverse_thickness_scale,
    );
    assert_close(
        thick_element.stored_energy / baseline_element.stored_energy,
        inverse_thickness_scale,
    );
}

fn electrostatic_triangle_y_request() -> SolveElectrostaticPlaneTriangle2dRequest {
    SolveElectrostaticPlaneTriangle2dRequest {
        nodes: vec![
            e_node("bottom-left", 0.0, 0.0, 12.0),
            e_node("right", 1.0, 0.0, 12.0),
            e_node("top", 0.0, 1.0, 4.0),
        ],
        elements: vec![ElectrostaticPlaneTriangleElementInput {
            id: "tri-y".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: THICKNESS,
            permittivity: EPSILON,
        }],
    }
}

fn electrostatic_quad_y_request() -> SolveElectrostaticPlaneQuad2dRequest {
    electrostatic_quad_y_request_with_thickness(THICKNESS)
}

fn electrostatic_quad_y_request_with_thickness(
    thickness: f64,
) -> SolveElectrostaticPlaneQuad2dRequest {
    SolveElectrostaticPlaneQuad2dRequest {
        nodes: vec![
            e_node("bottom-left", 0.0, 0.0, 12.0),
            e_node("bottom-right", 1.0, 0.0, 12.0),
            e_node("top-right", 1.0, 1.0, 4.0),
            e_node("top-left", 0.0, 1.0, 4.0),
        ],
        elements: vec![ElectrostaticPlaneQuadElementInput {
            id: "quad-y".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness,
            permittivity: EPSILON,
        }],
    }
}

fn magnetostatic_triangle_x_request() -> SolveMagnetostaticPlaneTriangle2dRequest {
    SolveMagnetostaticPlaneTriangle2dRequest {
        nodes: vec![
            m_node("ground-bottom", 0.0, 0.0, true, 0.0, 0.0),
            m_node("source-right", 1.0, 0.0, false, 0.0, 5.0),
            m_node("ground-top", 0.0, 1.0, true, 0.0, 0.0),
        ],
        elements: vec![MagnetostaticPlaneTriangleElementInput {
            id: "tri-x".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: THICKNESS,
            permeability: MU,
        }],
    }
}

fn magnetostatic_quad_x_request() -> SolveMagnetostaticPlaneQuad2dRequest {
    magnetostatic_quad_x_request_with_thickness(THICKNESS)
}

fn magnetostatic_quad_x_request_with_thickness(
    thickness: f64,
) -> SolveMagnetostaticPlaneQuad2dRequest {
    SolveMagnetostaticPlaneQuad2dRequest {
        nodes: vec![
            m_node("ground-bottom", 0.0, 0.0, true, 0.0, 0.0),
            m_node("source-bottom", 1.0, 0.0, false, 0.0, 5.0),
            m_node("source-top", 1.0, 1.0, false, 0.0, 5.0),
            m_node("ground-top", 0.0, 1.0, true, 0.0, 0.0),
        ],
        elements: vec![MagnetostaticPlaneQuadElementInput {
            id: "quad-x".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness,
            permeability: MU,
        }],
    }
}

fn e_node(id: &str, x: f64, y: f64, potential: f64) -> ElectrostaticPlaneNodeInput {
    ElectrostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_potential: true,
        potential,
        charge_density: 0.0,
    }
}

fn m_node(
    id: &str,
    x: f64,
    y: f64,
    fixed: bool,
    vector_potential: f64,
    current_density: f64,
) -> MagnetostaticPlaneNodeInput {
    MagnetostaticPlaneNodeInput {
        id: id.to_string(),
        x,
        y,
        fix_vector_potential: fixed,
        vector_potential,
        current_density,
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
