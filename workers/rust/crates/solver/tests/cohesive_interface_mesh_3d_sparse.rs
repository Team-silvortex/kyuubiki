use kyuubiki_protocol::{
    CohesiveInterface3dMaterialInput, CohesiveInterfaceMesh3dElementInput,
    CohesiveInterfaceMesh3dMaterialInput, CohesiveInterfaceMesh3dNodeInput,
    SolveCohesiveInterfaceMesh3dRequest,
};
use kyuubiki_solver::solve_cohesive_interface_mesh_3d;

const ELEMENT_COUNT: usize = 80;

#[test]
fn block_interface_reports_the_retained_sparse_global_shape() {
    let result = solve_cohesive_interface_mesh_3d(&block_request(ELEMENT_COUNT))
        .expect("block interface should solve");

    assert!(result.converged);
    assert_eq!(result.nodes.len(), 480);
    assert_eq!(result.elements.len(), ELEMENT_COUNT);
    assert_eq!(result.max_tangent_non_zero_count, 8_640);
    assert!((result.max_tangent_fill_ratio - 1.0 / 240.0).abs() < 1.0e-12);
    assert_eq!(result.linear_solver_methods, ["symmetric_band_cholesky"]);
    assert!((result.max_displacement - 0.001).abs() < 1.0e-12);
}

fn block_request(element_count: usize) -> SolveCohesiveInterfaceMesh3dRequest {
    let mut nodes = Vec::with_capacity(element_count * 6);
    let mut elements = Vec::with_capacity(element_count);
    for element in 0..element_count {
        let origin = element as f64 * 2.0;
        let base = nodes.len();
        for (suffix, x, y, upper) in [
            ("lower-a", origin, 0.0, false),
            ("lower-b", origin + 1.0, 0.0, false),
            ("lower-c", origin, 1.0, false),
            ("upper-a", origin, 0.0, true),
            ("upper-b", origin + 1.0, 0.0, true),
            ("upper-c", origin, 1.0, true),
        ] {
            nodes.push(CohesiveInterfaceMesh3dNodeInput {
                id: format!("{element}-{suffix}"),
                x,
                y,
                z: 0.0,
                fixed: if upper {
                    [true, true, false]
                } else {
                    [true; 3]
                },
                prescribed_displacement: None,
                load: if upper {
                    [0.0, 0.0, 1000.0 * 0.001 * 0.5 / 3.0]
                } else {
                    [0.0; 3]
                },
            });
        }
        elements.push(CohesiveInterfaceMesh3dElementInput {
            id: format!("interface-{element}"),
            lower_a: base,
            lower_b: base + 1,
            lower_c: base + 2,
            upper_a: base + 3,
            upper_b: base + 4,
            upper_c: base + 5,
            material_id: "adhesive".to_string(),
        });
    }
    SolveCohesiveInterfaceMesh3dRequest {
        id: "sparse-block".to_string(),
        nodes,
        materials: vec![CohesiveInterfaceMesh3dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface3dMaterialInput {
                normal_initial_stiffness: 1000.0,
                normal_compression_stiffness: 2000.0,
                normal_peak_traction: 100.0,
                normal_failure_separation: 1.0,
                shear_initial_stiffness: 500.0,
                shear_peak_traction: 50.0,
                shear_failure_separation: 1.0,
            },
        }],
        elements,
        host_tetrahedra: Vec::new(),
        load_steps: Some(1),
        control_history: None,
        max_iterations: Some(8),
        tolerance: Some(1.0e-11),
    }
}
