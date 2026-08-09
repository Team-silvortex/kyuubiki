use kyuubiki_protocol::{
    CohesiveInterface3dMaterialInput, CohesiveInterfaceMesh3dControlStepInput,
    CohesiveInterfaceMesh3dElementInput, CohesiveInterfaceMesh3dMaterialInput,
    CohesiveInterfaceMesh3dNodeInput, CohesiveTractionRegime, SolidTetra3dElementInput,
    SolveCohesiveInterfaceMesh3dRequest,
};
use kyuubiki_solver::solve_cohesive_interface_mesh_3d;

const TOLERANCE: f64 = 1.0e-9;

#[test]
fn uniform_normal_opening_matches_the_triangular_interface_closed_form() {
    let area = 0.5;
    let separation = 0.005;
    let traction = 5.0;
    let nodal_load = traction * area / 3.0;
    let mut request = base_request();
    for node in &mut request.nodes[3..6] {
        node.fixed = [true, true, false];
        node.load[2] = nodal_load;
    }

    let result = solve_cohesive_interface_mesh_3d(&request).expect("uniform opening should solve");

    assert!(result.converged);
    assert_close(result.elements[0].area, area);
    assert_close(result.elements[0].local_separation[2], separation);
    assert_close(result.elements[0].local_traction[2], traction);
    assert_close(result.max_resultant_traction, traction);
    for node in &result.nodes[3..6] {
        assert_close(node.displacement[2], separation);
    }
    for node in &result.nodes[..3] {
        assert_close(node.reaction[2], -nodal_load);
    }
    assert_eq!(
        result.linear_solver_methods,
        vec!["symmetric_band_cholesky"]
    );
    assert!(result.max_tangent_non_zero_count > 0);
    assert!(result.max_tangent_fill_ratio > 0.0);
}

#[test]
fn committed_damage_survives_a_prescribed_unloading_path() {
    let mut request = base_request();
    request.materials[0].properties.normal_peak_traction = 10.0;
    request.materials[0].properties.normal_failure_separation = 0.03;
    request.load_steps = None;
    request.control_history = Some(vec![
        prescribed_step(0.005),
        prescribed_step(0.02),
        prescribed_step(0.005),
    ]);

    let result = solve_cohesive_interface_mesh_3d(&request).expect("history should solve");

    assert!(result.converged);
    assert_close(result.max_normal_damage, 0.75);
    assert_close(result.elements[0].max_normal_damage, 0.75);
    assert_close(result.elements[0].local_traction[2], 1.25);
    assert!(
        result.elements[0]
            .integration_points
            .iter()
            .all(|point| point.regimes[2] == CohesiveTractionRegime::UnloadingReloading)
    );
}

#[test]
fn tetra_host_and_interface_share_one_global_equilibrium_system() {
    let mut request = base_request();
    request.nodes.push(node("apex", 0.0, 0.0, 1.0, [true; 3]));
    request.nodes[3].fixed = [true, true, false];
    request.nodes[3].load[2] = 2.5;
    request.host_tetrahedra.push(SolidTetra3dElementInput {
        id: "host-tetra".to_string(),
        node_a: 3,
        node_b: 4,
        node_c: 5,
        node_d: 6,
        youngs_modulus: 1000.0,
        poisson_ratio: 0.0,
    });

    let result = solve_cohesive_interface_mesh_3d(&request)
        .expect("cohesive and solid host coassembly should solve");

    let expected_displacement = 2.5 / (1000.0 * 0.5 / 6.0 + 1000.0 / 3.0);
    assert!(result.converged);
    assert_close(result.nodes[3].displacement[2], expected_displacement);
    assert_close(
        result.elements[0].local_separation[2],
        expected_displacement / 3.0,
    );
    assert_close(result.elements[0].local_traction[2], 2.0);
    assert_close(result.host_tetrahedra[0].strain_z, -expected_displacement);
    assert_close(result.host_tetrahedra[0].stress_z, -6.0);
    assert_close(result.host_tetrahedra[0].shear_yz, -3.0);
    assert_close(result.host_tetrahedra[0].shear_zx, -3.0);
    assert_close(result.max_host_von_mises_stress, 90.0_f64.sqrt());
}

#[test]
fn local_basis_tracks_a_rotated_interface_without_changing_the_normal_response() {
    let mut request = base_request();
    let rotated_points = [
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];
    for (node, point) in request.nodes.iter_mut().zip(rotated_points) {
        [node.x, node.y, node.z] = point;
    }
    for node in &mut request.nodes[3..6] {
        node.fixed = [false, true, true];
        node.load[0] = 5.0 * 0.5 / 3.0;
    }

    let result = solve_cohesive_interface_mesh_3d(&request).expect("rotated opening should solve");

    assert_close(result.elements[0].local_normal_direction[0], 1.0);
    assert_close(result.elements[0].local_separation[2], 0.005);
    assert_close(result.elements[0].local_traction[2], 5.0);
    for node in &result.nodes[3..6] {
        assert_close(node.displacement[0], 0.005);
    }
}

#[test]
fn invalid_geometry_material_and_host_connectivity_fail_before_iteration() {
    let mut separated = base_request();
    separated.nodes[3].z = 0.1;
    let error = solve_cohesive_interface_mesh_3d(&separated)
        .expect_err("initially separated node pairs must fail");
    assert!(error.contains("initially coincide"));

    let mut unknown_material = base_request();
    unknown_material.elements[0].material_id = "missing".to_string();
    let error = solve_cohesive_interface_mesh_3d(&unknown_material)
        .expect_err("unknown material must fail");
    assert!(error.contains("unknown material"));

    let mut invalid_unused_material = base_request();
    let mut unused = invalid_unused_material.materials[0].clone();
    unused.id = "invalid-unused".to_string();
    unused.properties.normal_initial_stiffness = 0.0;
    invalid_unused_material.materials.push(unused);
    let error = solve_cohesive_interface_mesh_3d(&invalid_unused_material)
        .expect_err("invalid unused material must fail");
    assert!(error.contains("initial_stiffness"));

    let mut invalid_host = base_request();
    invalid_host.host_tetrahedra.push(SolidTetra3dElementInput {
        id: "bad-host".to_string(),
        node_a: 0,
        node_b: 1,
        node_c: 2,
        node_d: 99,
        youngs_modulus: 1000.0,
        poisson_ratio: 0.0,
    });
    let error =
        solve_cohesive_interface_mesh_3d(&invalid_host).expect_err("missing host node must fail");
    assert!(error.contains("missing cohesive mesh node"));
}

fn base_request() -> SolveCohesiveInterfaceMesh3dRequest {
    SolveCohesiveInterfaceMesh3dRequest {
        id: "triangular-interface".to_string(),
        nodes: vec![
            node("lower-a", 0.0, 0.0, 0.0, [true; 3]),
            node("lower-b", 1.0, 0.0, 0.0, [true; 3]),
            node("lower-c", 0.0, 1.0, 0.0, [true; 3]),
            node("upper-a", 0.0, 0.0, 0.0, [true; 3]),
            node("upper-b", 1.0, 0.0, 0.0, [true; 3]),
            node("upper-c", 0.0, 1.0, 0.0, [true; 3]),
        ],
        materials: vec![CohesiveInterfaceMesh3dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface3dMaterialInput {
                normal_initial_stiffness: 1000.0,
                normal_compression_stiffness: 1200.0,
                normal_peak_traction: 100.0,
                normal_failure_separation: 1.0,
                shear_initial_stiffness: 800.0,
                shear_peak_traction: 80.0,
                shear_failure_separation: 1.0,
            },
        }],
        elements: vec![CohesiveInterfaceMesh3dElementInput {
            id: "interface-1".to_string(),
            lower_a: 0,
            lower_b: 1,
            lower_c: 2,
            upper_a: 3,
            upper_b: 4,
            upper_c: 5,
            material_id: "adhesive".to_string(),
        }],
        host_tetrahedra: Vec::new(),
        load_steps: Some(1),
        control_history: None,
        max_iterations: None,
        tolerance: Some(1.0e-11),
    }
}

fn prescribed_step(opening: f64) -> CohesiveInterfaceMesh3dControlStepInput {
    let mut prescribed_displacements = vec![[0.0; 3]; 6];
    for displacement in &mut prescribed_displacements[3..6] {
        displacement[2] = opening;
    }
    CohesiveInterfaceMesh3dControlStepInput {
        load_factor: 0.0,
        prescribed_displacements,
    }
}

fn node(id: &str, x: f64, y: f64, z: f64, fixed: [bool; 3]) -> CohesiveInterfaceMesh3dNodeInput {
    CohesiveInterfaceMesh3dNodeInput {
        id: id.to_string(),
        x,
        y,
        z,
        fixed,
        prescribed_displacement: None,
        load: [0.0; 3],
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= TOLERANCE * expected.abs().max(1.0),
        "expected {expected:.12e}, got {actual:.12e}"
    );
}
