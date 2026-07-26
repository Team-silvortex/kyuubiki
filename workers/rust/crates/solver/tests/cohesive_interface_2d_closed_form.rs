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

fn assert_force_balance(forces: [[f64; 2]; 4]) {
    assert_close(forces.iter().map(|force| force[0]).sum(), 0.0);
    assert_close(forces.iter().map(|force| force[1]).sum(), 0.0);
}

fn assert_close(actual: f64, expected: f64) {
    let tolerance = 1.0e-10 * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}
