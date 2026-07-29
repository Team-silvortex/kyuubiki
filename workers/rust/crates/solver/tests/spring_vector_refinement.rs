use kyuubiki_protocol::{
    SolveSpring2dRequest, SolveSpring3dRequest, Spring2dElementInput, Spring2dNodeInput,
    Spring3dElementInput, Spring3dNodeInput,
};
use kyuubiki_solver::{solve_spring_2d, solve_spring_3d};

const TOL: f64 = 1.0e-10;
const LENGTH: f64 = 2.0;
const LOAD_X: f64 = 800.0;
const LOAD_Y: f64 = -600.0;
const LOAD_Z: f64 = 900.0;
const STIFFNESS_X: f64 = 40_000.0;
const STIFFNESS_Y: f64 = 30_000.0;
const STIFFNESS_Z: f64 = 60_000.0;

#[test]
fn orthogonal_2d_vector_springs_are_refinement_invariant() {
    let ux = LOAD_X / STIFFNESS_X;
    let uy = LOAD_Y / STIFFNESS_Y;
    let energy = 0.5 * (LOAD_X * ux + LOAD_Y * uy);

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_spring_2d(&mesh_2d(elements)).expect("refined 2D spring should solve");
        assert_close(result.nodes[0].ux, ux);
        assert_close(result.nodes[0].uy, uy);
        assert_close(result.max_displacement, (ux * ux + uy * uy).sqrt());
        assert_close(result.max_force, LOAD_X.abs().max(LOAD_Y.abs()));
        assert_close(result.total_strain_energy, energy);

        for step in 1..=elements {
            let x_node = &result.nodes[step];
            let y_node = &result.nodes[elements + step];
            let fraction = 1.0 - step as f64 / elements as f64;
            assert_close(x_node.ux, ux * fraction);
            assert_close(x_node.uy, 0.0);
            assert_close(y_node.ux, 0.0);
            assert_close(y_node.uy, uy * fraction);
        }
        for element in &result.elements[..elements] {
            assert_close(element.extension, -ux / elements as f64);
            assert_close(element.force, -LOAD_X);
            assert_close(
                element.strain_energy,
                0.5 * STIFFNESS_X * ux * ux / elements as f64,
            );
        }
        for element in &result.elements[elements..] {
            assert_close(element.extension, -uy / elements as f64);
            assert_close(element.force, -LOAD_Y);
            assert_close(
                element.strain_energy,
                0.5 * STIFFNESS_Y * uy * uy / elements as f64,
            );
        }
    }
}

#[test]
fn orthogonal_3d_vector_springs_are_refinement_invariant() {
    let ux = LOAD_X / STIFFNESS_X;
    let uy = LOAD_Y / STIFFNESS_Y;
    let uz = LOAD_Z / STIFFNESS_Z;
    let energy = 0.5 * (LOAD_X * ux + LOAD_Y * uy + LOAD_Z * uz);

    for elements in [1_usize, 2, 4, 8, 16] {
        let result = solve_spring_3d(&mesh_3d(elements)).expect("refined 3D spring should solve");
        assert_close(result.nodes[0].ux, ux);
        assert_close(result.nodes[0].uy, uy);
        assert_close(result.nodes[0].uz, uz);
        assert_close(
            result.max_displacement,
            (ux * ux + uy * uy + uz * uz).sqrt(),
        );
        assert_close(
            result.max_force,
            LOAD_X.abs().max(LOAD_Y.abs()).max(LOAD_Z.abs()),
        );
        assert_close(result.total_strain_energy, energy);

        for step in 1..=elements {
            let fraction = 1.0 - step as f64 / elements as f64;
            let x_node = &result.nodes[step];
            let y_node = &result.nodes[elements + step];
            let z_node = &result.nodes[2 * elements + step];
            assert_close(x_node.ux, ux * fraction);
            assert_close(x_node.uy, 0.0);
            assert_close(x_node.uz, 0.0);
            assert_close(y_node.ux, 0.0);
            assert_close(y_node.uy, uy * fraction);
            assert_close(y_node.uz, 0.0);
            assert_close(z_node.ux, 0.0);
            assert_close(z_node.uy, 0.0);
            assert_close(z_node.uz, uz * fraction);
        }
        check_axis_elements(
            &result.elements[..elements],
            -ux,
            -LOAD_X,
            STIFFNESS_X,
            elements,
        );
        check_axis_elements(
            &result.elements[elements..2 * elements],
            -uy,
            -LOAD_Y,
            STIFFNESS_Y,
            elements,
        );
        check_axis_elements(
            &result.elements[2 * elements..],
            -uz,
            -LOAD_Z,
            STIFFNESS_Z,
            elements,
        );
    }
}

fn mesh_2d(count: usize) -> SolveSpring2dRequest {
    let mut nodes = vec![Spring2dNodeInput {
        id: "free".to_string(),
        x: 0.0,
        y: 0.0,
        fix_x: false,
        fix_y: false,
        load_x: LOAD_X,
        load_y: LOAD_Y,
    }];
    for step in 1..=count {
        nodes.push(node_2d(format!("x-{step}"), step, count, "x"));
    }
    for step in 1..=count {
        nodes.push(node_2d(format!("y-{step}"), step, count, "y"));
    }
    let mut elements = Vec::new();
    push_2d_axis(&mut elements, 0, 1, count, STIFFNESS_X, "x");
    push_2d_axis(&mut elements, 0, count + 1, count, STIFFNESS_Y, "y");
    SolveSpring2dRequest { nodes, elements }
}

fn mesh_3d(count: usize) -> SolveSpring3dRequest {
    let mut nodes = vec![Spring3dNodeInput {
        id: "free".to_string(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        fix_x: false,
        fix_y: false,
        fix_z: false,
        load_x: LOAD_X,
        load_y: LOAD_Y,
        load_z: LOAD_Z,
    }];
    for step in 1..=count {
        nodes.push(node_3d(format!("x-{step}"), step, count, "x"));
    }
    for step in 1..=count {
        nodes.push(node_3d(format!("y-{step}"), step, count, "y"));
    }
    for step in 1..=count {
        nodes.push(node_3d(format!("z-{step}"), step, count, "z"));
    }
    let mut elements = Vec::new();
    push_3d_axis(&mut elements, 0, 1, count, STIFFNESS_X, "x");
    push_3d_axis(&mut elements, 0, count + 1, count, STIFFNESS_Y, "y");
    push_3d_axis(&mut elements, 0, 2 * count + 1, count, STIFFNESS_Z, "z");
    SolveSpring3dRequest { nodes, elements }
}

fn node_2d(id: String, step: usize, count: usize, axis: &str) -> Spring2dNodeInput {
    let position = LENGTH * step as f64 / count as f64;
    Spring2dNodeInput {
        id,
        x: if axis == "x" { position } else { 0.0 },
        y: if axis == "y" { position } else { 0.0 },
        fix_x: axis != "x" || step == count,
        fix_y: axis != "y" || step == count,
        load_x: 0.0,
        load_y: 0.0,
    }
}

fn node_3d(id: String, step: usize, count: usize, axis: &str) -> Spring3dNodeInput {
    let position = LENGTH * step as f64 / count as f64;
    Spring3dNodeInput {
        id,
        x: if axis == "x" { position } else { 0.0 },
        y: if axis == "y" { position } else { 0.0 },
        z: if axis == "z" { position } else { 0.0 },
        fix_x: axis != "x" || step == count,
        fix_y: axis != "y" || step == count,
        fix_z: axis != "z" || step == count,
        load_x: 0.0,
        load_y: 0.0,
        load_z: 0.0,
    }
}

fn push_2d_axis(
    elements: &mut Vec<Spring2dElementInput>,
    free_index: usize,
    first_index: usize,
    count: usize,
    equivalent_stiffness: f64,
    axis: &str,
) {
    for step in 0..count {
        elements.push(Spring2dElementInput {
            id: format!("{axis}-{step}"),
            node_i: if step == 0 {
                free_index
            } else {
                first_index + step - 1
            },
            node_j: first_index + step,
            stiffness: equivalent_stiffness * count as f64,
        });
    }
}

fn push_3d_axis(
    elements: &mut Vec<Spring3dElementInput>,
    free_index: usize,
    first_index: usize,
    count: usize,
    equivalent_stiffness: f64,
    axis: &str,
) {
    for step in 0..count {
        elements.push(Spring3dElementInput {
            id: format!("{axis}-{step}"),
            node_i: if step == 0 {
                free_index
            } else {
                first_index + step - 1
            },
            node_j: first_index + step,
            stiffness: equivalent_stiffness * count as f64,
        });
    }
}

fn check_axis_elements(
    elements: &[kyuubiki_protocol::Spring3dElementResult],
    total_extension: f64,
    force: f64,
    equivalent_stiffness: f64,
    count: usize,
) {
    for element in elements {
        assert_close(element.extension, total_extension / count as f64);
        assert_close(element.force, force);
        assert_close(
            element.strain_energy,
            0.5 * equivalent_stiffness * total_extension * total_extension / count as f64,
        );
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOL * expected.abs().max(1.0),
        "expected {actual} to be close to {expected}",
    );
}
