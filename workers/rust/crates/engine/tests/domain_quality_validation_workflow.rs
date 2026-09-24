use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraphRunRequest, WorkflowNodeRunStatus};
use serde_json::{Value, json};

struct Case {
    payload: Value,
    config: Value,
    error_path: &'static str,
}

fn valid() -> Case {
    Case {
        payload: json!({"max_stress":1.0,"mass":1.0}),
        config: json!({
            "enabled_terms":["max_stress","mass"],
            "targets":{"max_stress":2.0,"mass":2.0},
            "weights":{"max_stress":1.0,"mass":1.0}
        }),
        error_path: "",
    }
}

fn invalid_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    let mut invalid_metric = valid();
    invalid_metric.payload["max_stress"] = Value::Null;
    invalid_metric.payload["von_mises_peak"] = json!(1.0);
    invalid_metric.error_path = "payload.max_stress";
    cases.push(invalid_metric);
    let mut unknown = valid();
    unknown.config["enabled_terms"][1] = json!("typo_mass");
    unknown.error_path = "config.enabled_terms[1]";
    cases.push(unknown);
    let mut target = valid();
    target.config["targets"]["mass"] = json!(-1.0);
    target.error_path = "config.targets.mass";
    cases.push(target);
    let mut weight = valid();
    weight.config["weights"]["max_stress"] = json!("1");
    weight.error_path = "config.weights.max_stress";
    cases.push(weight);
    let mut ratio = valid();
    ratio.payload["max_stress"] = json!(1e308);
    ratio.config["targets"]["max_stress"] = json!(0.1);
    ratio.error_path = "max_stress.ratio";
    cases.push(ratio);
    let mut penalty = valid();
    penalty.payload["max_stress"] = json!(1e308);
    penalty.config["weights"]["max_stress"] = json!(4.0);
    penalty.error_path = "max_stress.penalty";
    cases.push(penalty);
    let mut total = valid();
    total.payload = json!({"max_stress":1e308,"mass":1e308});
    total.config["targets"] = json!({"max_stress":1.0,"mass":1.0});
    total.error_path = "quality_score";
    cases.push(total);
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
    let mut config = case.config.clone();
    if recover {
        config["on_error"] = json!("skip");
    }
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"quality-validation",
            "name":"Quality evidence recovery","version":"1.0.0",
            "entry_nodes":["input","independent"],"output_nodes":["raw","decision","independent_output"],
            "nodes":[
                {"id":"input","kind":"input","inputs":[],"outputs":[port("payload")]},
                {"id":"assess","kind":"transform","operator_id":"transform.score_structural_quality",
                 "config":config,"inputs":[port("payload")],"outputs":[port("summary")]},
                {"id":"compose","kind":"transform","operator_id":"transform.compose_quality_objective",
                 "inputs":[port("structural")],"outputs":[port("objective")]},
                {"id":"decision","kind":"output","inputs":[port("objective")],"outputs":[]},
                {"id":"raw","kind":"output","inputs":[port("payload")],"outputs":[]},
                {"id":"independent","kind":"input","inputs":[],"outputs":[port("value")]},
                {"id":"independent_output","kind":"output","inputs":[port("value")],"outputs":[]}
            ],
            "edges":[edge("input","payload","assess","payload"),edge("input","payload","raw","payload"),
                     edge("assess","summary","compose","structural"),edge("compose","objective","decision","objective"),
                     edge("independent","value","independent_output","value")]
        },
        "input_artifacts":{"input":case.payload,"independent":{"value":7}}
    })).unwrap()
}

#[test]
fn invalid_quality_evidence_stops_before_composite_objective_creation() {
    for case in invalid_cases() {
        let error = run_workflow_graph(request(&case, false)).unwrap_err();
        assert!(error.contains("workflow node assess failed"), "{error}");
        assert!(
            error.contains("transform.score_structural_quality"),
            "{error}"
        );
        assert!(error.contains(case.error_path), "{error}");
        assert!(!error.contains("panicked"), "{error}");
    }
}

#[test]
fn quality_branch_recovery_preserves_raw_evidence_without_a_false_objective() {
    for case in invalid_cases() {
        let run = run_workflow_graph(request(&case, true)).unwrap();
        assert_eq!(run.failed_nodes, vec!["assess"]);
        assert_eq!(run.skipped_nodes.len(), 2);
        for node in ["compose", "decision"] {
            assert!(run.skipped_nodes.iter().any(|id| id == node));
        }
        for artifact in ["assess.summary", "compose.objective", "decision.objective"] {
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
        assert!(
            trace
                .error_message
                .as_deref()
                .unwrap()
                .contains(case.error_path)
        );
    }
}

#[test]
fn corrected_quality_replay_restores_the_complete_objective_chain() {
    for case in invalid_cases() {
        assert_eq!(
            run_workflow_graph(request(&case, true))
                .unwrap()
                .failed_nodes
                .len(),
            1
        );
        let run = run_workflow_graph(request(&valid(), false)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert!(run.skipped_nodes.is_empty());
        let summary = &run.artifacts["assess.summary"];
        assert_eq!(summary["structural_quality_term_count"], 2);
        assert_eq!(summary["structural_quality_score"], 1.0);
        let objective = &run.artifacts["decision.objective"];
        assert_eq!(objective["composite_quality_ready"], true);
        assert_eq!(objective["composite_quality_score"], 1.0);
    }
}

#[test]
fn missing_metrics_remain_blocking_through_composite_scoring() {
    let mut case = valid();
    case.payload.as_object_mut().unwrap().remove("mass");
    let run = run_workflow_graph(request(&case, false)).unwrap();
    assert!(run.failed_nodes.is_empty());
    let summary = &run.artifacts["assess.summary"];
    assert_eq!(summary["structural_quality_ready"], false);
    assert_eq!(summary["structural_quality_missing_metric_count"], 1);
    let objective = &run.artifacts["decision.objective"];
    assert_eq!(objective["composite_quality_ready"], false);
    assert_eq!(objective["composite_quality_blocked_term_count"], 1);
    assert_eq!(objective["composite_quality_missing_metric_count"], 1);
}
