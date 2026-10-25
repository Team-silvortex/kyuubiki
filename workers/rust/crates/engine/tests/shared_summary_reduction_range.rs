use kyuubiki_engine::{
    BuiltInOperatorRegistryKind, built_in_operator_registry, run_workflow_graph_with_options,
};
use kyuubiki_protocol::{OperatorRunRequest, WorkflowArtifactProjection, WorkflowGraphRunOptions};
use serde_json::{Value, json};

fn cases() -> Vec<Value> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-summary-reduction-range.json"
    ))
    .unwrap();
    fixture["cases"].as_array().unwrap().clone()
}

fn direct(sample: &Value) -> Result<Value, String> {
    built_in_operator_registry(BuiltInOperatorRegistryKind::Transform)
        .run(OperatorRunRequest {
            operator_id: sample["operator_id"].as_str().unwrap().into(),
            input: json!({"payload":sample["payload"],"config":sample["config"]}),
            context: Default::default(),
        })
        .map(|result| result.summary)
        .map_err(|error| error.to_string())
}

fn verify(operator: &str) {
    for sample in cases()
        .iter()
        .filter(|sample| sample["operator_id"] == operator)
    {
        if let Some(field) = sample.get("error") {
            let error = direct(sample).unwrap_err();
            assert!(error.contains(operator), "{error}");
            assert!(error.contains(field.as_str().unwrap()), "{error}");
        } else {
            let result = direct(sample).unwrap();
            for (key, expected) in sample["expected"].as_object().unwrap() {
                if expected.is_number() {
                    assert_eq!(
                        result[key].as_f64(),
                        expected.as_f64(),
                        "{}: {key}",
                        sample["id"]
                    );
                } else {
                    assert_eq!(&result[key], expected, "{}: {key}", sample["id"]);
                }
            }
        }
    }
}

#[test]
fn shared_summary_aggregate_range_and_cancellation() {
    verify("transform.aggregate_summary_collection");
}

#[test]
fn shared_summary_normalize_rejects_range_and_rule_failures_before_clamping() {
    verify("transform.normalize_summary_fields");
}

#[test]
fn shared_summary_selection_rejects_invalid_scores_before_ranking() {
    verify("transform.select_best_summary");
}

fn request(sample: &Value, skip: bool) -> Value {
    let single = sample["shape"] == "single";
    let inputs = if single {
        json!({"left": sample["payload"]})
    } else {
        sample["payload"].clone()
    };
    let sources: Vec<_> = inputs.as_object().unwrap().keys().cloned().collect();
    let port = |id: &str| json!({"id":id,"artifact_type":"artifact/json"});
    let edge = |from: &str, to: &str, target: &str| {
        json!({
            "id":format!("{from}-{to}"),"from":{"node":from,"port":"payload"},
            "to":{"node":to,"port":target},"artifact_type":"artifact/json"
        })
    };
    let mut nodes: Vec<_> = sources
        .iter()
        .map(|id| {
            json!({
                "id":id,"kind":"input","inputs":[],"outputs":[port("payload")]
            })
        })
        .collect();
    let mut config = sample["config"].clone();
    if skip {
        config["on_error"] = json!("skip");
    }
    nodes.push(json!({"id":"assess","kind":"transform","operator_id":sample["operator_id"],
        "config":config,"inputs":sources.iter().map(|id| port(if single { "payload" } else { id })).collect::<Vec<_>>(),
        "outputs":[port("payload")]}));
    for id in ["result", "raw"] {
        nodes.push(json!({"id":id,"kind":"output","inputs":[port("payload")],"outputs":[]}));
    }
    let mut edges: Vec<_> = sources
        .iter()
        .map(|id| edge(id, "assess", if single { "payload" } else { id }))
        .collect();
    edges.extend([
        edge("assess", "result", "payload"),
        edge("left", "raw", "payload"),
    ]);
    json!({"graph":{"schema_version":"kyuubiki.workflow-graph/v1","id":"summary-reduction-range",
        "name":"Summary reduction range","version":"1.0.0","nodes":nodes,"edges":edges,
        "entry_nodes":sources,"output_nodes":["result","raw"]},"input_artifacts":inputs})
}

#[test]
fn shared_summary_reduction_failures_isolate_without_partial_results_and_replay_cleanly() {
    let cases = cases();
    for sample in &cases {
        let input = request(sample, false);
        let raw = input["input_artifacts"]["left"].clone();
        let run = run_workflow_graph_with_options(
            serde_json::from_value(input).unwrap(),
            Default::default(),
        );
        if sample.get("error").is_none() {
            assert_eq!(
                run.unwrap().artifacts["result.payload"],
                direct(sample).unwrap()
            );
            continue;
        }
        assert!(run.unwrap_err().contains("assess"));
        for projection in [
            WorkflowArtifactProjection::All,
            WorkflowArtifactProjection::Outputs,
            WorkflowArtifactProjection::None,
        ] {
            let failed = run_workflow_graph_with_options(
                serde_json::from_value(request(sample, true)).unwrap(),
                WorkflowGraphRunOptions {
                    artifact_projection: projection,
                },
            )
            .unwrap();
            assert_eq!(failed.failed_nodes, ["assess"]);
            assert_eq!(failed.skipped_nodes, ["result"]);
            assert!(!failed.artifacts.contains_key("assess.payload"));
            assert!(!failed.artifacts.contains_key("result.payload"));
            assert!(
                !failed
                    .artifact_lineage
                    .iter()
                    .any(|entry| entry.node_id == "assess")
            );
            if projection == WorkflowArtifactProjection::None {
                assert!(failed.artifacts.is_empty());
            } else {
                assert_eq!(failed.artifacts["raw.payload"], raw);
            }
        }
        let clean = cases
            .iter()
            .find(|other| {
                other["operator_id"] == sample["operator_id"] && other.get("error").is_none()
            })
            .unwrap();
        let fresh = run_workflow_graph_with_options(
            serde_json::from_value(request(clean, false)).unwrap(),
            Default::default(),
        )
        .unwrap();
        assert!(fresh.failed_nodes.is_empty());
        assert_eq!(fresh.artifacts["result.payload"], direct(clean).unwrap());
    }
}

#[test]
fn shared_summary_numeric_options_cannot_silently_default() {
    let cases = cases();
    let normalize = cases
        .iter()
        .find(|sample| sample["id"] == "normalize-regular-clamp")
        .unwrap();
    let select = cases
        .iter()
        .find(|sample| sample["id"] == "select-regular")
        .unwrap();
    for bad in [Value::Null, json!(true), json!("bad"), json!([]), json!({})] {
        for key in ["scale", "offset", "clamp_min", "clamp_max"] {
            let mut sample = normalize.clone();
            sample["config"]["rules"] = json!([{"source":"metric",key:bad}]);
            let error = direct(&sample).unwrap_err();
            assert!(error.contains(&format!("config.rules[0].{key}")), "{error}");
        }
        let mut sample = select.clone();
        sample["config"]["criteria"] = json!([{"field":"metric","weight":bad}]);
        assert!(
            direct(&sample)
                .unwrap_err()
                .contains("config.criteria[0].weight")
        );
    }
}

#[test]
fn shared_summary_aggregate_preserves_tails_under_permutation_and_binary_scaling() {
    let mut sample = cases().remove(0);
    for exponent in [-900, 0, 900] {
        let scale = 2.0_f64.powi(exponent);
        let values = [2.0_f64.powi(60) * scale, scale, -2.0_f64.powi(60) * scale];
        for order in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            sample["payload"] = json!({"left":{"metric":values[order[0]]},
                "middle":{"metric":values[order[1]]},"right":{"metric":values[order[2]]}});
            assert_eq!(
                direct(&sample).unwrap()["aggregate_metric_mean"].as_f64(),
                Some(scale / 3.0)
            );
        }
    }
    sample["payload"] = json!({"left":{"metric":f64::MAX},"middle":{"metric":f64::MAX},"right":{"metric":f64::MAX}});
    assert_eq!(
        direct(&sample).unwrap()["aggregate_metric_mean"].as_f64(),
        Some(f64::MAX)
    );
}
