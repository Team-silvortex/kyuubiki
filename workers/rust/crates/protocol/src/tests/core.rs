use super::prelude::*;

#[test]
fn applies_progress_to_job() {
    let mut job = Job::new("job-1", "project-1", "case-1");
    let mut event = ProgressEvent::new("job-1", JobStatus::Solving, 0.5);
    event.iteration = Some(12);
    event.residual = Some(1.0e-4);

    job.apply_progress(&event);

    assert_eq!(job.status, JobStatus::Solving);
    assert_eq!(job.progress, 0.5);
    assert_eq!(job.iteration, Some(12));
    assert_eq!(job.residual, Some(1.0e-4));
}

#[test]
fn exposes_lowercase_status_names() {
    assert_eq!(JobStatus::Solving.as_str(), "solving");
    assert_eq!(JobStatus::Completed.as_str(), "completed");
}

#[test]
fn serializes_rpc_round_trip() {
    let request = RpcRequest {
        rpc_version: RPC_VERSION,
        id: "rpc-1".to_string(),
        method: RpcMethod::SolveBar1d,
        params: serde_json::to_value(SolveBarRequest {
            length: 1.0,
            area: 0.01,
            youngs_modulus: 210.0e9,
            elements: 3,
            tip_force: 1000.0,
        })
        .expect("request params should serialize"),
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: RpcRequest = serde_json::from_str(&json).expect("request should decode");

    assert_eq!(decoded.method, RpcMethod::SolveBar1d);
    assert_eq!(decoded.rpc_version, RPC_VERSION);
    assert_eq!(decoded.id, "rpc-1");
    let params: SolveBarRequest = serde_json::from_value(decoded.params).expect("params");
    assert_eq!(params.elements, 3);
}

#[test]
fn serializes_operator_descriptor_round_trip() {
    let descriptor = OperatorDescriptor {
        id: "solve.frame_3d".to_string(),
        version: "1.0.0".to_string(),
        domain: "mechanical".to_string(),
        family: "frame_3d".to_string(),
        kind: OperatorKind::Solver,
        summary: "Solve a 3D frame model with six-DOF nodes.".to_string(),
        capability_tags: vec![
            "verified".to_string(),
            "mechanical".to_string(),
            "frame".to_string(),
        ],
        origin: OperatorOrigin::BuiltIn,
        input_schema: OperatorSchemaRef {
            schema: "kyuubiki.operator.solve.frame_3d.input".to_string(),
            version: "1".to_string(),
        },
        output_schema: OperatorSchemaRef {
            schema: "kyuubiki.operator.solve.frame_3d.output".to_string(),
            version: "1".to_string(),
        },
        inputs: vec![OperatorPortDescriptor {
            id: "model".to_string(),
            artifact_type: "model/frame_3d".to_string(),
            description: "Frame model payload".to_string(),
            dataset_value: Some("frame_model".to_string()),
            schema_ref: Some(OperatorSchemaRef {
                schema: "kyuubiki.operator.solve.frame_3d.input".to_string(),
                version: "1".to_string(),
            }),
        }],
        outputs: vec![OperatorPortDescriptor {
            id: "result".to_string(),
            artifact_type: "result/frame_3d".to_string(),
            description: "Frame solve result".to_string(),
            dataset_value: Some("frame_result".to_string()),
            schema_ref: Some(OperatorSchemaRef {
                schema: "kyuubiki.operator.solve.frame_3d.output".to_string(),
                version: "1".to_string(),
            }),
        }],
        validation: OperatorValidationProfile {
            baseline_status: OperatorValidationStatus::Verified,
            baseline_cases: vec!["frame_3d_baseline".to_string()],
            smoke_paths: vec!["workflow_graph".to_string(), "orchestrated_api".to_string()],
        },
    };

    let json = serde_json::to_string(&descriptor).expect("descriptor should serialize");
    let decoded: OperatorDescriptor =
        serde_json::from_str(&json).expect("descriptor should decode");

    assert_eq!(decoded.id, "solve.frame_3d");
    assert_eq!(decoded.kind, OperatorKind::Solver);
    assert_eq!(decoded.origin, OperatorOrigin::BuiltIn);
    assert_eq!(decoded.inputs.len(), 1);
    assert_eq!(
        decoded.validation.baseline_status,
        OperatorValidationStatus::Verified
    );
}

#[test]
fn serializes_operator_run_request_and_result_round_trip() {
    let request = OperatorRunRequest {
        operator_id: "extract.result_summary".to_string(),
        input: serde_json::json!({
            "job_id": "job-42",
            "result_kind": "frame_3d"
        }),
        context: OperatorRunContext {
            orchestrated: true,
            project_id: Some("project-1".to_string()),
            model_id: Some("model-7".to_string()),
            workflow_run_id: Some("run-9".to_string()),
        },
    };

    let json = serde_json::to_string(&request).expect("request should serialize");
    let decoded: OperatorRunRequest = serde_json::from_str(&json).expect("request should decode");
    assert_eq!(decoded.operator_id, "extract.result_summary");
    assert_eq!(decoded.context.project_id.as_deref(), Some("project-1"));

    let result = OperatorRunResult {
        operator_id: decoded.operator_id,
        summary: serde_json::json!({
            "max_stress": 1.26e5,
            "max_displacement": 5.3e-7
        }),
        artifacts: vec![OperatorArtifactRef {
            kind: "result_chunk".to_string(),
            id: "chunk-1".to_string(),
            label: "Primary summary".to_string(),
        }],
    };

    let json = serde_json::to_string(&result).expect("result should serialize");
    let decoded: OperatorRunResult = serde_json::from_str(&json).expect("result should decode");
    assert_eq!(decoded.artifacts.len(), 1);
    assert_eq!(decoded.artifacts[0].kind, "result_chunk");
}
