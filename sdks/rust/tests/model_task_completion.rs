use kyuubiki_headless_sdk::{
    ApprovedModelPlanStep, KyuubikiSession, MODEL_COLLABORATION_SCHEMA_VERSION,
    MODEL_PLAN_APPROVAL_SCHEMA_VERSION, MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION,
    ModelActionDispatch, ModelActionDispatcher, ModelApprovalVerifier, ModelCollaborationPolicy,
    ModelCollaborationSession, ModelHeadlessPlan, ModelPlanApproval, ModelReceiptVerifier,
    ModelResearchExecutionReceipt, ModelResearchExecutionStatus, ModelResearchFrontierStage,
    ModelToolCall, ModelWorkflowProposal, SdkResult, SessionModelActionDispatcher,
    build_model_headless_plan, compute_model_headless_plan_digest, execute_model_headless_plan,
    operator_task_recovery_summary, start_model_research_frontier,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

struct Dispatcher {
    seen: Mutex<Vec<String>>,
    receipt: Value,
}

impl ModelActionDispatcher for Dispatcher {
    fn dispatch_model_action(&self, action: &str, _: &Value) -> SdkResult<ModelActionDispatch> {
        self.seen.lock().unwrap().push(action.into());
        Ok(ModelActionDispatch {
            authority: "test-control-plane".into(),
            output: if action.ends_with("execute") {
                self.receipt.clone()
            } else {
                json!({"ok": true})
            },
        })
    }
}

struct Verifier;

impl ModelApprovalVerifier for Verifier {
    fn verify_model_approval(&self, _: &ModelHeadlessPlan, _: &ModelPlanApproval) -> SdkResult<()> {
        Ok(())
    }
}

impl ModelReceiptVerifier for Verifier {
    fn verify_model_receipt(&self, _: &ModelResearchExecutionReceipt) -> SdkResult<()> {
        Ok(())
    }
}

fn task(id: &str) -> Value {
    // Transport-only fixtures bind receipts; they do not qualify TaskIR admission or a solver.
    json!({
        "schema_version": "kyuubiki.operator-task-ir/v1",
        "task_id": id,
        "integrity": {"task_digest": "a".repeat(64)},
        "operator": {"id": "fem.bar.axial_1d"},
        "execution_program": {"program_id": "fem.bar.axial_1d"}
    })
}

fn executed(task: &Value) -> Value {
    json!({
        "status": "executed",
        "task_id": task["task_id"],
        "task_digest": task["integrity"]["task_digest"],
        "operator_id": task["operator"]["id"],
        "program_id": task["execution_program"]["program_id"],
        "result": {"displacement": [0.0, -0.0, 1.0e-8]},
        "execution_readiness": {"status": "executed", "ready_to_dispatch": true}
    })
}

fn batch() -> Value {
    json!({
        "quality_execution_batch_contract": "kyuubiki.quality_execution_batch/v1",
        "task_count": 2,
        "tasks": [
            {"case_id": "case-a", "task_ir": task("task-a")},
            {"case_id": "case-b", "task_ir": task("task-b")}
        ]
    })
}

fn completed_batch(batch: &Value) -> Value {
    let digest_fields = [
        "quality_execution_batch_contract",
        "operator_id",
        "task_count",
        "case_index",
        "tasks",
    ];
    let fields: serde_json::Map<_, _> = digest_fields
        .into_iter()
        .filter_map(|key| batch.get(key).map(|value| (key.into(), value.clone())))
        .collect();
    let digest = Sha256::digest(serde_json::to_vec(&fields).unwrap());
    json!({
        "status": "executed",
        "operator_task_batch_execution_contract": "kyuubiki.operator_task_batch_execution/v1",
        "batch_digest": format!("{digest:x}"),
        "task_count": 2, "executed_count": 2, "ok_count": 2, "error_count": 0,
        "results": batch["tasks"].as_array().unwrap().iter().map(|entry| {
            let mut result = executed(&entry["task_ir"]);
            result["case_id"] = entry["case_id"].clone();
            result["status"] = json!("ok");
            result.as_object_mut().unwrap().remove("program_id");
            result
        }).collect::<Vec<_>>()
    })
}

fn plan(action: &str, payload: Value) -> (ModelHeadlessPlan, ModelPlanApproval) {
    let session = ModelCollaborationSession {
        schema_version: MODEL_COLLABORATION_SCHEMA_VERSION.into(),
        session_id: "completion-session".into(),
        workflow_id: "completion-workflow".into(),
        objective: "Do not advance before a task completed.".into(),
        language: "en".into(),
        created_at: "2026-10-05T00:00:00Z".into(),
        policy: ModelCollaborationPolicy {
            allowed_actions: vec![
                "service_health".into(),
                action.into(),
                "operator_catalog_list".into(),
            ],
            allow_sensitive: true,
            ..ModelCollaborationPolicy::default()
        },
    };
    let proposal = ModelWorkflowProposal {
        schema_version: MODEL_WORKFLOW_PROPOSAL_SCHEMA_VERSION.into(),
        session_id: session.session_id.clone(),
        summary: "Bounded task execution and downstream observation".into(),
        calls: [
            ("service_health", json!({})),
            (action, payload),
            ("operator_catalog_list", json!({})),
        ]
        .into_iter()
        .map(|(action, payload)| ModelToolCall {
            id: None,
            action: action.into(),
            payload,
            reason: None,
        })
        .collect(),
    };
    let plan = build_model_headless_plan(&session, &proposal).unwrap();
    assert!(plan.ok, "{:?}", plan.issues);
    let approval = ModelPlanApproval {
        schema_version: MODEL_PLAN_APPROVAL_SCHEMA_VERSION.into(),
        approval_id: "completion-approval".into(),
        session_id: plan.session_id.clone(),
        workflow_id: plan.workflow_id.clone(),
        plan_digest: compute_model_headless_plan_digest(&plan).unwrap(),
        authority: "test-caller".into(),
        issued_at: "2026-10-05T00:00:00Z".into(),
        approved_steps: plan
            .steps
            .iter()
            .filter(|step| step.requires_confirmation)
            .map(|step| ApprovedModelPlanStep {
                index: step.index,
                action: step.action.clone(),
            })
            .collect(),
    };
    (plan, approval)
}

fn run(action: &str, payload: Value, output: Value) -> ModelResearchExecutionReceipt {
    let (plan, approval) = plan(action, payload);
    let dispatcher = Dispatcher {
        seen: Mutex::new(vec![]),
        receipt: output.clone(),
    };
    let receipt =
        execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &Verifier).unwrap();
    if receipt.status == ModelResearchExecutionStatus::Failed {
        assert_eq!(receipt.completed_steps, 1);
        assert_eq!(receipt.failed_step, Some(2));
        assert_eq!(receipt.records.len(), 2);
        assert_eq!(*dispatcher.seen.lock().unwrap(), ["service_health", action]);
        assert_eq!(
            receipt.records[1].authority.as_deref(),
            Some("test-control-plane")
        );
        assert_eq!(receipt.records[1].output, Some(output));
        let serialized = serde_json::to_value(&receipt).unwrap();
        let restored: ModelResearchExecutionReceipt = serde_json::from_value(serialized).unwrap();
        let frontier = start_model_research_frontier(&restored, &Verifier).unwrap();
        assert_eq!(frontier.stage, ModelResearchFrontierStage::Blocked);
        assert_eq!(frontier.next_action, None);
    } else {
        assert_eq!(receipt.completed_steps, 3);
        assert_eq!(dispatcher.seen.lock().unwrap().len(), 3);
    }
    receipt
}

#[test]
fn pending_or_failed_task_receipts_stop_model_research_before_downstream_actions() {
    let task = task("task-a");
    for status in [
        "blocked",
        "verified_pending_execution",
        "verified_pending_engine_execution",
        "ready_for_package_resolution",
        "failed",
        "error",
        "cancelled",
        "unknown",
        "",
        "EXECUTED",
    ] {
        let mut output = executed(&task);
        output["status"] = json!(status);
        output
            .as_object_mut()
            .unwrap()
            .remove("execution_readiness");
        let receipt = run("operator_task_execute", json!({"task": task}), output);
        assert_eq!(
            receipt.status,
            ModelResearchExecutionStatus::Failed,
            "status {status}"
        );
    }
}

#[test]
fn nested_agent_pending_and_contradictory_readiness_are_not_model_completion() {
    let task = task("task-a");
    let mut output = executed(&task);
    let mut agent = executed(&task);
    agent.as_object_mut().unwrap().remove("status");
    agent["operator_task_ir_status"] = json!("verified_pending_engine_execution");
    agent["execution_readiness"] = json!({
        "status": "blocked", "ready_to_dispatch": false,
        "required_action": "attach_operator_package_runtime"
    });
    output["result"] = agent;
    let receipt = run("operator_task_execute", json!({"task": task}), output);
    assert_eq!(receipt.status, ModelResearchExecutionStatus::Failed);
    assert!(
        receipt.records[1]
            .error
            .as_ref()
            .unwrap()
            .contains("operator_task_execution_blocked")
    );
    assert_eq!(
        operator_task_recovery_summary(&serde_json::to_value(&receipt).unwrap())["recovery_actions"],
        json!(["attach_operator_package_runtime"])
    );

    let mut output = executed(&task);
    output["execution_readiness"]["ready_to_dispatch"] = json!(false);
    assert_eq!(
        run("operator_task_execute", json!({"task": task}), output).status,
        ModelResearchExecutionStatus::Failed
    );
    for field in ["blocking_stage", "blocking_reason", "required_action"] {
        let mut output = executed(&task);
        output["execution_readiness"][field] = json!("do-not-echo");
        let receipt = run("operator_task_execute", json!({"task": task}), output);
        assert_eq!(
            receipt.status,
            ModelResearchExecutionStatus::Failed,
            "{field}"
        );
        assert!(
            !receipt.records[1]
                .error
                .as_ref()
                .unwrap()
                .contains("do-not-echo")
        );
    }
}

#[test]
fn stale_missing_and_malformed_task_receipts_cannot_claim_model_completion() {
    let task = task("task-a");
    for path in [
        "/task_id",
        "/task_digest",
        "/operator_id",
        "/program_id",
        "/result",
    ] {
        for rejected in [json!(null), json!(false), json!("stale-secret-value")] {
            if path == "/result" && !rejected.is_null() {
                continue;
            }
            let mut output = executed(&task);
            *output.pointer_mut(path).unwrap() = rejected;
            let receipt = run("operator_task_execute", json!({"task": task}), output);
            assert_eq!(
                receipt.status,
                ModelResearchExecutionStatus::Failed,
                "{path}"
            );
            assert!(
                !receipt.records[1]
                    .error
                    .as_ref()
                    .unwrap()
                    .contains("stale-secret-value")
            );
        }
        let mut output = executed(&task);
        output.as_object_mut().unwrap().remove(&path[1..]);
        assert_eq!(
            run("operator_task_execute", json!({"task": task}), output).status,
            ModelResearchExecutionStatus::Failed,
            "missing {path}"
        );
    }
    for (field, value) in [
        ("ok", json!(false)),
        ("error", json!({"secret": "do-not-echo"})),
        ("blocked_stage", json!("package_fetch")),
        ("validation_receipt", json!([])),
        (
            "provenance_receipt",
            json!({"task_id": "stale-secret-value"}),
        ),
        ("validation_receipt", json!({"digest_verified": false})),
        ("execution_readiness", json!(null)),
    ] {
        let mut output = executed(&task);
        output[field] = value;
        let receipt = run("operator_task_execute", json!({"task": task}), output);
        assert_eq!(
            receipt.status,
            ModelResearchExecutionStatus::Failed,
            "{field}"
        );
        assert!(
            !receipt.records[1]
                .error
                .as_ref()
                .unwrap()
                .contains("do-not-echo")
        );
    }
}

#[test]
fn nested_agent_receipt_is_independently_bound_and_validated() {
    let task = task("task-a");
    let mut agent = executed(&task);
    agent.as_object_mut().unwrap().remove("status");
    agent["operator_task_ir_status"] = json!("executed");
    for (field, value) in [
        ("task_digest", json!("stale")),
        ("program_id", json!("wrong-program")),
        ("operator_task_ir_status", json!("unknown")),
        ("validation_receipt", json!({"digest_verified": false})),
    ] {
        let mut output = executed(&task);
        output["result"] = agent.clone();
        output["result"][field] = value;
        assert_eq!(
            run("operator_task_execute", json!({"task": task}), output).status,
            ModelResearchExecutionStatus::Failed,
            "nested {field}"
        );
    }
    let mut output = executed(&task);
    output["result"] = agent;
    assert_eq!(
        run("operator_task_execute", json!({"task": task}), output).status,
        ModelResearchExecutionStatus::Completed
    );
}

#[test]
fn partial_or_skipped_task_batches_never_complete_model_research() {
    let batch = batch();
    for (field, value) in [
        ("ok_count", json!(1)),
        ("error_count", json!(1)),
        ("executed_count", json!(1)),
        ("task_count", json!(1)),
        ("ok_count", json!(u64::MAX)),
        ("error_count", json!(-1)),
        ("task_count", json!("2")),
        ("batch_digest", json!("stale")),
        ("operator_task_batch_execution_contract", json!("unknown")),
    ] {
        let mut output = completed_batch(&batch);
        output[field] = value;
        assert_eq!(
            run(
                "operator_task_batch_execute",
                json!({"batch": batch}),
                output
            )
            .status,
            ModelResearchExecutionStatus::Failed,
            "{field}"
        );
    }
    let mut output = completed_batch(&batch);
    output["results"].as_array_mut().unwrap().pop();
    assert_eq!(
        run(
            "operator_task_batch_execute",
            json!({"batch": batch}),
            output
        )
        .status,
        ModelResearchExecutionStatus::Failed
    );
}

#[test]
fn batch_cases_and_nested_agent_receipts_must_match_the_submitted_tasks() {
    let batch = batch();
    let original = completed_batch(&batch);
    for (field, value) in [
        ("case_id", json!("unknown")),
        ("task_id", json!("task-a")),
        ("task_digest", json!("stale")),
        ("operator_id", json!("wrong")),
        ("status", json!("error")),
        ("result", json!(null)),
        (
            "execution_readiness",
            json!({"status": "blocked", "ready_to_dispatch": false}),
        ),
    ] {
        let mut output = original.clone();
        output["results"][1][field] = value;
        assert_eq!(
            run(
                "operator_task_batch_execute",
                json!({"batch": batch}),
                output
            )
            .status,
            ModelResearchExecutionStatus::Failed,
            "entry {field}"
        );
    }
    let mut output = original.clone();
    output["results"][1] = output["results"][0].clone();
    assert_eq!(
        run(
            "operator_task_batch_execute",
            json!({"batch": batch}),
            output
        )
        .status,
        ModelResearchExecutionStatus::Failed
    );

    let mut output = original;
    let mut agent = executed(&batch["tasks"][1]["task_ir"]);
    agent["operator_task_ir_status"] = json!("verified_pending_execution");
    agent.as_object_mut().unwrap().remove("execution_readiness");
    output["results"][1]["result"] = agent;
    assert_eq!(
        run(
            "operator_task_batch_execute",
            json!({"batch": batch}),
            output
        )
        .status,
        ModelResearchExecutionStatus::Failed
    );
}

#[test]
fn valid_single_and_reordered_batch_results_remain_available_without_float_loss() {
    let task = task("task-a");
    let output = executed(&task);
    let receipt = run(
        "operator_task_execute",
        json!({"task": task}),
        output.clone(),
    );
    assert_eq!(receipt.status, ModelResearchExecutionStatus::Completed);
    let result = receipt.records[1].output.as_ref().unwrap();
    assert_eq!(result, &output);
    assert_eq!(
        result["result"]["displacement"][1]
            .as_f64()
            .unwrap()
            .to_bits(),
        (-0.0_f64).to_bits()
    );
    let batch = batch();
    let mut output = completed_batch(&batch);
    output["results"].as_array_mut().unwrap().reverse();
    let receipt = run(
        "operator_task_batch_execute",
        json!({"batch": batch}),
        output.clone(),
    );
    assert_eq!(receipt.status, ModelResearchExecutionStatus::Completed);
    assert_eq!(receipt.records[1].output, Some(output));
}

#[test]
fn a_repaired_task_can_run_a_fresh_exactly_approved_model_plan() {
    let task = task("task-a");
    let mut blocked = executed(&task);
    blocked["execution_readiness"] = json!({"status": "blocked", "ready_to_dispatch": false});
    assert_eq!(
        run("operator_task_execute", json!({"task": task}), blocked).status,
        ModelResearchExecutionStatus::Failed
    );
    assert_eq!(
        run(
            "operator_task_execute",
            json!({"task": task}),
            executed(&task)
        )
        .status,
        ModelResearchExecutionStatus::Completed
    );
}

#[test]
fn preparation_does_not_require_a_computation_result() {
    let task = task("task-a");
    let (plan, approval) = plan("operator_task_prepare", json!({"task": task}));
    let dispatcher = Dispatcher {
        seen: Mutex::new(vec![]),
        receipt: json!({"status": "verified"}),
    };
    let receipt =
        execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &Verifier).unwrap();
    assert_eq!(receipt.status, ModelResearchExecutionStatus::Completed);
}

#[test]
fn contradictory_batch_success_flags_and_failure_lists_are_rejected() {
    let batch = batch();
    for (field, value) in [
        ("ok", json!(false)),
        ("error", json!("do-not-echo")),
        ("blocked_stage", json!("package_fetch")),
        ("failure_receipts", json!([{}])),
        ("error_codes", json!(["failure"])),
        ("failed_case_ids", json!(["case-a"])),
        ("error_code_counts", json!({"failure": 1})),
        (
            "execution_readiness",
            json!({"status": "blocked", "ready_to_dispatch": false}),
        ),
    ] {
        let mut output = completed_batch(&batch);
        output[field] = value;
        assert_eq!(
            run(
                "operator_task_batch_execute",
                json!({"batch": batch}),
                output
            )
            .status,
            ModelResearchExecutionStatus::Failed,
            "{field}"
        );
    }
    for field in [
        "batch_digest",
        "task_count",
        "executed_count",
        "ok_count",
        "error_count",
        "results",
    ] {
        let mut output = completed_batch(&batch);
        output.as_object_mut().unwrap().remove(field);
        assert_eq!(
            run(
                "operator_task_batch_execute",
                json!({"batch": batch}),
                output
            )
            .status,
            ModelResearchExecutionStatus::Failed,
            "missing {field}"
        );
    }
}

#[test]
fn session_http_task_gate_retains_blockers_and_recovers_with_fresh_execution() {
    let task = task("task-a");
    let healthy = executed(&task);
    let mut blocked = healthy.clone();
    blocked["execution_readiness"] = json!({
        "status": "blocked", "ready_to_dispatch": false,
        "required_action": "attach_operator_package_runtime"
    });
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let responses = [
        json!({"status": "ok"}),
        blocked.clone(),
        json!({"status": "ok"}),
        healthy.clone(),
        json!({"operators": []}),
    ];
    let submitted_task = task.clone();
    let server = thread::spawn(move || {
        let mut paths = vec![];
        for body in responses {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(std::time::Instant::now() < deadline, "request deadline");
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("accept: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let (path, payload) = read_request(&mut stream);
            if path == "/api/v1/operator-tasks/execute" {
                assert_eq!(payload["task"], submitted_task);
            }
            paths.push(path);
            let body = serde_json::to_vec(&body).unwrap();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
            for chunk in body.chunks(19) {
                stream.write_all(chunk).unwrap();
            }
        }
        paths
    });
    let session = KyuubikiSession::from_control_plane(&format!("http://{address}"), None).unwrap();
    let dispatcher = SessionModelActionDispatcher::new(&session);
    let (plan, approval) = plan("operator_task_execute", json!({"task": task}));
    let failed =
        execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &Verifier).unwrap();
    assert_eq!(failed.status, ModelResearchExecutionStatus::Failed);
    assert_eq!(failed.completed_steps, 1);
    assert_eq!(failed.records[1].output, Some(blocked));
    assert_eq!(
        failed.records[1].authority.as_deref(),
        Some("control_plane")
    );
    assert_eq!(
        operator_task_recovery_summary(&serde_json::to_value(failed).unwrap())["recovery_actions"],
        json!(["attach_operator_package_runtime"])
    );
    let recovered =
        execute_model_headless_plan(&dispatcher, &plan, Some(&approval), &Verifier).unwrap();
    assert_eq!(recovered.status, ModelResearchExecutionStatus::Completed);
    assert_eq!(recovered.completed_steps, 3);
    assert_eq!(recovered.records[1].output, Some(healthy));
    assert_eq!(
        server.join().unwrap(),
        [
            "/api/health",
            "/api/v1/operator-tasks/execute",
            "/api/health",
            "/api/v1/operator-tasks/execute",
            "/api/v1/operators",
        ]
    );
}

fn read_request(stream: &mut TcpStream) -> (String, Value) {
    let mut request = vec![];
    let (header_end, body_len) = loop {
        let mut buffer = [0; 512];
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0, "incomplete headers");
        request.extend_from_slice(&buffer[..count]);
        if let Some(index) = request.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&request[..index]).unwrap();
            let length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                .unwrap_or(0);
            break (index + 4, length);
        }
    };
    while request.len() < header_end + body_len {
        let mut buffer = [0; 512];
        let count = stream.read(&mut buffer).unwrap();
        assert!(count > 0, "incomplete body");
        request.extend_from_slice(&buffer[..count]);
    }
    let headers = std::str::from_utf8(&request[..header_end]).unwrap();
    let path = headers
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .into();
    let payload = if body_len == 0 {
        Value::Null
    } else {
        serde_json::from_slice(&request[header_end..header_end + body_len]).unwrap()
    };
    (path, payload)
}
