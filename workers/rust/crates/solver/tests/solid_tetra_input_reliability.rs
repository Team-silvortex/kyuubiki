use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
};
use kyuubiki_solver::solve_solid_tetra_3d;

#[test]
fn solid_tetra_3d_rejects_non_finite_node_coordinates_and_loads() {
    let mut request = tetra_request();
    request.nodes[3].z = f64::NAN;
    assert!(solve_solid_tetra_3d(&request).is_err());

    let mut request = tetra_request();
    request.nodes[3].load_z = f64::INFINITY;
    assert!(solve_solid_tetra_3d(&request).is_err());
}

#[test]
fn solid_tetra_3d_rejects_invalid_topology_and_material_bounds() {
    let mut request = tetra_request();
    request.elements[0].node_d = 9;
    assert!(solve_solid_tetra_3d(&request).is_err());

    let mut request = tetra_request();
    request.elements[0].node_d = request.elements[0].node_a;
    assert!(solve_solid_tetra_3d(&request).is_err());

    let mut request = tetra_request();
    request.elements[0].poisson_ratio = 0.5;
    assert!(solve_solid_tetra_3d(&request).is_err());

    let mut request = tetra_request();
    request.elements[0].poisson_ratio = f64::NAN;
    assert!(solve_solid_tetra_3d(&request).is_err());
}

fn tetra_request() -> SolveSolidTetra3dRequest {
    SolveSolidTetra3dRequest {
        nodes: vec![
            node("n0", 0.0, 0.0, 0.0, true, 0.0),
            node("n1", 1.0, 0.0, 0.0, true, 0.0),
            node("n2", 0.0, 1.0, 0.0, true, 0.0),
            node("n3", 0.0, 0.0, 1.0, false, -1000.0),
        ],
        elements: vec![SolidTetra3dElementInput {
            id: "t0".to_string(),
            node_a: 0,
            node_b: 1,
            node_c: 2,
            node_d: 3,
            youngs_modulus: 70.0e9,
            poisson_ratio: 0.33,
        }],
    }
}

fn node(id: &str, x: f64, y: f64, z: f64, fixed: bool, load_z: f64) -> SolidTetra3dNodeInput {
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
        load_z,
    }
}
