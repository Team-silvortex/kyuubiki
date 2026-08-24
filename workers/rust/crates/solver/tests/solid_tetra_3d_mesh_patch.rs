use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
    SolveSolidTetra3dResult,
};
use kyuubiki_solver::solve_solid_tetra_3d;

const LENGTH_X: f64 = 1.2;
const LENGTH_Y: f64 = 0.7;
const LENGTH_Z: f64 = 0.5;
const YOUNGS_MODULUS: f64 = 210.0e9;
const POISSON_RATIO: f64 = 0.29;
const APPLIED_STRESS: f64 = 125.0e6;
const RELATIVE_TOLERANCE: f64 = 2.0e-8;

#[test]
fn solid_tetra_3d_multi_element_patch_is_exact_and_balanced_across_refinement() {
    for divisions in [1, 2, 4, 8] {
        let result = solve_solid_tetra_3d(&patch_request(divisions))
            .expect("multi-element solid tetra patch should solve");
        let strain_x = APPLIED_STRESS / YOUNGS_MODULUS;
        let strain_yz = -POISSON_RATIO * strain_x;
        let displacement_scale = strain_x * LENGTH_X;

        for node in &result.nodes {
            assert_scaled_close(node.ux, strain_x * node.x, displacement_scale);
            assert_scaled_close(node.uy, strain_yz * node.y, displacement_scale);
            assert_scaled_close(node.uz, strain_yz * node.z, displacement_scale);
        }
        for element in &result.elements {
            assert_scaled_close(element.strain_x, strain_x, strain_x);
            assert_scaled_close(element.strain_y, strain_yz, strain_x);
            assert_scaled_close(element.strain_z, strain_yz, strain_x);
            assert_scaled_close(element.gamma_xy, 0.0, strain_x);
            assert_scaled_close(element.gamma_yz, 0.0, strain_x);
            assert_scaled_close(element.gamma_zx, 0.0, strain_x);
            assert_scaled_close(element.stress_x, APPLIED_STRESS, APPLIED_STRESS);
            assert_scaled_close(element.stress_y, 0.0, APPLIED_STRESS);
            assert_scaled_close(element.stress_z, 0.0, APPLIED_STRESS);
            assert_scaled_close(element.shear_xy, 0.0, APPLIED_STRESS);
            assert_scaled_close(element.shear_yz, 0.0, APPLIED_STRESS);
            assert_scaled_close(element.shear_zx, 0.0, APPLIED_STRESS);
        }

        let volume = LENGTH_X * LENGTH_Y * LENGTH_Z;
        let applied_force = APPLIED_STRESS * LENGTH_Y * LENGTH_Z;
        let expected_energy = 0.5 * APPLIED_STRESS * strain_x * volume;
        assert_scaled_close(result.total_volume, volume, volume);
        assert_scaled_close(result.max_von_mises_stress, APPLIED_STRESS, APPLIED_STRESS);
        assert_scaled_close(result.total_strain_energy, expected_energy, expected_energy);
        assert_vector_close(
            result.equilibrium.applied_force,
            [applied_force, 0.0, 0.0],
            applied_force,
        );
        assert_vector_close(
            result.equilibrium.reaction_force,
            [-applied_force, 0.0, 0.0],
            applied_force,
        );
        assert_vector_close(result.equilibrium.balance_error, [0.0; 3], applied_force);
        assert!(
            result.equilibrium.free_residual_relative_error <= RELATIVE_TOLERANCE,
            "mesh {divisions} free residual was {}",
            result.equilibrium.free_residual_relative_error,
        );
        assert!(
            result.equilibrium.force_balance_relative_error <= RELATIVE_TOLERANCE,
            "mesh {divisions} force balance error was {}",
            result.equilibrium.force_balance_relative_error,
        );
        assert!(result.quality.minimum_mean_ratio_quality > 0.20);
        assert_eq!(result.quality.distorted_element_count, 0);
        assert_eq!(result.quality.severely_distorted_element_count, 0);
        assert_eq!(result.quality.near_incompressible_element_count, 0);
        assert!(result.quality.watch_terms.is_empty());
    }
}

#[test]
fn solid_tetra_3d_reports_near_incompressible_locking_risk() {
    let mut request = patch_request(1);
    for element in &mut request.elements {
        element.poisson_ratio = 0.49;
    }
    let result = solve_solid_tetra_3d(&request).expect("near-incompressible patch should solve");

    assert_eq!(
        result.quality.near_incompressible_element_count,
        request.elements.len(),
    );
    assert!(
        result
            .quality
            .watch_terms
            .iter()
            .any(|term| term == "near_incompressible_volumetric_locking_risk"),
    );
}

#[test]
fn solid_tetra_3d_equilibrium_fields_are_backward_deserialization_compatible() {
    let result = solve_solid_tetra_3d(&patch_request(1)).expect("patch should solve");
    let mut legacy = serde_json::to_value(result).expect("result should serialize");
    let object = legacy.as_object_mut().expect("result should be an object");
    object.remove("equilibrium");
    object.remove("quality");
    for node in object["nodes"]
        .as_array_mut()
        .expect("nodes should be an array")
    {
        let node = node.as_object_mut().expect("node should be an object");
        node.remove("reaction_x");
        node.remove("reaction_y");
        node.remove("reaction_z");
    }
    for element in object["elements"]
        .as_array_mut()
        .expect("elements should be an array")
    {
        element
            .as_object_mut()
            .expect("element should be an object")
            .remove("mean_ratio_quality");
    }

    let decoded: SolveSolidTetra3dResult =
        serde_json::from_value(legacy).expect("legacy result should still deserialize");
    assert_eq!(decoded.equilibrium, Default::default());
    assert_eq!(decoded.quality, Default::default());
    assert!(
        decoded
            .nodes
            .iter()
            .all(|node| node.reaction_x == 0.0 && node.reaction_y == 0.0 && node.reaction_z == 0.0)
    );
}

fn patch_request(divisions: usize) -> SolveSolidTetra3dRequest {
    let mut nodes = patch_nodes(divisions);
    let elements = patch_elements(divisions);
    apply_x_face_traction(&mut nodes, divisions);
    SolveSolidTetra3dRequest { nodes, elements }
}

fn patch_nodes(divisions: usize) -> Vec<SolidTetra3dNodeInput> {
    let mut nodes = Vec::with_capacity((divisions + 1).pow(3));
    for i in 0..=divisions {
        for j in 0..=divisions {
            for k in 0..=divisions {
                let origin = i == 0 && j == 0 && k == 0;
                let rotation_anchor = i == 0 && j == 0 && k == divisions;
                nodes.push(SolidTetra3dNodeInput {
                    id: format!("n-{i}-{j}-{k}"),
                    x: LENGTH_X * i as f64 / divisions as f64,
                    y: LENGTH_Y * j as f64 / divisions as f64,
                    z: LENGTH_Z * k as f64 / divisions as f64,
                    fix_x: i == 0,
                    fix_y: origin || rotation_anchor,
                    fix_z: origin,
                    load_x: 0.0,
                    load_y: 0.0,
                    load_z: 0.0,
                });
            }
        }
    }
    nodes
}

fn patch_elements(divisions: usize) -> Vec<SolidTetra3dElementInput> {
    let mut elements = Vec::with_capacity(6 * divisions.pow(3));
    for i in 0..divisions {
        for j in 0..divisions {
            for k in 0..divisions {
                let v000 = node_index(i, j, k, divisions);
                let v100 = node_index(i + 1, j, k, divisions);
                let v010 = node_index(i, j + 1, k, divisions);
                let v110 = node_index(i + 1, j + 1, k, divisions);
                let v001 = node_index(i, j, k + 1, divisions);
                let v101 = node_index(i + 1, j, k + 1, divisions);
                let v011 = node_index(i, j + 1, k + 1, divisions);
                let v111 = node_index(i + 1, j + 1, k + 1, divisions);
                for (local, [node_a, node_b, node_c, node_d]) in [
                    [v000, v100, v110, v111],
                    [v000, v110, v010, v111],
                    [v000, v010, v011, v111],
                    [v000, v011, v001, v111],
                    [v000, v001, v101, v111],
                    [v000, v101, v100, v111],
                ]
                .into_iter()
                .enumerate()
                {
                    elements.push(SolidTetra3dElementInput {
                        id: format!("t-{i}-{j}-{k}-{local}"),
                        node_a,
                        node_b,
                        node_c,
                        node_d,
                        youngs_modulus: YOUNGS_MODULUS,
                        poisson_ratio: POISSON_RATIO,
                    });
                }
            }
        }
    }
    elements
}

fn apply_x_face_traction(nodes: &mut [SolidTetra3dNodeInput], divisions: usize) {
    let triangle_force =
        APPLIED_STRESS * LENGTH_Y * LENGTH_Z / (2.0 * divisions.pow(2) as f64 * 3.0);
    for j in 0..divisions {
        for k in 0..divisions {
            let v00 = node_index(divisions, j, k, divisions);
            let v10 = node_index(divisions, j + 1, k, divisions);
            let v01 = node_index(divisions, j, k + 1, divisions);
            let v11 = node_index(divisions, j + 1, k + 1, divisions);
            for node in [v00, v10, v11, v00, v11, v01] {
                nodes[node].load_x += triangle_force;
            }
        }
    }
}

fn node_index(i: usize, j: usize, k: usize, divisions: usize) -> usize {
    let width = divisions + 1;
    (i * width + j) * width + k
}

fn assert_vector_close(actual: [f64; 3], expected: [f64; 3], scale: f64) {
    for axis in 0..3 {
        assert_scaled_close(actual[axis], expected[axis], scale);
    }
}

fn assert_scaled_close(actual: f64, expected: f64, scale: f64) {
    assert!(
        (actual - expected).abs() <= RELATIVE_TOLERANCE * scale.abs().max(1.0e-30),
        "expected {actual:.12e} to be close to {expected:.12e} at scale {scale:.12e}",
    );
}
