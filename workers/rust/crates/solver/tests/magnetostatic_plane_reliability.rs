use kyuubiki_protocol::{
    MagnetostaticPlaneNodeInput, MagnetostaticPlaneQuadElementInput,
    MagnetostaticPlaneTriangleElementInput, SolveMagnetostaticPlaneQuad2dRequest,
    SolveMagnetostaticPlaneTriangle2dRequest,
};
use kyuubiki_solver::{solve_magnetostatic_plane_quad_2d, solve_magnetostatic_plane_triangle_2d};

const TOL: f64 = 1.0e-10;
const THICKNESS: f64 = 0.1;
const MU_0: f64 = 4.0e-7 * std::f64::consts::PI;

#[test]
fn solves_magnetostatic_plane_triangle_field() {
    let result = solve_magnetostatic_plane_triangle_2d(&request())
        .expect("magnetostatic triangle should solve");

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_vector_potential > 0.0);
    assert!(result.max_magnetic_field_strength > 0.0);
    assert!(result.max_flux_density > 0.0);
    assert!(result.max_magnetic_energy_density > 0.0);
    assert!(result.total_stored_energy > 0.0);
    let element = &result.elements[0];
    assert!(element.area > 0.0);
    assert!(element.magnetic_flux_density_magnitude.is_finite());
    assert!(element.magnetic_field_strength_magnitude.is_finite());
    assert!(element.magnetic_energy_density.is_finite());
}

#[test]
fn solves_magnetostatic_plane_quad_field() {
    let result = solve_magnetostatic_plane_quad_2d(&quad_request())
        .expect("magnetostatic quad should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 1);
    assert!(result.max_vector_potential > 0.0);
    assert!(result.max_magnetic_field_strength > 0.0);
    assert!(result.max_flux_density > 0.0);
    assert!(result.max_magnetic_energy_density > 0.0);
    assert!(result.total_stored_energy > 0.0);
    assert!(result.elements[0].area > 0.0);
    assert!(result.elements[0].magnetic_energy_density.is_finite());
}

#[test]
fn magnetostatic_triangle_linear_field_is_diagonal_invariant_and_permeability_scaled() {
    let diagonal_a = solve_magnetostatic_plane_triangle_2d(&two_triangle_patch(MU_0))
        .expect("first magnetostatic diagonal should solve");
    let diagonal_b = solve_magnetostatic_plane_triangle_2d(&cross_diagonal_patch(MU_0))
        .expect("second magnetostatic diagonal should solve");
    let perturbed = solve_magnetostatic_plane_triangle_2d(&cross_diagonal_patch(MU_0 * 1.13))
        .expect("permeability-perturbed magnetostatic patch should solve");

    assert_eq!(diagonal_a.nodes.len(), diagonal_b.nodes.len());
    for (left, right) in diagonal_a.nodes.iter().zip(diagonal_b.nodes.iter()) {
        assert_close(left.vector_potential, right.vector_potential);
    }

    for element in diagonal_b.elements.iter() {
        assert_close(element.vector_potential_gradient_x, 0.0);
        assert_close(element.magnetic_flux_density_y, 0.0);
        assert_close(element.magnetic_flux_density_x, diagonal_b.max_flux_density);
    }

    assert_close(diagonal_a.max_flux_density, diagonal_b.max_flux_density);
    assert_close(
        perturbed.max_flux_density / diagonal_b.max_flux_density,
        1.13,
    );
    assert_close(
        perturbed.max_magnetic_field_strength,
        diagonal_b.max_magnetic_field_strength,
    );
    assert_close(
        perturbed.total_stored_energy / diagonal_b.total_stored_energy,
        1.13,
    );
    assert_close(
        perturbed.elements[0].vector_potential_gradient_y
            / diagonal_b.elements[0].vector_potential_gradient_y,
        1.13,
    );
}

#[test]
fn magnetostatic_quad_manufactured_linear_potential_is_refinement_invariant() {
    for subdivisions in [1_usize, 2, 4, 8] {
        let result = solve_magnetostatic_plane_quad_2d(&magnetostatic_quad_mesh(subdivisions))
            .expect("refined quad magnetostatic manufactured field should solve");

        assert_eq!(result.elements.len(), subdivisions * subdivisions);
        assert_close(result.max_vector_potential, MANUFACTURED_POTENTIAL_GRADIENT);
        assert_close(result.max_flux_density, MANUFACTURED_POTENTIAL_GRADIENT);
        assert_close(
            result.max_magnetic_field_strength,
            MANUFACTURED_POTENTIAL_GRADIENT / MU_0,
        );
        assert_close(
            result.total_stored_energy,
            0.5 * MANUFACTURED_POTENTIAL_GRADIENT.powi(2) * THICKNESS / MU_0,
        );

        for node in &result.nodes {
            assert_close(node.vector_potential, manufactured_vector_potential(node.y));
        }
        for element in &result.elements {
            assert_close(element.vector_potential_gradient_x, 0.0);
            assert_close(
                element.vector_potential_gradient_y,
                MANUFACTURED_POTENTIAL_GRADIENT,
            );
            assert_close(
                element.magnetic_flux_density_x,
                MANUFACTURED_POTENTIAL_GRADIENT,
            );
            assert_close(element.magnetic_flux_density_y, 0.0);
        }
    }
}

#[test]
fn magnetostatic_triangle_manufactured_linear_potential_is_refinement_invariant() {
    for subdivisions in [1_usize, 2, 4, 8] {
        let result =
            solve_magnetostatic_plane_triangle_2d(&magnetostatic_triangle_mesh(subdivisions))
                .expect("refined triangle magnetostatic manufactured field should solve");

        assert_eq!(result.elements.len(), subdivisions * subdivisions * 2);
        assert_close(result.max_vector_potential, MANUFACTURED_POTENTIAL_GRADIENT);
        assert_close(result.max_flux_density, MANUFACTURED_POTENTIAL_GRADIENT);
        assert_close(
            result.max_magnetic_field_strength,
            MANUFACTURED_POTENTIAL_GRADIENT / MU_0,
        );
        assert_close(
            result.total_stored_energy,
            0.5 * MANUFACTURED_POTENTIAL_GRADIENT.powi(2) * THICKNESS / MU_0,
        );

        for node in &result.nodes {
            assert_close(node.vector_potential, manufactured_vector_potential(node.y));
        }
        for element in &result.elements {
            assert_close(element.vector_potential_gradient_x, 0.0);
            assert_close(
                element.vector_potential_gradient_y,
                MANUFACTURED_POTENTIAL_GRADIENT,
            );
            assert_close(
                element.magnetic_flux_density_x,
                MANUFACTURED_POTENTIAL_GRADIENT,
            );
            assert_close(element.magnetic_flux_density_y, 0.0);
        }
    }
}

fn request() -> SolveMagnetostaticPlaneTriangle2dRequest {
    SolveMagnetostaticPlaneTriangle2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, 0.0),
            node("n2", 0.0, 1.0, false, 0.0, 5.0),
        ],
        elements: vec![MagnetostaticPlaneTriangleElementInput {
            id: "m0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            thickness: THICKNESS,
            permeability: MU_0,
        }],
    }
}

fn quad_request() -> SolveMagnetostaticPlaneQuad2dRequest {
    SolveMagnetostaticPlaneQuad2dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, true, 0.0, 0.0),
            node("n1", 1.0, 0.0, true, 0.0, 0.0),
            node("n2", 1.0, 1.0, false, 0.0, 5.0),
            node("n3", 0.0, 1.0, false, 0.0, 5.0),
        ],
        elements: vec![MagnetostaticPlaneQuadElementInput {
            id: "q0".to_string(),
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: THICKNESS,
            permeability: MU_0,
        }],
    }
}

fn two_triangle_patch(permeability: f64) -> SolveMagnetostaticPlaneTriangle2dRequest {
    SolveMagnetostaticPlaneTriangle2dRequest {
        nodes: patch_nodes(),
        elements: vec![
            triangle("lower", 0, 1, 2, permeability),
            triangle("upper", 0, 2, 3, permeability),
        ],
    }
}

fn cross_diagonal_patch(permeability: f64) -> SolveMagnetostaticPlaneTriangle2dRequest {
    SolveMagnetostaticPlaneTriangle2dRequest {
        nodes: patch_nodes(),
        elements: vec![
            triangle("left", 0, 1, 3, permeability),
            triangle("right", 1, 2, 3, permeability),
        ],
    }
}

const MANUFACTURED_POTENTIAL_GRADIENT: f64 = 5.0;

fn magnetostatic_quad_mesh(subdivisions: usize) -> SolveMagnetostaticPlaneQuad2dRequest {
    let mut elements = Vec::new();
    for y_index in 0..subdivisions {
        for x_index in 0..subdivisions {
            elements.push(MagnetostaticPlaneQuadElementInput {
                id: format!("quad-{x_index}-{y_index}"),
                node_i: grid_index(x_index, y_index, subdivisions),
                node_j: grid_index(x_index + 1, y_index, subdivisions),
                node_k: grid_index(x_index + 1, y_index + 1, subdivisions),
                node_l: grid_index(x_index, y_index + 1, subdivisions),
                thickness: THICKNESS,
                permeability: MU_0,
            });
        }
    }
    SolveMagnetostaticPlaneQuad2dRequest {
        nodes: magnetostatic_grid_nodes(subdivisions),
        elements,
    }
}

fn magnetostatic_triangle_mesh(subdivisions: usize) -> SolveMagnetostaticPlaneTriangle2dRequest {
    let mut elements = Vec::new();
    for y_index in 0..subdivisions {
        for x_index in 0..subdivisions {
            let lower_left = grid_index(x_index, y_index, subdivisions);
            let lower_right = grid_index(x_index + 1, y_index, subdivisions);
            let upper_right = grid_index(x_index + 1, y_index + 1, subdivisions);
            let upper_left = grid_index(x_index, y_index + 1, subdivisions);
            elements.push(triangle(
                &format!("tri-a-{x_index}-{y_index}"),
                lower_left,
                lower_right,
                upper_right,
                MU_0,
            ));
            elements.push(triangle(
                &format!("tri-b-{x_index}-{y_index}"),
                lower_left,
                upper_right,
                upper_left,
                MU_0,
            ));
        }
    }
    SolveMagnetostaticPlaneTriangle2dRequest {
        nodes: magnetostatic_grid_nodes(subdivisions),
        elements,
    }
}

fn magnetostatic_grid_nodes(subdivisions: usize) -> Vec<MagnetostaticPlaneNodeInput> {
    let mut nodes = Vec::new();
    for y_index in 0..=subdivisions {
        for x_index in 0..=subdivisions {
            let x = x_index as f64 / subdivisions as f64;
            let y = y_index as f64 / subdivisions as f64;
            nodes.push(node(
                &format!("node-{x_index}-{y_index}"),
                x,
                y,
                true,
                manufactured_vector_potential(y),
                0.0,
            ));
        }
    }
    nodes
}

fn grid_index(x_index: usize, y_index: usize, subdivisions: usize) -> usize {
    y_index * (subdivisions + 1) + x_index
}

fn manufactured_vector_potential(y: f64) -> f64 {
    MANUFACTURED_POTENTIAL_GRADIENT * y
}

fn patch_nodes() -> Vec<MagnetostaticPlaneNodeInput> {
    vec![
        node("ground-left", 0.0, 0.0, true, 0.0, 0.0),
        node("ground-right", 1.0, 0.0, true, 0.0, 0.0),
        node("source-right", 1.0, 1.0, false, 0.0, 5.0),
        node("source-left", 0.0, 1.0, false, 0.0, 5.0),
    ]
}

fn triangle(
    id: &str,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    permeability: f64,
) -> MagnetostaticPlaneTriangleElementInput {
    MagnetostaticPlaneTriangleElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        node_k,
        thickness: THICKNESS,
        permeability,
    }
}

fn node(
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
