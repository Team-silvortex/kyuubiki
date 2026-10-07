use crate::{AgentDescriptor, CancelExecutionRequest, RpcMethod, RpcRequest};
use serde_json::{Value, json};

#[test]
fn exact_cancel_contract_round_trips_and_is_advertised_separately_from_job_cancel() {
    let target = CancelExecutionRequest {
        process_instance_id: "process-one".into(),
        request_id: "execution-one".into(),
        generation: 1,
        job_id: "job-one".into(),
    };
    target.validate().unwrap();
    let request = RpcRequest {
        rpc_version: 1,
        id: "control-one".into(),
        method: RpcMethod::CancelExecution,
        params: serde_json::to_value(&target).unwrap(),
    };
    let bytes = serde_json::to_vec(&request).unwrap();
    assert_eq!(
        serde_json::from_slice::<RpcRequest>(&bytes).unwrap(),
        request
    );
    assert_eq!(
        serde_json::from_value::<CancelExecutionRequest>(request.params).unwrap(),
        target
    );
    let descriptor = AgentDescriptor::solver_agent_default();
    for method in [RpcMethod::CancelExecution, RpcMethod::CancelJob] {
        assert!(descriptor.protocol.methods.contains(&method));
        assert!(
            descriptor
                .capabilities
                .iter()
                .find(|entry| entry.id == "control")
                .unwrap()
                .methods
                .contains(&method)
        );
    }
}

#[test]
fn exact_cancel_contract_rejects_incomplete_targets_aliases_and_invalid_generations() {
    let exact = json!({"process_instance_id":"process", "request_id":"request", "generation":1, "job_id":"job"});
    for field in ["process_instance_id", "request_id", "generation", "job_id"] {
        let mut missing = exact.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<CancelExecutionRequest>(missing).is_err());
    }
    let mut alias = exact.clone();
    alias["expected_generation"] = json!(1);
    assert!(serde_json::from_value::<CancelExecutionRequest>(alias).is_err());
    for invalid in [Value::Null, json!(0), json!(-1), json!(1.5), json!("1")] {
        let mut value = exact.clone();
        value["generation"] = invalid;
        assert!(
            !serde_json::from_value::<CancelExecutionRequest>(value)
                .is_ok_and(|target| target.validate().is_ok())
        );
    }
}

#[test]
fn exact_cancel_contract_bounds_identity_bytes_without_normalizing_or_truncating_them() {
    let exact = json!({"process_instance_id":"process", "request_id":"request", "generation":1, "job_id":"job"});
    for field in ["process_instance_id", "request_id", "job_id"] {
        for invalid in [
            "",
            " ",
            "\u{00a0}\u{2003}\u{3000}",
            "x\ny",
            &"x".repeat(257),
            &"\u{754c}".repeat(100),
        ] {
            let mut value = exact.clone();
            value[field] = json!(invalid);
            let target: CancelExecutionRequest = serde_json::from_value(value).unwrap();
            assert!(target.validate().is_err(), "accepted {field}");
        }
        let mut value = exact.clone();
        value[field] = json!("x".repeat(256));
        let target: CancelExecutionRequest = serde_json::from_value(value.clone()).unwrap();
        target.validate().unwrap();
        assert_eq!(serde_json::to_value(target).unwrap(), value);
        for valid in ["\u{feff}", " spaced-identity "] {
            let mut value = exact.clone();
            value[field] = json!(valid);
            let target: CancelExecutionRequest = serde_json::from_value(value.clone()).unwrap();
            target.validate().unwrap();
            assert_eq!(serde_json::to_value(target).unwrap(), value);
        }
    }
}
