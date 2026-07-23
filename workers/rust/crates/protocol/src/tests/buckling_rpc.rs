use crate::{
    BucklingBeam1dElementInput, BucklingBeam1dModeResult, BucklingBeam1dNodeInput,
    BucklingModeDirectionAssessment, Frame2dElementInput, Frame2dNodeInput,
    Frame2dPDeltaStepResult, Frame2dStabilityKinematics, Frame2dStabilityPathControl, RPC_VERSION,
    RpcMethod, RpcRequest, SolveBucklingBeam1dRequest, SolveBucklingFrame2dRequest,
    SolveFrame2dPDeltaRequest, SolveFrame2dRequest,
};

#[test]
fn buckling_beam_rpc_round_trip_preserves_reference_load_pattern() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "buckling-column".to_string(),
        method: RpcMethod::SolveBucklingBeam1d,
        params: serde_json::to_value(SolveBucklingBeam1dRequest {
            nodes: vec![node("a", 0.0, true), node("b", 2.0, true)],
            elements: vec![BucklingBeam1dElementInput {
                id: "column".to_string(),
                node_i: 0,
                node_j: 1,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                reference_compressive_force: 100_000.0,
            }],
            mode_count: Some(1),
        })
        .expect("buckling request should serialize"),
    };
    let encoded = serde_json::to_string(&request).expect("rpc should serialize");
    let decoded: RpcRequest = serde_json::from_str(&encoded).expect("rpc should decode");
    let params: SolveBucklingBeam1dRequest =
        serde_json::from_value(decoded.params).expect("buckling params should decode");

    assert_eq!(decoded.method, RpcMethod::SolveBucklingBeam1d);
    assert_eq!(params.elements[0].reference_compressive_force, 100_000.0);
    assert_eq!(params.mode_count, Some(1));
}

#[test]
fn p_delta_rpc_round_trip_preserves_imperfection_controls() {
    let buckling = SolveBucklingFrame2dRequest {
        frame: SolveFrame2dRequest {
            nodes: vec![frame_node("base", 0.0, true), frame_node("top", 2.0, false)],
            elements: vec![Frame2dElementInput {
                id: "column".to_string(),
                node_i: 0,
                node_j: 1,
                area: 0.01,
                youngs_modulus: 210.0e9,
                moment_of_inertia: 8.0e-6,
                section_modulus: 1.0e-4,
            }],
        },
        mode_count: Some(1),
    };
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "p-delta-column".to_string(),
        method: RpcMethod::SolveFrame2dPDelta,
        params: serde_json::to_value(SolveFrame2dPDeltaRequest {
            buckling,
            imperfection_amplitude: 0.002,
            kinematics: Frame2dStabilityKinematics::Corotational,
            path_control: Frame2dStabilityPathControl::ArcLength,
            imperfection_shape: Some(vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
            imperfection_mode_index: None,
            maximum_load_factor: Some(2.0),
            load_steps: Some(8),
            max_iterations: Some(24),
            tolerance: Some(1.0e-9),
            max_step_cutbacks: Some(6),
            arc_length_radius: Some(0.01),
            arc_length_load_scale: Some(0.25),
            arc_length_target_iterations: Some(7),
        })
        .expect("p-delta request should serialize"),
    };
    let decoded: RpcRequest = serde_json::from_str(
        &serde_json::to_string(&request).expect("p-delta rpc should serialize"),
    )
    .expect("p-delta rpc should decode");
    let params: SolveFrame2dPDeltaRequest =
        serde_json::from_value(decoded.params.clone()).expect("p-delta params should decode");

    assert_eq!(decoded.method, RpcMethod::SolveFrame2dPDelta);
    assert_eq!(params.imperfection_amplitude, 0.002);
    assert_eq!(params.imperfection_shape.as_ref().unwrap()[3], 1.0);
    assert_eq!(params.load_steps, Some(8));
    assert_eq!(params.max_iterations, Some(24));
    assert_eq!(params.tolerance, Some(1.0e-9));
    assert_eq!(params.max_step_cutbacks, Some(6));
    assert_eq!(params.arc_length_radius, Some(0.01));
    assert_eq!(params.arc_length_load_scale, Some(0.25));
    assert_eq!(params.arc_length_target_iterations, Some(7));
    assert_eq!(params.kinematics, Frame2dStabilityKinematics::Corotational);
    assert_eq!(params.path_control, Frame2dStabilityPathControl::ArcLength);

    let mut legacy = decoded.params;
    let legacy_object = legacy.as_object_mut().unwrap();
    legacy_object.remove("kinematics");
    legacy_object.remove("max_iterations");
    legacy_object.remove("tolerance");
    legacy_object.remove("max_step_cutbacks");
    legacy_object.remove("path_control");
    legacy_object.remove("arc_length_radius");
    legacy_object.remove("arc_length_load_scale");
    legacy_object.remove("arc_length_target_iterations");
    let legacy: SolveFrame2dPDeltaRequest =
        serde_json::from_value(legacy).expect("legacy p-delta params should decode");
    assert_eq!(
        legacy.kinematics,
        Frame2dStabilityKinematics::LinearizedPDelta
    );
    assert_eq!(legacy.max_iterations, None);
    assert_eq!(legacy.tolerance, None);
    assert_eq!(legacy.max_step_cutbacks, None);
    assert_eq!(
        legacy.path_control,
        Frame2dStabilityPathControl::LoadControl
    );
    assert_eq!(legacy.arc_length_radius, None);
    assert_eq!(legacy.arc_length_load_scale, None);
    assert_eq!(legacy.arc_length_target_iterations, None);
}

#[test]
fn legacy_buckling_mode_results_default_direction_diagnostics() {
    let mode: BucklingBeam1dModeResult = serde_json::from_value(serde_json::json!({
        "index": 0,
        "load_factor": 2.5,
        "residual_norm": 1.0e-9,
        "shape": [0.0, 1.0]
    }))
    .expect("legacy buckling result should remain readable");

    assert_eq!(mode.relative_gap_to_next, None);
    assert_eq!(
        mode.direction_assessment,
        BucklingModeDirectionAssessment::Unassessed
    );
}

#[test]
fn legacy_p_delta_steps_default_adaptive_failure_diagnostics() {
    let step: Frame2dPDeltaStepResult = serde_json::from_value(serde_json::json!({
        "step": 1,
        "load_factor": 0.5,
        "critical_factor_ratio": 0.25,
        "residual_norm": 1.0e-9,
        "imperfection_amplification": 1.2,
        "max_incremental_displacement": 0.001,
        "displacements": [0.0, 0.001, 0.0]
    }))
    .expect("legacy p-delta step should remain readable");

    assert!(step.converged);
    assert_eq!(step.substeps, 1);
    assert_eq!(step.cutbacks, 0);
    assert_eq!(step.achieved_load_factor, None);
    assert_eq!(step.failure_reason, None);
    assert_eq!(step.failure_detail, None);
    assert_eq!(step.arc_length_constraint_error, None);
    assert_eq!(step.arc_length_radius, None);
}

#[test]
fn buckling_frame_rpc_round_trip_preserves_static_preload_model() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "buckling-frame".to_string(),
        method: RpcMethod::SolveBucklingFrame2d,
        params: serde_json::to_value(SolveBucklingFrame2dRequest {
            frame: SolveFrame2dRequest {
                nodes: vec![frame_node("base", 0.0, true), frame_node("top", 2.0, false)],
                elements: vec![Frame2dElementInput {
                    id: "column".to_string(),
                    node_i: 0,
                    node_j: 1,
                    area: 0.01,
                    youngs_modulus: 210.0e9,
                    moment_of_inertia: 8.0e-6,
                    section_modulus: 1.0e-4,
                }],
            },
            mode_count: Some(2),
        })
        .expect("buckling frame request should serialize"),
    };
    let encoded = serde_json::to_string(&request).expect("rpc should serialize");
    let decoded: RpcRequest = serde_json::from_str(&encoded).expect("rpc should decode");
    let params: SolveBucklingFrame2dRequest =
        serde_json::from_value(decoded.params).expect("buckling frame params should decode");

    assert_eq!(decoded.method, RpcMethod::SolveBucklingFrame2d);
    assert_eq!(params.frame.nodes[1].load_y, -100_000.0);
    assert_eq!(params.mode_count, Some(2));
}

fn node(id: &str, x: f64, fix_y: bool) -> BucklingBeam1dNodeInput {
    BucklingBeam1dNodeInput {
        id: id.to_string(),
        x,
        fix_y,
        fix_rz: false,
    }
}

fn frame_node(id: &str, y: f64, base: bool) -> Frame2dNodeInput {
    Frame2dNodeInput {
        id: id.to_string(),
        x: 0.0,
        y,
        fix_x: true,
        fix_y: base,
        fix_rz: false,
        load_x: 0.0,
        load_y: if base { 0.0 } else { -100_000.0 },
        moment_z: 0.0,
    }
}
