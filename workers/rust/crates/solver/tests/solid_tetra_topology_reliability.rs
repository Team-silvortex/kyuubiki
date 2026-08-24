use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
};
use kyuubiki_solver::solve_solid_tetra_3d;

#[test]
fn solid_tetra_rejects_restraints_that_leave_a_rigid_rotation() {
    let mut request = tetra_request();
    for node in &mut request.nodes {
        node.fix_x = false;
        node.fix_y = false;
        node.fix_z = false;
    }
    for node in &mut request.nodes[..2] {
        node.fix_x = true;
        node.fix_y = true;
        node.fix_z = true;
    }

    let error = solve_solid_tetra_3d(&request)
        .expect_err("two fixed points must not hide rotation around their connecting line");
    assert!(
        error.contains("component 0") && error.contains("rigid-body rank 5/6"),
        "unexpected restraint-rank error: {error}",
    );
}

#[test]
fn solid_tetra_rejects_orphan_nodes_and_unrestrained_components() {
    let mut orphan = tetra_request();
    orphan.nodes.push(node("orphan", 9.0, 9.0, 9.0, false));
    let error = solve_solid_tetra_3d(&orphan).expect_err("orphan node should fail preflight");
    assert!(
        error.contains("orphan") && error.contains("not referenced by any element"),
        "unexpected orphan error: {error}",
    );

    let mut floating = tetra_request();
    append_component(&mut floating, 3.0, false);
    let error = solve_solid_tetra_3d(&floating)
        .expect_err("a floating second component should fail before factorization");
    assert!(
        error.contains("component 1") && error.contains("rigid-body rank 0/6"),
        "unexpected floating-component error: {error}",
    );
}

#[test]
fn solid_tetra_solves_multiple_independently_restrained_components() {
    let mut request = tetra_request();
    append_component(&mut request, 3.0, true);
    let result = solve_solid_tetra_3d(&request)
        .expect("two independently restrained components should solve as one block system");

    assert_eq!(result.quality.connected_component_count, 2);
    assert_eq!(result.nodes.len(), 8);
    assert_eq!(result.elements.len(), 2);
    assert!(result.equilibrium.free_residual_relative_error < 1.0e-12);
    assert!(result.equilibrium.force_balance_relative_error < 1.0e-12);
}

#[test]
fn solid_tetra_response_is_invariant_to_node_and_element_index_order() {
    let baseline = solve_solid_tetra_3d(&tetra_request()).expect("baseline should solve");
    let mut reordered = tetra_request();
    let permutation = [3, 1, 0, 2];
    let old_nodes = reordered.nodes.clone();
    reordered.nodes = permutation.map(|old| old_nodes[old].clone()).to_vec();
    let mut old_to_new = [0; 4];
    for (new, old) in permutation.into_iter().enumerate() {
        old_to_new[old] = new;
    }
    let element = &mut reordered.elements[0];
    element.node_a = old_to_new[element.node_a];
    element.node_b = old_to_new[element.node_b];
    element.node_c = old_to_new[element.node_c];
    element.node_d = old_to_new[element.node_d];
    let reordered = solve_solid_tetra_3d(&reordered).expect("reordered mesh should solve");

    for baseline_node in &baseline.nodes {
        let reordered_node = reordered
            .nodes
            .iter()
            .find(|node| node.id == baseline_node.id)
            .expect("node id should survive reindexing");
        assert_close(reordered_node.ux, baseline_node.ux);
        assert_close(reordered_node.uy, baseline_node.uy);
        assert_close(reordered_node.uz, baseline_node.uz);
    }
    assert_close(
        reordered.max_von_mises_stress,
        baseline.max_von_mises_stress,
    );
    assert_close(reordered.total_strain_energy, baseline.total_strain_energy);
}

fn tetra_request() -> SolveSolidTetra3dRequest {
    SolveSolidTetra3dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0, true),
            node("n1", 1.0, 0.0, 0.0, true),
            node("n2", 0.0, 1.0, 0.0, true),
            node("n3", 0.0, 0.0, 1.0, false),
        ],
        elements: vec![element("t0", 0)],
    }
}

fn append_component(request: &mut SolveSolidTetra3dRequest, offset_x: f64, restrained: bool) {
    let start = request.nodes.len();
    request.nodes.extend([
        node("m0", offset_x, 0.0, 0.0, restrained),
        node("m1", offset_x + 1.0, 0.0, 0.0, restrained),
        node("m2", offset_x, 1.0, 0.0, restrained),
        node("m3", offset_x, 0.0, 1.0, false),
    ]);
    request.elements.push(element("t1", start));
}

fn element(id: &str, start: usize) -> SolidTetra3dElementInput {
    SolidTetra3dElementInput {
        id: id.to_string(),
        node_a: start,
        node_b: start + 1,
        node_c: start + 2,
        node_d: start + 3,
        youngs_modulus: 70.0e9,
        poisson_ratio: 0.31,
    }
}

fn node(id: &str, x: f64, y: f64, z: f64, fixed: bool) -> SolidTetra3dNodeInput {
    SolidTetra3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fix_x: fixed,
        fix_y: fixed,
        fix_z: fixed,
        load_x: 0.0,
        load_y: 0.0,
        load_z: if fixed { 0.0 } else { -1000.0 },
    }
}

fn assert_close(actual: f64, expected: f64) {
    let scale = expected.abs().max(1.0);
    assert!((actual - expected).abs() <= 1.0e-10 * scale);
}
