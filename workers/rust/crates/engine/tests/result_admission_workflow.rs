use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraphRunRequest, WorkflowNodeRunStatus};
use serde_json::{Value, json};

fn request(
    contact: bool,
    iterations: usize,
    recover: bool,
    extract_first: bool,
) -> WorkflowGraphRunRequest {
    let family = if contact {
        "contact_gap_1d"
    } else {
        "nonlinear_spring_1d"
    };
    let model_type = format!("study_model/{family}");
    let result_type = format!("result/{family}");
    let summary_type = "artifact/result_summary";
    let mut model = json!({
        "nodes":[{"id":"base","x":0.0,"fix_x":true,"load_x":0.0},
                 {"id":"tip","x":1.0,"fix_x":false,"load_x":2.0}],
        "elements":[{"id":"spring","node_i":0,"node_j":1,"stiffness":1.0,
                     "cubic_stiffness":if contact { 0.0 } else { 1.0 }}],
        "load_steps":if contact { 4 } else { 1 },"max_iterations":iterations,"tolerance":1e-12
    });
    if contact {
        model["contacts"] = json!([{"id":"stop","node":1,"gap":1.0,"normal_stiffness":10.0}]);
    }
    let mut score_config = json!({
        "enabled_terms":["max_displacement"],"targets":{"max_displacement":10.0}
    });
    let mut extract_config = json!({"fields":["max_displacement"]});
    if recover {
        score_config["on_error"] = json!("skip");
        extract_config["on_error"] = json!("skip");
    }
    let mut nodes = vec![
        json!({"id":"input","kind":"input","inputs":[],"outputs":[port("model",&model_type)]}),
        json!({"id":"solve","kind":"solve","operator_id":format!("solve.{family}"),
               "inputs":[port("model",&model_type)],"outputs":[port("result",&result_type)]}),
        json!({"id":"score","kind":"transform","operator_id":"transform.score_structural_quality",
               "config":score_config,"inputs":[port("payload",if extract_first { summary_type } else { &result_type })],
               "outputs":[port("summary",summary_type)]}),
        json!({"id":"decision","kind":"output","inputs":[port("summary",summary_type)],"outputs":[]}),
        json!({"id":"raw","kind":"output","inputs":[port("result",&result_type)],"outputs":[]}),
        json!({"id":"independent","kind":"input","inputs":[],"outputs":[port("value",summary_type)]}),
        json!({"id":"independent_output","kind":"output","inputs":[port("value",summary_type)],"outputs":[]}),
    ];
    let mut edges = vec![
        edge("input", "model", "solve", "model", &model_type),
        edge("solve", "result", "raw", "result", &result_type),
        edge("score", "summary", "decision", "summary", summary_type),
        edge(
            "independent",
            "value",
            "independent_output",
            "value",
            summary_type,
        ),
    ];
    if extract_first {
        nodes.push(json!({"id":"extract","kind":"extract","operator_id":"extract.result_summary",
            "config":extract_config,"inputs":[port("payload",&result_type)],"outputs":[port("summary",summary_type)]}));
        edges.push(edge("solve", "result", "extract", "payload", &result_type));
        edges.push(edge("extract", "summary", "score", "payload", summary_type));
    } else {
        edges.push(edge("solve", "result", "score", "payload", &result_type));
    }
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"result-admission",
            "name":"Incomplete result admission","version":"1.0.0",
            "entry_nodes":["input","independent"],"output_nodes":["raw","decision","independent_output"],
            "nodes":nodes,"edges":edges
        },
        "input_artifacts":{"input":model,"independent":{"value":7}}
    })).unwrap()
}

fn port(id: &str, artifact: &str) -> Value {
    json!({"id":id,"artifact_type":artifact})
}

fn edge(from: &str, output: &str, to: &str, input: &str, artifact: &str) -> Value {
    json!({"id":format!("{from}-{to}"),"from":{"node":from,"port":output},
           "to":{"node":to,"port":input},"artifact_type":artifact})
}

#[test]
fn partial_solver_results_fail_fast_before_quality_or_summary_publication() {
    for contact in [false, true] {
        for extract_first in [false, true] {
            let error = run_workflow_graph(request(contact, 1, false, extract_first)).unwrap_err();
            let node = if extract_first { "extract" } else { "score" };
            assert!(
                error.contains(&format!("workflow node {node} failed")),
                "{error}"
            );
            assert!(error.contains("payload.converged=false"), "{error}");
            assert!(error.contains("achieved_load_factor="), "{error}");
        }
    }
}

#[test]
fn explicit_recovery_keeps_diagnostics_and_independent_work_without_a_false_quality_result() {
    for contact in [false, true] {
        for extract_first in [false, true] {
            let run = run_workflow_graph(request(contact, 1, true, extract_first)).unwrap();
            let failed = if extract_first { "extract" } else { "score" };
            assert_eq!(run.failed_nodes, vec![failed]);
            assert!(run.skipped_nodes.iter().any(|node| node == "decision"));
            if extract_first {
                assert!(run.skipped_nodes.iter().any(|node| node == "score"));
                assert!(!run.artifacts.contains_key("extract.summary"));
            }
            assert!(!run.artifacts.contains_key("score.summary"));
            assert!(!run.artifacts.contains_key("decision.summary"));
            assert_eq!(
                run.artifacts["independent_output.value"],
                json!({"value":7})
            );
            let raw = &run.artifacts["raw.result"];
            assert_eq!(raw["converged"], false);
            assert_eq!(raw["achieved_load_factor"], if contact { 0.5 } else { 0.0 });
            assert_eq!(raw["residual_norm"], 0.0);
            let trace = run
                .node_runs
                .iter()
                .find(|trace| trace.node_id == failed)
                .unwrap();
            assert_eq!(trace.status, WorkflowNodeRunStatus::Failed);
            assert!(
                trace
                    .error_message
                    .as_deref()
                    .unwrap()
                    .contains("payload.converged=false")
            );
        }
    }
}

#[test]
fn full_load_replay_after_failure_completes_direct_and_summary_quality_chains() {
    for contact in [false, true] {
        for extract_first in [false, true] {
            let failed = run_workflow_graph(request(contact, 1, true, extract_first)).unwrap();
            assert!(!failed.failed_nodes.is_empty());
            let run = run_workflow_graph(request(contact, 32, false, extract_first)).unwrap();
            assert!(run.failed_nodes.is_empty());
            assert!(run.skipped_nodes.is_empty());
            assert_eq!(run.artifacts["raw.result"]["converged"], true);
            assert_eq!(run.artifacts["raw.result"]["achieved_load_factor"], 1.0);
            assert_eq!(
                run.artifacts["decision.summary"]["structural_quality_ready"],
                true
            );
            assert_eq!(
                run.artifacts["independent_output.value"],
                json!({"value":7})
            );
        }
    }
}
