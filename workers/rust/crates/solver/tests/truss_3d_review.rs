use kyuubiki_protocol::{
    SolveTruss3dRequest, Truss3dElementInput, Truss3dElementResult, Truss3dNodeInput,
};
use kyuubiki_solver::solve_truss_3d;

const TOL: f64 = 1.0e-7;

#[test]
fn truss_3d_review_bundle_checks_supports_member_forces_and_loaded_node_balance() {
    let request = SolveTruss3dRequest {
        nodes: vec![
            node("base_0", 0.0, 0.0, 0.0, true, true, true, 0.0),
            node("base_1", 1.2, 0.0, 0.0, true, true, true, 0.0),
            node("base_2", 0.0, 1.2, 0.0, true, true, true, 0.0),
            node("loaded_top", 0.35, 0.35, 1.0, false, false, false, -1600.0),
        ],
        elements: vec![
            element("base_01", 0, 1),
            element("base_12", 1, 2),
            element("base_20", 2, 0),
            element("leg_0", 0, 3),
            element("leg_1", 1, 3),
            element("leg_2", 2, 3),
        ],
    };

    let result = solve_truss_3d(&request).expect("review 3d truss should solve");

    assert_eq!(result.nodes.len(), 4);
    assert_eq!(result.elements.len(), 6);
    for (index, support) in result.nodes[0..3].iter().enumerate() {
        assert_eq!(support.index, index);
        assert_close(support.ux, 0.0, 1.0e-12);
        assert_close(support.uy, 0.0, 1.0e-12);
        assert_close(support.uz, 0.0, 1.0e-12);
    }

    let loaded = &result.nodes[3];
    assert_close(loaded.ux, 2.897_530_666_749_509e-7, 1.0e-12);
    assert_close(loaded.uy, 2.897_530_666_749_509e-7, 1.0e-12);
    assert_close(loaded.uz, -0.000_001_525_842_024_648_877_3, 1.0e-12);
    assert_close(
        result.max_displacement,
        0.000_001_579_907_454_086_998_8,
        1.0e-12,
    );
    assert_close(result.max_stress, 74_386.378_681_404_68, 1.0e-9);
    assert!(result.total_strain_energy > 0.0);
    assert!(result.max_strain_energy_density > 0.0);

    for (index, element) in result.elements.iter().enumerate() {
        assert_eq!(element.index, index);
        assert!(element.length > 0.0);
        assert!(element.strain.is_finite());
        assert!(element.stress.is_finite());
        assert!(element.axial_force.is_finite());
        assert_close(
            element.strain_energy_density,
            0.5 * element.stress * element.strain,
            1.0e-12,
        );
    }
    assert_close(
        result.total_strain_energy,
        total_strain_energy(&request, &result.elements),
        TOL,
    );
    for (index, base_element) in result.elements[0..3].iter().enumerate() {
        assert_eq!(base_element.index, index);
        assert_close(base_element.strain, 0.0, 1.0e-12);
        assert_close(base_element.stress, 0.0, 1.0e-12);
        assert_close(base_element.axial_force, 0.0, 1.0e-12);
    }
    assert_close(result.elements[3].stress, -74_386.378_681_404_68, 1.0e-9);
    assert_close(result.elements[4].stress, -63_387.695_966_961_9, 1.0e-9);
    assert_close(result.elements[5].stress, -63_387.695_966_961_9, 1.0e-9);

    let (internal_x, internal_y, internal_z) =
        loaded_node_internal_force(&request, &result.elements, 3);
    assert_close(internal_x + request.nodes[3].load_x, 0.0, TOL);
    assert_close(internal_y + request.nodes[3].load_y, 0.0, TOL);
    assert_close(internal_z + request.nodes[3].load_z, 0.0, TOL);
}

fn total_strain_energy(request: &SolveTruss3dRequest, elements: &[Truss3dElementResult]) -> f64 {
    elements
        .iter()
        .zip(request.elements.iter())
        .map(|(element, input)| element.strain_energy_density * input.area * element.length)
        .sum()
}

fn loaded_node_internal_force(
    request: &SolveTruss3dRequest,
    elements: &[Truss3dElementResult],
    node_index: usize,
) -> (f64, f64, f64) {
    let mut force_x = 0.0;
    let mut force_y = 0.0;
    let mut force_z = 0.0;
    for element in elements {
        if element.node_i != node_index && element.node_j != node_index {
            continue;
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let l = dx / length;
        let m = dy / length;
        let n = dz / length;
        let sign = if element.node_i == node_index {
            1.0
        } else {
            -1.0
        };
        force_x += sign * element.axial_force * l;
        force_y += sign * element.axial_force * m;
        force_z += sign * element.axial_force * n;
    }
    (force_x, force_y, force_z)
}

fn node(
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

fn element(id: &str, node_i: usize, node_j: usize) -> Truss3dElementInput {
    Truss3dElementInput {
        id: id.to_string(),
        node_i,
        node_j,
        area: 0.01,
        youngs_modulus: 70.0e9,
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance * scale,
        "expected {actual} to be close to {expected}",
    );
}
