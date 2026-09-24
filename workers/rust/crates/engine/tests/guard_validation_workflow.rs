use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraphRunRequest, WorkflowNodeRunStatus};
use serde_json::{Value, json};

struct Case {
    operator: &'static str,
    payload: Value,
    config: Value,
    error_path: &'static str,
}

fn valid(pair: bool) -> Case {
    if pair {
        Case {
            operator: "transform.benchmark_structural_pair",
            payload: json!({"left":{"max_stress":1.0,"mass":1.0},"right":{"max_stress":2.0,"mass":2.0}}),
            config: json!({"criteria":[{"field":"max_stress"},{"field":"mass"}]}),
            error_path: "",
        }
    } else {
        Case {
            operator: "transform.evaluate_structural_guard",
            payload: json!({"max_stress":1.0,"mass":1.0}),
            config: json!({"rules":[{"field":"max_stress","threshold":10.0},{"field":"mass","threshold":10.0}]}),
            error_path: "",
        }
    }
}

fn invalid_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for pair in [false, true] {
        let mut missing = valid(pair);
        if pair {
            missing.payload["left"]
                .as_object_mut()
                .unwrap()
                .remove("mass");
            missing.error_path = "payload.left.mass";
        } else {
            missing.payload.as_object_mut().unwrap().remove("mass");
            missing.error_path = "payload.mass";
        }
        cases.push(missing);
        let mut bad_config = valid(pair);
        if pair {
            bad_config.config["criteria"][0]["goal"] = json!("higher");
            bad_config.error_path = "config.criteria[0].goal";
        } else {
            bad_config.config["rules"][0]["comparison"] = json!("greater");
            bad_config.error_path = "config.rules[0].comparison";
        }
        cases.push(bad_config);
    }
    let mut overflow = valid(true);
    overflow.payload["left"]["max_stress"] = json!(-1e308);
    overflow.payload["right"]["max_stress"] = json!(1e308);
    overflow.error_path = "config.criteria[0].delta";
    cases.push(overflow);
    let mut score_overflow = valid(true);
    score_overflow.config["criteria"][0]["weight"] = json!(1e308);
    score_overflow.config["criteria"][1]["weight"] = json!(1e308);
    score_overflow.error_path = "left_score";
    cases.push(score_overflow);
    let mut labels = valid(true);
    labels.config["left_label"] = json!("same");
    labels.config["right_label"] = json!("same");
    labels.error_path = "config.left_label";
    cases.push(labels);
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
            "schema_version":"kyuubiki.workflow-graph/v1","id":"guard-validation",
            "name":"Guard validation recovery","version":"1.0.0",
            "entry_nodes":["input","independent"],"output_nodes":["raw","decision","independent_output"],
            "nodes":[
                {"id":"input","kind":"input","inputs":[],"outputs":[port("payload")]},
                {"id":"assess","kind":"transform","operator_id":case.operator,"config":config,
                 "inputs":[port("payload")],"outputs":[port("summary")]},
                {"id":"decision","kind":"output","inputs":[port("summary")],"outputs":[]},
                {"id":"raw","kind":"output","inputs":[port("payload")],"outputs":[]},
                {"id":"independent","kind":"input","inputs":[],"outputs":[port("value")]},
                {"id":"independent_output","kind":"output","inputs":[port("value")],"outputs":[]}
            ],
            "edges":[edge("input","payload","assess","payload"),edge("input","payload","raw","payload"),
                     edge("assess","summary","decision","summary"),edge("independent","value","independent_output","value")]
        },
        "input_artifacts":{"input":case.payload,"independent":{"value":7}}
    })).unwrap()
}

#[test]
fn invalid_rule_evidence_stops_the_default_workflow_with_its_exact_cause() {
    for case in invalid_cases() {
        let error = run_workflow_graph(request(&case, false)).unwrap_err();
        assert!(error.contains("workflow node assess failed"), "{error}");
        assert!(error.contains(case.operator), "{error}");
        assert!(error.contains(case.error_path), "{error}");
        assert!(!error.contains("panicked"), "{error}");
    }
}

#[test]
fn invalid_rule_recovery_preserves_raw_inputs_and_independent_work_without_a_verdict() {
    for case in invalid_cases() {
        let run = run_workflow_graph(request(&case, true)).unwrap();
        assert_eq!(run.failed_nodes, vec!["assess"]);
        assert_eq!(run.skipped_nodes, vec!["decision"]);
        assert!(!run.artifacts.contains_key("assess.summary"));
        assert!(!run.artifacts.contains_key("decision.summary"));
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
fn corrected_evidence_replay_checks_every_rule_and_criterion() {
    for case in invalid_cases() {
        let failed = run_workflow_graph(request(&case, true)).unwrap();
        assert_eq!(failed.failed_nodes.len(), 1);
        let pair = case.operator.contains("benchmark");
        let run = run_workflow_graph(request(&valid(pair), false)).unwrap();
        assert!(run.failed_nodes.is_empty());
        assert!(run.skipped_nodes.is_empty());
        let verdict = &run.artifacts["decision.summary"];
        if pair {
            assert_eq!(verdict["benchmark_winner"], "left");
            assert_eq!(verdict["benchmark_criteria_count"], 2);
        } else {
            assert_eq!(verdict["guard_status"], "pass");
            assert_eq!(verdict["guard_checked_rule_count"], 2);
        }
    }
}
