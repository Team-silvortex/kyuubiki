use kyuubiki_protocol::{
    CohesiveInterface2dMaterialInput, CohesiveInterfaceMesh2dControlStepInput,
    CohesiveInterfaceMesh2dElementInput, CohesiveInterfaceMesh2dMaterialInput,
    CohesiveInterfaceMesh2dNodeInput, SolveCohesiveInterfaceMesh2dRequest,
    SolveCohesiveInterfaceMesh2dResult,
};
use kyuubiki_solver::solve_cohesive_interface_mesh_2d;

use super::assert_close;

#[test]
fn support_loads_do_not_relax_free_equilibrium() {
    for support_load in [1.0e15, 1.0e180] {
        let mut request = request(1.0);
        request.nodes[0].load[1] = support_load;
        let result = solve(&request);
        assert!(result.converged, "{:?}", result.failure_reason);
        assert_close(result.nodes[2].displacement[1], 0.005);
        assert_close(result.nodes[3].displacement[1], 0.005);
        assert_close(result.elements[0].local_traction[1], 5.0);
        assert_close(result.nodes[0].reaction[1], -support_load - 2.5);
        assert!(result.steps[0].reaction_norm.is_finite());
    }
}

#[test]
fn equivalent_load_factor_parameterizations_preserve_the_solution() {
    for factor in [1.0e-180, 1.0e-15, 1.0, 1.0e15, 1.0e180] {
        let mut request = request(1.0);
        for node in &mut request.nodes[2..] {
            node.load[1] /= factor;
        }
        request.control_history = Some(vec![control(factor)]);
        let result = solve(&request);
        assert!(
            result.converged,
            "factor {factor}: {:?}",
            result.failure_reason
        );
        assert_close(result.nodes[2].displacement[1], 0.005);
        assert_close(result.nodes[3].displacement[1], 0.005);
        assert_close(result.elements[0].local_traction[1], 5.0);
        assert_eq!(result.completed_load_factor, factor);
    }
}

#[test]
fn large_finite_force_units_do_not_overflow_convergence_norms() {
    let scale = 1.0e160;
    let result = solve(&request(scale));
    assert!(result.converged, "{:?}", result.failure_reason);
    assert_close(result.nodes[2].displacement[1], 0.005);
    assert_close(result.nodes[3].displacement[1], 0.005);
    assert_close(result.steps[0].max_resultant_traction, 5.0 * scale);
    assert!(result.residual_norm.is_finite());
    assert!(result.residual_norm / scale <= 1.0e-10);
    assert!(result.steps[0].reaction_norm.is_finite());
}

#[test]
fn prescribed_motion_scales_equilibrium_without_external_free_loads() {
    let run = |scale| {
        let mut request = request(scale);
        for node in &mut request.nodes {
            node.load = [0.0; 2];
        }
        request.nodes[3].fixed = [true; 2];
        let mut step = control(0.0);
        step.prescribed_displacements[..2].fill([0.0, 0.005]);
        request.control_history = Some(vec![step]);
        solve(&request)
    };
    let reference = run(1.0);
    let result = run(1.0e160);
    assert!(reference.converged);
    assert!(reference.nodes[2].displacement[1] > 0.005);
    assert!(result.converged, "{:?}", result.failure_reason);
    for (node, expected) in result.nodes.iter().zip(&reference.nodes) {
        assert_close(node.displacement[1], expected.displacement[1]);
    }
    assert!(result.residual_norm.is_finite());
    assert!(result.residual_norm / 1.0e160 <= 1.0e-10);
    assert_close(result.max_normal_damage, 0.0);
}

#[test]
fn overflowing_load_step_restores_the_last_equilibrium() {
    let mut request = request(1.0);
    for node in &mut request.nodes[2..] {
        node.load[1] = 1.0e308;
    }
    request.control_history = Some(vec![control(1.0e-308)]);
    let prefix = solve(&request);
    assert!(prefix.converged);
    assert_close(prefix.nodes[2].displacement[1], 0.002);
    request.control_history.as_mut().unwrap().push(control(2.0));
    assert_rollback(&solve(&request), &prefix);
}

#[test]
fn overflowing_support_load_does_not_commit_a_new_damage_history() {
    let mut request = request(1.0);
    for node in &mut request.nodes {
        node.fixed = [true; 2];
        node.load = [0.0, 1.0e308];
    }
    let mut accepted = control(0.0);
    accepted.prescribed_displacements[2..].fill([0.0, 0.02]);
    request.control_history = Some(vec![accepted]);
    let prefix = solve(&request);
    assert!(prefix.converged);
    assert_close(prefix.max_normal_damage, 0.75);
    let mut rejected = control(2.0);
    rejected.prescribed_displacements[2..].fill([0.0, 0.03]);
    request.control_history.as_mut().unwrap().push(rejected);
    assert_rollback(&solve(&request), &prefix);
}

fn assert_rollback(
    result: &SolveCohesiveInterfaceMesh2dResult,
    prefix: &SolveCohesiveInterfaceMesh2dResult,
) {
    assert!(!result.converged);
    assert_eq!(result.steps.len(), 2);
    assert!(result.steps[0].converged);
    assert!(!result.steps[1].converged);
    assert_eq!(result.steps[1].reaction_norm, prefix.steps[0].reaction_norm);
    assert_eq!(
        result.steps[1].max_displacement,
        prefix.steps[0].max_displacement
    );
    assert!(result.residual_norm.is_finite());
    assert!(
        result
            .failure_reason
            .as_deref()
            .unwrap()
            .contains("non-finite")
    );
    assert_eq!(result.completed_load_factor, prefix.completed_load_factor);
    for (actual, expected) in result.nodes.iter().zip(&prefix.nodes) {
        assert_eq!(actual.displacement, expected.displacement);
        assert_eq!(actual.reaction, expected.reaction);
    }
    assert_eq!(
        result.elements[0].local_traction,
        prefix.elements[0].local_traction
    );
    assert_eq!(result.max_normal_damage, prefix.max_normal_damage);
    assert_eq!(
        result.elements[0].max_normal_damage,
        prefix.elements[0].max_normal_damage
    );
    let json = serde_json::to_string(result).expect("failed result should serialize");
    let decoded: SolveCohesiveInterfaceMesh2dResult =
        serde_json::from_str(&json).expect("failed result must round-trip without null numbers");
    assert!(!decoded.converged);
    assert_eq!(decoded.failure_reason, result.failure_reason);
}

fn solve(request: &SolveCohesiveInterfaceMesh2dRequest) -> SolveCohesiveInterfaceMesh2dResult {
    solve_cohesive_interface_mesh_2d(request).expect("valid model should return step diagnostics")
}

fn control(load_factor: f64) -> CohesiveInterfaceMesh2dControlStepInput {
    CohesiveInterfaceMesh2dControlStepInput {
        load_factor,
        prescribed_displacements: vec![[0.0; 2]; 4],
        prescribed_rotations: vec![],
    }
}

fn request(scale: f64) -> SolveCohesiveInterfaceMesh2dRequest {
    SolveCohesiveInterfaceMesh2dRequest {
        id: "plane-convergence".to_string(),
        nodes: (0..4)
            .map(|index| CohesiveInterfaceMesh2dNodeInput {
                id: format!("node-{index}"),
                x: (index % 2) as f64,
                y: 0.0,
                fixed: [true, index < 2],
                prescribed_displacement: None,
                load: [0.0, if index < 2 { 0.0 } else { 2.5 * scale }],
                fixed_rotation: false,
                prescribed_rotation: None,
                moment_z: 0.0,
            })
            .collect(),
        materials: vec![CohesiveInterfaceMesh2dMaterialInput {
            id: "adhesive".to_string(),
            properties: CohesiveInterface2dMaterialInput {
                normal_initial_stiffness: 1000.0 * scale,
                normal_compression_stiffness: 1200.0 * scale,
                normal_peak_traction: 10.0 * scale,
                normal_failure_separation: 0.03,
                shear_initial_stiffness: 800.0 * scale,
                shear_peak_traction: 8.0 * scale,
                shear_failure_separation: 0.05,
            },
        }],
        elements: vec![CohesiveInterfaceMesh2dElementInput {
            id: "interface".to_string(),
            lower_i: 0,
            lower_j: 1,
            upper_i: 2,
            upper_j: 3,
            thickness: 1.0,
            material_id: "adhesive".to_string(),
        }],
        connector_springs: vec![],
        host_trusses: vec![],
        host_plane_triangles: vec![],
        host_plane_quads: vec![],
        host_frames: vec![],
        load_steps: None,
        control_history: Some(vec![control(1.0)]),
        max_iterations: Some(12),
        tolerance: Some(1.0e-11),
    }
}
