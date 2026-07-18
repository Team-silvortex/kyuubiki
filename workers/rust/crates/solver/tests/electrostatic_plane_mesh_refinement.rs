use kyuubiki_protocol::{
    ElectrostaticPlaneNodeInput, ElectrostaticPlaneQuadElementInput,
    ElectrostaticPlaneTriangleElementInput, SolveElectrostaticPlaneQuad2dRequest,
    SolveElectrostaticPlaneTriangle2dRequest,
};
use kyuubiki_solver::{solve_electrostatic_plane_quad_2d, solve_electrostatic_plane_triangle_2d};

const TOL: f64 = 1.0e-10;
const THICKNESS: f64 = 0.1;
const PERMITTIVITY: f64 = 3.0;
const POTENTIAL_GRADIENT: f64 = 8.0;

#[test]
fn electrostatic_quad_manufactured_linear_potential_is_refinement_invariant() {
    for subdivisions in [1_usize, 2, 4, 8] {
        let result = solve_electrostatic_plane_quad_2d(&quad_mesh(subdivisions))
            .expect("refined quad electrostatic manufactured field should solve");
        assert_eq!(result.elements.len(), subdivisions * subdivisions);
        assert_electrostatic_quad_result(&result);
    }
}

#[test]
fn electrostatic_triangle_manufactured_linear_potential_is_refinement_invariant() {
    for subdivisions in [1_usize, 2, 4, 8] {
        let result = solve_electrostatic_plane_triangle_2d(&triangle_mesh(subdivisions))
            .expect("refined triangle electrostatic manufactured field should solve");
        assert_eq!(result.elements.len(), subdivisions * subdivisions * 2);
        assert_electrostatic_triangle_result(&result);
    }
}

fn assert_electrostatic_quad_result(
    result: &kyuubiki_protocol::SolveElectrostaticPlaneQuad2dResult,
) {
    assert_close(result.max_potential, POTENTIAL_GRADIENT);
    assert_close(result.max_electric_field, POTENTIAL_GRADIENT);
    assert_close(result.max_flux_density, PERMITTIVITY * POTENTIAL_GRADIENT);
    assert_close(
        result.max_electric_energy_density,
        0.5 * PERMITTIVITY * POTENTIAL_GRADIENT.powi(2),
    );
    assert_close(
        result.total_stored_energy,
        0.5 * PERMITTIVITY * POTENTIAL_GRADIENT.powi(2) * THICKNESS,
    );
    for node in &result.nodes {
        assert_close(node.potential, manufactured_potential(node.x));
    }
    for element in &result.elements {
        assert_close(element.potential_gradient_x, POTENTIAL_GRADIENT);
        assert_close(element.potential_gradient_y, 0.0);
        assert_close(element.electric_field_x, -POTENTIAL_GRADIENT);
        assert_close(element.electric_field_y, 0.0);
        assert_close(
            element.electric_flux_density_x,
            -PERMITTIVITY * POTENTIAL_GRADIENT,
        );
        assert_close(element.electric_flux_density_y, 0.0);
    }
}

fn assert_electrostatic_triangle_result(
    result: &kyuubiki_protocol::SolveElectrostaticPlaneTriangle2dResult,
) {
    assert_close(result.max_potential, POTENTIAL_GRADIENT);
    assert_close(result.max_electric_field, POTENTIAL_GRADIENT);
    assert_close(result.max_flux_density, PERMITTIVITY * POTENTIAL_GRADIENT);
    assert_close(
        result.max_electric_energy_density,
        0.5 * PERMITTIVITY * POTENTIAL_GRADIENT.powi(2),
    );
    assert_close(
        result.total_stored_energy,
        0.5 * PERMITTIVITY * POTENTIAL_GRADIENT.powi(2) * THICKNESS,
    );
    for node in &result.nodes {
        assert_close(node.potential, manufactured_potential(node.x));
    }
    for element in &result.elements {
        assert_close(element.potential_gradient_x, POTENTIAL_GRADIENT);
        assert_close(element.potential_gradient_y, 0.0);
        assert_close(element.electric_field_x, -POTENTIAL_GRADIENT);
        assert_close(element.electric_field_y, 0.0);
        assert_close(
            element.electric_flux_density_x,
            -PERMITTIVITY * POTENTIAL_GRADIENT,
        );
        assert_close(element.electric_flux_density_y, 0.0);
    }
}

fn quad_mesh(subdivisions: usize) -> SolveElectrostaticPlaneQuad2dRequest {
    let mut elements = Vec::new();
    for y in 0..subdivisions {
        for x in 0..subdivisions {
            elements.push(ElectrostaticPlaneQuadElementInput {
                id: format!("quad-{x}-{y}"),
                node_i: grid_index(x, y, subdivisions),
                node_j: grid_index(x + 1, y, subdivisions),
                node_k: grid_index(x + 1, y + 1, subdivisions),
                node_l: grid_index(x, y + 1, subdivisions),
                thickness: THICKNESS,
                permittivity: PERMITTIVITY,
            });
        }
    }
    SolveElectrostaticPlaneQuad2dRequest {
        nodes: grid_nodes(subdivisions),
        elements,
    }
}

fn triangle_mesh(subdivisions: usize) -> SolveElectrostaticPlaneTriangle2dRequest {
    let mut elements = Vec::new();
    for y in 0..subdivisions {
        for x in 0..subdivisions {
            let lower_left = grid_index(x, y, subdivisions);
            let lower_right = grid_index(x + 1, y, subdivisions);
            let upper_right = grid_index(x + 1, y + 1, subdivisions);
            let upper_left = grid_index(x, y + 1, subdivisions);
            elements.push(triangle(
                format!("tri-a-{x}-{y}"),
                lower_left,
                lower_right,
                upper_right,
            ));
            elements.push(triangle(
                format!("tri-b-{x}-{y}"),
                lower_left,
                upper_right,
                upper_left,
            ));
        }
    }
    SolveElectrostaticPlaneTriangle2dRequest {
        nodes: grid_nodes(subdivisions),
        elements,
    }
}

fn grid_nodes(subdivisions: usize) -> Vec<ElectrostaticPlaneNodeInput> {
    let mut nodes = Vec::new();
    for y in 0..=subdivisions {
        for x in 0..=subdivisions {
            let x_coord = x as f64 / subdivisions as f64;
            let y_coord = y as f64 / subdivisions as f64;
            nodes.push(ElectrostaticPlaneNodeInput {
                id: format!("node-{x}-{y}"),
                x: x_coord,
                y: y_coord,
                fix_potential: true,
                potential: manufactured_potential(x_coord),
                charge_density: 0.0,
            });
        }
    }
    nodes
}

fn triangle(
    id: String,
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> ElectrostaticPlaneTriangleElementInput {
    ElectrostaticPlaneTriangleElementInput {
        id,
        node_i,
        node_j,
        node_k,
        thickness: THICKNESS,
        permittivity: PERMITTIVITY,
    }
}

fn grid_index(x: usize, y: usize, subdivisions: usize) -> usize {
    y * (subdivisions + 1) + x
}

fn manufactured_potential(x: f64) -> f64 {
    POTENTIAL_GRADIENT * x
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= TOL * scale,
        "expected {actual} to be close to {expected}",
    );
}
