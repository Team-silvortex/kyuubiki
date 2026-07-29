use kyuubiki_protocol::{
    PlaneNodeInput, PlaneQuadElementInput, SolvePlaneQuad2dRequest, SolvePlaneQuad2dResult,
};
use kyuubiki_solver::solve_plane_quad_2d;

const YOUNGS_MODULUS: f64 = 200.0e9;
const POISSON_RATIO: f64 = 0.25;
const THICKNESS: f64 = 0.1;
const SIGMA_X: f64 = 20.0e6;
const LENGTH: f64 = 2.0;
const HEIGHT: f64 = 1.0;
const TOP_TAPER: f64 = 0.3;

#[test]
fn distorted_isoparametric_quad_reproduces_the_constant_strain_patch() {
    let result = solve_plane_quad_2d(&patch_mesh(1, false))
        .expect("distorted isoparametric quad patch should solve");

    assert_patch_response(&result);
    assert_close(result.elements[0].area, 2.15, "trapezoid area");
    assert_close(result.total_strain_energy, 215.0, "trapezoid strain energy");
}

#[test]
fn distorted_quad_mesh_preserves_the_affine_patch_across_refinement() {
    for divisions in [1_usize, 2, 4] {
        let result = solve_plane_quad_2d(&patch_mesh(divisions, true))
            .expect("refined distorted quad patch should solve");

        assert_eq!(result.elements.len(), divisions * divisions);
        assert_patch_response(&result);
        assert_close(
            result.elements.iter().map(|element| element.area).sum(),
            2.15,
            "refined mesh area",
        );
        assert_close(
            result.total_strain_energy,
            215.0,
            "refined mesh strain energy",
        );
    }
}

#[test]
fn quad_rejects_inverted_node_ordering() {
    let mut request = patch_mesh(1, false);
    let element = &mut request.elements[0];
    std::mem::swap(&mut element.node_j, &mut element.node_l);

    let error = solve_plane_quad_2d(&request).expect_err("inverted quad must be rejected");
    assert!(
        error.contains("positive Jacobian"),
        "unexpected inverted-quad error: {error}",
    );
}

fn assert_patch_response(result: &SolvePlaneQuad2dResult) {
    let strain_x = SIGMA_X / YOUNGS_MODULUS;
    let strain_y = -POISSON_RATIO * strain_x;

    for node in &result.nodes {
        assert_close(node.ux, strain_x * node.x, "patch node ux");
        assert_close(node.uy, strain_y * node.y, "patch node uy");
    }
    for element in &result.elements {
        assert_close(element.strain_x, strain_x, "patch strain x");
        assert_close(element.strain_y, strain_y, "patch strain y");
        assert_near_zero(element.gamma_xy, 1.0e-12, "patch shear strain");
        assert_close(element.stress_x, SIGMA_X, "patch stress x");
        assert_near_zero(element.stress_y, 1.0e-6, "patch stress y");
        assert_near_zero(element.tau_xy, 1.0e-6, "patch shear stress");
        assert_close(element.von_mises, SIGMA_X, "patch von Mises stress");
        assert_close(
            element.strain_energy_density,
            0.5 * SIGMA_X * strain_x,
            "patch strain-energy density",
        );
    }
}

fn patch_mesh(divisions: usize, distort_interior: bool) -> SolvePlaneQuad2dRequest {
    let mut nodes = Vec::with_capacity((divisions + 1).pow(2));
    for row in 0..=divisions {
        let eta = row as f64 / divisions as f64;
        for column in 0..=divisions {
            let xi = column as f64 / divisions as f64;
            let mut x = xi * (LENGTH + TOP_TAPER * eta);
            let mut y = HEIGHT * eta;
            if distort_interior && row > 0 && row < divisions && column > 0 && column < divisions {
                let bubble = (std::f64::consts::PI * xi).sin() * (std::f64::consts::PI * eta).sin();
                x += 0.08 * bubble;
                y += 0.05 * bubble;
            }

            let on_left = column == 0;
            let on_right = column == divisions;
            let edge_force = if on_right {
                right_edge_nodal_force(row, divisions)
            } else {
                0.0
            };
            nodes.push(PlaneNodeInput {
                id: format!("n{column}_{row}"),
                x,
                y,
                fix_x: on_left,
                fix_y: row == 0 && column == 0,
                load_x: edge_force,
                load_y: 0.0,
            });
        }
    }

    let mut elements = Vec::with_capacity(divisions * divisions);
    for row in 0..divisions {
        for column in 0..divisions {
            let lower_left = node_index(column, row, divisions);
            let lower_right = node_index(column + 1, row, divisions);
            let upper_right = node_index(column + 1, row + 1, divisions);
            let upper_left = node_index(column, row + 1, divisions);
            elements.push(PlaneQuadElementInput {
                id: format!("q{column}_{row}"),
                node_i: lower_left,
                node_j: lower_right,
                node_k: upper_right,
                node_l: upper_left,
                thickness: THICKNESS,
                youngs_modulus: YOUNGS_MODULUS,
                poisson_ratio: POISSON_RATIO,
            });
        }
    }

    SolvePlaneQuad2dRequest { nodes, elements }
}

fn node_index(column: usize, row: usize, divisions: usize) -> usize {
    row * (divisions + 1) + column
}

fn right_edge_nodal_force(row: usize, divisions: usize) -> f64 {
    let edge_force = SIGMA_X * THICKNESS * HEIGHT / divisions as f64;
    let adjacent_edges = if row == 0 || row == divisions {
        1.0
    } else {
        2.0
    };
    adjacent_edges * 0.5 * edge_force
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= 1.0e-8 * scale,
        "{label}: expected {actual} to be close to {expected}",
    );
}

fn assert_near_zero(actual: f64, tolerance: f64, label: &str) {
    assert!(
        actual.abs() <= tolerance,
        "{label}: expected {actual} to be within {tolerance} of zero",
    );
}
