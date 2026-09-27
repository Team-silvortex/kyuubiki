use kyuubiki_protocol::{
    SolidTetra3dElementInput, SolidTetra3dNodeInput, SolveSolidTetra3dRequest,
};
use kyuubiki_solver::solve_solid_tetra_3d;

fn request(offset: f64) -> SolveSolidTetra3dRequest {
    let nodes = [
        [0.0; 3],
        [1.0, 0.25, 0.0],
        [0.25, 1.25, 0.25],
        [0.0, 0.25, 1.5],
    ]
    .into_iter()
    .enumerate()
    .map(|(i, [x, y, z])| SolidTetra3dNodeInput {
        id: format!("n{i}"),
        x: x + offset,
        y: y - offset,
        z: z + offset,
        fix_x: i != 3,
        fix_y: i != 3,
        fix_z: i != 3,
        load_x: if i == 3 { 3.0 } else { 0.0 },
        load_y: if i == 3 { -2.0 } else { 0.0 },
        load_z: if i == 3 { 1.0 } else { 0.0 },
    })
    .collect();
    SolveSolidTetra3dRequest {
        nodes,
        elements: vec![SolidTetra3dElementInput {
            id: "bulk".into(),
            node_a: 0,
            node_b: 1,
            node_c: 2,
            node_d: 3,
            youngs_modulus: 1200.0,
            poisson_ratio: 0.25,
        }],
    }
}

#[test]
fn solid_operator_translation_preserves_displacements_stress_energy_and_reactions() {
    let base = solve_solid_tetra_3d(&request(0.0)).unwrap();
    for offset in [2f64.powi(20), -2f64.powi(20), 2f64.powi(40), -2f64.powi(40)] {
        let result = solve_solid_tetra_3d(&request(offset)).expect("translated model must solve");
        for (actual, expected) in [
            (result.total_volume, base.total_volume),
            (result.total_strain_energy, base.total_strain_energy),
            (result.max_von_mises_stress, base.max_von_mises_stress),
        ] {
            close(actual, expected);
        }
        for (actual, expected) in result.nodes.iter().zip(&base.nodes) {
            for (a, b) in [
                (actual.ux, expected.ux),
                (actual.uy, expected.uy),
                (actual.uz, expected.uz),
                (actual.reaction_x, expected.reaction_x),
                (actual.reaction_y, expected.reaction_y),
                (actual.reaction_z, expected.reaction_z),
            ] {
                close(a, b);
            }
        }
        assert!(result.equilibrium.free_residual_relative_error < 1e-12);
        assert!(result.equilibrium.force_balance_relative_error < 1e-12);
    }
}

#[test]
fn solid_operator_retains_representable_huge_volume_and_rejects_overflow_then_replays() {
    let mut model = request(0.0);
    for (node, point) in model.nodes.iter_mut().zip([
        [0.0; 3],
        [1e103, 0.0, 0.0],
        [0.0, 1e103, 0.0],
        [0.0, 0.0, 1e103],
    ]) {
        [node.x, node.y, node.z] = point;
        node.fix_x = true;
        node.fix_y = true;
        node.fix_z = true;
        node.load_x = 0.0;
        node.load_y = 0.0;
        node.load_z = 0.0;
    }
    let result = solve_solid_tetra_3d(&model)
        .expect("finite volume must not overflow before division by six");
    assert!((result.total_volume / 1e308 - 5.0 / 3.0).abs() < 1e-13);
    assert_eq!(result.total_strain_energy, 0.0);
    for node in &mut model.nodes {
        node.x *= 2.0;
        node.y *= 2.0;
        node.z *= 2.0;
    }
    assert!(solve_solid_tetra_3d(&model).is_err());
    assert!(solve_solid_tetra_3d(&request(0.0)).is_ok());
}

fn close(actual: f64, expected: f64) {
    assert!(
        actual.is_finite() && (actual - expected).abs() < 2e-11 * expected.abs().max(1e-9),
        "actual={actual:e} expected={expected:e}"
    );
}
