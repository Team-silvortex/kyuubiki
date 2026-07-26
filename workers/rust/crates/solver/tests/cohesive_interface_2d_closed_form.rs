use kyuubiki_protocol::{
    CohesiveInterface2dDisplacementStepInput, CohesiveInterface2dElementInput,
    CohesiveInterface2dMaterialInput, CohesiveInterface2dNodeInput, CohesiveTractionRegime,
    SolveCohesiveInterface2dRequest,
};
use kyuubiki_solver::solve_cohesive_interface_2d;

#[test]
fn horizontal_element_matches_normal_and_shear_closed_forms() {
    let result = solve_cohesive_interface_2d(&horizontal_request(vec![
        displacement_step([0.0, 0.03]),
        displacement_step([0.03, 0.03]),
    ]))
    .expect("2d cohesive interface should solve");

    assert_close(result.interface_length, 1.0);
    assert_close(result.interface_area, 2.0);
    assert_eq!(result.local_tangent_direction, [1.0, 0.0]);
    assert_eq!(result.local_normal_direction, [0.0, 1.0]);

    let normal = &result.steps[0];
    assert_close(normal.local_separation[1], 0.03);
    assert_close(normal.local_traction[1], 5.0);
    assert_close(normal.local_tangent[1], -250.0);
    assert_close(normal.element_nodal_internal_forces[0][1], -5.0);
    assert_close(normal.element_nodal_internal_forces[2][1], 5.0);
    assert_force_balance(normal.element_nodal_internal_forces);

    let combined = &result.steps[1];
    assert_close(combined.local_traction[0], 2.5);
    assert_close(combined.local_traction[1], 5.0);
    assert_close(combined.global_traction[0], 2.5);
    assert_close(combined.global_traction[1], 5.0);
    assert_force_balance(combined.element_nodal_internal_forces);
}

#[test]
fn rotated_element_preserves_local_response_and_rotates_global_force() {
    let mut request = horizontal_request(vec![displacement_step([-0.03, 0.03])]);
    request.nodes = nodes([[0.0, 0.0], [0.0, 1.0], [0.0, 0.0], [0.0, 1.0]]);
    let result =
        solve_cohesive_interface_2d(&request).expect("rotated cohesive interface should solve");
    let step = &result.steps[0];

    assert_eq!(result.local_tangent_direction, [0.0, 1.0]);
    assert_eq!(result.local_normal_direction, [-1.0, 0.0]);
    assert_close(step.local_separation[0], 0.03);
    assert_close(step.local_separation[1], 0.03);
    assert_close(step.global_traction[0], -5.0);
    assert_close(step.global_traction[1], 2.5);
    assert_force_balance(step.element_nodal_internal_forces);
}

#[test]
fn directional_damage_is_independent_and_frozen_on_unloading() {
    let result = solve_cohesive_interface_2d(&horizontal_request(vec![
        displacement_step([0.03, 0.03]),
        displacement_step([0.015, 0.015]),
    ]))
    .expect("cyclic 2d cohesive interface should solve");

    let loaded = &result.steps[0];
    let unloaded = &result.steps[1];
    assert_close(unloaded.shear_damage, loaded.shear_damage);
    assert_close(unloaded.normal_damage, loaded.normal_damage);
    assert_eq!(
        unloaded.shear_regime,
        CohesiveTractionRegime::UnloadingReloading
    );
    assert_eq!(
        unloaded.normal_regime,
        CohesiveTractionRegime::UnloadingReloading
    );
    assert_close(
        unloaded.local_traction[0],
        unloaded.local_tangent[0] * unloaded.local_separation[0],
    );
    assert_close(
        unloaded.local_traction[1],
        unloaded.local_tangent[1] * unloaded.local_separation[1],
    );
}

#[test]
fn rejects_open_initial_geometry_and_incomplete_displacement_steps() {
    let mut open_geometry = horizontal_request(vec![displacement_step([0.0, 0.01])]);
    open_geometry.nodes[2].y = 0.001;
    assert!(solve_cohesive_interface_2d(&open_geometry).is_err());

    let mut incomplete = horizontal_request(vec![displacement_step([0.0, 0.01])]);
    incomplete.displacement_history[0].nodal_displacements.pop();
    assert!(solve_cohesive_interface_2d(&incomplete).is_err());

    let mut repeated_node = horizontal_request(vec![displacement_step([0.0, 0.01])]);
    repeated_node.element.upper_i = repeated_node.element.lower_i;
    assert!(solve_cohesive_interface_2d(&repeated_node).is_err());
}

#[test]
fn two_point_integration_resists_antisymmetric_jump_mode() {
    let result = solve_cohesive_interface_2d(&horizontal_request(vec![nonuniform_step(
        [0.02, 0.0],
        [-0.02, 0.0],
    )]))
    .expect("antisymmetric jump should retain element stiffness");
    let step = &result.steps[0];

    assert_eq!(step.integration_points.len(), 2);
    assert_close(step.local_separation[0], 0.0);
    assert_close(step.global_traction[0], 0.0);
    assert!(
        step.element_nodal_internal_forces
            .iter()
            .any(|force| force[0].abs() > 1.0e-6)
    );
    assert!(
        step.integration_points[0].local_separation[0]
            * step.integration_points[1].local_separation[0]
            < 0.0
    );
    assert_force_balance(step.element_nodal_internal_forces);
}

#[test]
fn rigid_translation_produces_zero_jump_force_and_traction() {
    let rigid = CohesiveInterface2dDisplacementStepInput {
        nodal_displacements: vec![[1.25, -0.75]; 4],
    };
    let result = solve_cohesive_interface_2d(&horizontal_request(vec![rigid]))
        .expect("rigid translation should solve");
    let step = &result.steps[0];

    for value in step
        .local_separation
        .iter()
        .chain(step.local_traction.iter())
        .chain(step.global_traction.iter())
        .chain(step.element_nodal_internal_forces.iter().flatten())
    {
        assert_close(*value, 0.0);
    }
}

#[test]
fn every_assembled_tangent_column_matches_nodal_force_central_difference() {
    let base_displacement = 0.04;
    let perturbation = 1.0e-7;
    let base = solve_with_dof_perturbation(base_displacement, None);
    let base_step = &base.steps[0];
    for column in 0..8 {
        let plus = solve_with_dof_perturbation(base_displacement, Some((column, perturbation)));
        let minus = solve_with_dof_perturbation(base_displacement, Some((column, -perturbation)));
        let plus_forces = flatten_forces(plus.steps[0].element_nodal_internal_forces);
        let minus_forces = flatten_forces(minus.steps[0].element_nodal_internal_forces);
        for row in 0..8 {
            let numerical = (plus_forces[row] - minus_forces[row]) / (2.0 * perturbation);
            assert_close_with_tolerance(base_step.element_tangent[row][column], numerical, 2.0e-6);
        }
    }
}

fn horizontal_request(
    displacement_history: Vec<CohesiveInterface2dDisplacementStepInput>,
) -> SolveCohesiveInterface2dRequest {
    SolveCohesiveInterface2dRequest {
        nodes: nodes([[0.0, 0.0], [1.0, 0.0], [0.0, 0.0], [1.0, 0.0]]),
        element: CohesiveInterface2dElementInput {
            id: "interface-0".to_string(),
            lower_i: 0,
            lower_j: 1,
            upper_i: 2,
            upper_j: 3,
            thickness: 2.0,
        },
        material: CohesiveInterface2dMaterialInput {
            normal_initial_stiffness: 1_000.0,
            normal_compression_stiffness: 2_000.0,
            normal_peak_traction: 10.0,
            normal_failure_separation: 0.05,
            shear_initial_stiffness: 500.0,
            shear_peak_traction: 5.0,
            shear_failure_separation: 0.05,
        },
        displacement_history,
    }
}

fn nodes(coordinates: [[f64; 2]; 4]) -> Vec<CohesiveInterface2dNodeInput> {
    coordinates
        .into_iter()
        .enumerate()
        .map(|(index, coordinate)| CohesiveInterface2dNodeInput {
            id: format!("n{index}"),
            x: coordinate[0],
            y: coordinate[1],
        })
        .collect()
}

fn displacement_step(upper_jump: [f64; 2]) -> CohesiveInterface2dDisplacementStepInput {
    CohesiveInterface2dDisplacementStepInput {
        nodal_displacements: vec![[0.0, 0.0], [0.0, 0.0], upper_jump, upper_jump],
    }
}

fn nonuniform_step(
    upper_i: [f64; 2],
    upper_j: [f64; 2],
) -> CohesiveInterface2dDisplacementStepInput {
    CohesiveInterface2dDisplacementStepInput {
        nodal_displacements: vec![[0.0, 0.0], [0.0, 0.0], upper_i, upper_j],
    }
}

fn solve_with_dof_perturbation(
    upper_j_opening: f64,
    perturbation: Option<(usize, f64)>,
) -> kyuubiki_protocol::SolveCohesiveInterface2dResult {
    let mut step = nonuniform_step([0.0, 0.0], [0.0, upper_j_opening]);
    if let Some((dof, value)) = perturbation {
        step.nodal_displacements[dof / 2][dof % 2] += value;
    }
    solve_cohesive_interface_2d(&horizontal_request(vec![step]))
        .expect("nonuniform opening should solve")
}

fn flatten_forces(forces: [[f64; 2]; 4]) -> [f64; 8] {
    [
        forces[0][0],
        forces[0][1],
        forces[1][0],
        forces[1][1],
        forces[2][0],
        forces[2][1],
        forces[3][0],
        forces[3][1],
    ]
}

fn assert_force_balance(forces: [[f64; 2]; 4]) {
    assert_close(forces.iter().map(|force| force[0]).sum(), 0.0);
    assert_close(forces.iter().map(|force| force[1]).sum(), 0.0);
}

fn assert_close_with_tolerance(actual: f64, expected: f64, relative_tolerance: f64) {
    let tolerance = relative_tolerance * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}, tolerance {tolerance}"
    );
}

fn assert_close(actual: f64, expected: f64) {
    let tolerance = 1.0e-10 * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}
