use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraphRunRequest, WorkflowNodeRunStatus};
use serde_json::{Value, json};

struct Case {
    cfd: bool,
    payload: Value,
    path: &'static str,
}

fn valid(cfd: bool) -> Case {
    let payload = if cfd {
        json!({
            "nodes":[{"vx":0.0,"vy":0.0,"p":0.0},{"vx":0.1,"vy":0.1,"p":0.1}],
            "elements":[{"div_u":0.001,"re":1.0,"dissipation":0.1}]
        })
    } else {
        json!({"frequencies":[{"max_displacement":0.001},{"max_displacement":0.002}]})
    };
    Case {
        cfd,
        payload,
        path: "",
    }
}

fn invalid_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for (collection, index, field, path) in [
        ("nodes", 1, "p", "payload.nodes[1].p"),
        (
            "nodes",
            1,
            "velocity_magnitude",
            "payload.nodes[1].velocity_magnitude",
        ),
        (
            "elements",
            0,
            "dissipation",
            "payload.elements[0].dissipation",
        ),
    ] {
        let mut case = valid(true);
        case.payload[collection][index][field] = Value::Null;
        case.path = path;
        cases.push(case);
    }
    let mut shape = valid(true);
    shape.payload["elements"][0] = Value::Null;
    shape.path = "payload.elements[0]";
    cases.push(shape);
    let mut span = valid(true);
    span.payload["nodes"][0]["p"] = json!(-1e308);
    span.payload["nodes"][1]["p"] = json!(1e308);
    span.path = "cfd_pressure_span";
    cases.push(span);
    let mut convergence = valid(true);
    convergence.payload["converged"] = json!(false);
    convergence.path = "payload.converged";
    cases.push(convergence);
    let mut dynamic = valid(false);
    dynamic.payload["frequencies"][1]["max_displacement"] = Value::Null;
    dynamic.payload["frequencies"][1]["displacement_amplitude"] = json!(0.001);
    dynamic.path = "payload.frequencies[1].max_displacement";
    cases.push(dynamic);
    let mut missing = valid(false);
    missing.payload["frequencies"][1] = json!({});
    missing.path = "payload.frequencies[1].max_displacement";
    cases.push(missing);
    cases
}

fn request(case: &Case, recover: bool) -> WorkflowGraphRunRequest {
    let artifact = "artifact/result_summary";
    let port = |id| json!({"id":id,"artifact_type":artifact});
    let edge = |from: &str, output: &str, to: &str, input: &str| {
        json!({
            "id":format!("{from}-{to}"),"artifact_type":artifact,
            "from":{"node":from,"port":output},"to":{"node":to,"port":input}
        })
    };
    let mut config = if case.cfd {
        json!({})
    } else {
        json!({"enabled_terms":["max_displacement"]})
    };
    if recover {
        config["on_error"] = json!("skip");
    }
    let (kind, operator, next, input) = if case.cfd {
        (
            "extract",
            "extract.stokes_flow_result_diagnostics",
            "transform.score_cfd_quality",
            "payload",
        )
    } else {
        (
            "transform",
            "transform.score_dynamic_quality",
            "transform.compose_quality_objective",
            "dynamic",
        )
    };
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"metric-integrity",
            "name":"Complete metric evidence","version":"1.0.0",
            "entry_nodes":["input","independent"],"output_nodes":["raw","decision","independent_output"],
            "nodes":[
                {"id":"input","kind":"input","inputs":[],"outputs":[port("payload")]},
                {"id":"assess","kind":kind,"operator_id":operator,"config":config,
                 "inputs":[port("payload")],"outputs":[port("summary")]},
                {"id":"next","kind":"transform","operator_id":next,
                 "inputs":[port(input)],"outputs":[port("summary")]},
                {"id":"decision","kind":"output","inputs":[port("summary")],"outputs":[]},
                {"id":"raw","kind":"output","inputs":[port("payload")],"outputs":[]},
                {"id":"independent","kind":"input","inputs":[],"outputs":[port("value")]},
                {"id":"independent_output","kind":"output","inputs":[port("value")],"outputs":[]}
            ],
            "edges":[edge("input","payload","assess","payload"),edge("input","payload","raw","payload"),
                     edge("assess","summary","next",input),edge("next","summary","decision","summary"),
                     edge("independent","value","independent_output","value")]
        },
        "input_artifacts":{"input":case.payload,"independent":{"value":7}}
    })).unwrap()
}

#[test]
fn corrupt_samples_stop_before_a_partial_summary_or_objective_is_published() {
    for case in invalid_cases() {
        let error = run_workflow_graph(request(&case, false)).unwrap_err();
        assert!(error.contains("workflow node assess failed"), "{error}");
        assert!(error.contains(case.path), "{error}");
        assert!(!error.contains("panicked"), "{error}");
    }
}

#[test]
fn metric_failure_recovery_keeps_raw_inputs_and_independent_work() {
    for case in invalid_cases() {
        let run = run_workflow_graph(request(&case, true)).unwrap();
        assert_eq!(run.failed_nodes, vec!["assess"]);
        assert_eq!(run.skipped_nodes.len(), 2);
        for node in ["next", "decision"] {
            assert!(run.skipped_nodes.iter().any(|id| id == node));
        }
        for artifact in ["assess.summary", "next.summary", "decision.summary"] {
            assert!(!run.artifacts.contains_key(artifact));
        }
        assert_eq!(run.artifacts["raw.payload"], case.payload);
        assert_eq!(
            run.artifacts["independent_output.value"],
            json!({"value":7})
        );
        let trace = run
            .node_runs
            .iter()
            .find(|trace| trace.node_id == "assess")
            .unwrap();
        assert_eq!(trace.status, WorkflowNodeRunStatus::Failed);
        assert!(trace.error_message.as_deref().unwrap().contains(case.path));
    }
}

#[test]
fn corrected_samples_replay_through_cfd_and_dynamic_quality_chains() {
    for case in invalid_cases() {
        assert_eq!(
            run_workflow_graph(request(&case, true))
                .unwrap()
                .failed_nodes
                .len(),
            1
        );
        let run = run_workflow_graph(request(&valid(case.cfd), false)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert!(run.skipped_nodes.is_empty());
        let summary = &run.artifacts["decision.summary"];
        let prefix = if case.cfd { "cfd" } else { "composite" };
        assert_eq!(summary[format!("{prefix}_quality_ready")], true);
        assert_eq!(summary[format!("{prefix}_quality_missing_metric_count")], 0);
        assert!(
            summary[format!("{prefix}_quality_score")]
                .as_f64()
                .unwrap()
                .is_finite()
        );
    }
}
