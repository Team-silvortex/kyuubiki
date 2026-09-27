use kyuubiki_engine::{
    BuiltInOperatorRegistryKind, built_in_operator_registry, run_workflow_graph_with_options,
};
use kyuubiki_protocol::{OperatorRunRequest, WorkflowArtifactProjection, WorkflowGraphRunOptions};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-named-input-contract.json"
    ))
    .unwrap()
}

fn cases(fixture: &Value) -> Vec<Value> {
    let mut cases: Vec<_> = fixture["pair_operators"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| {
            let mut sample = fixture["pair"].clone();
            sample["operator_id"] = id.clone();
            sample
        })
        .collect();
    cases.push(fixture["sweep"].clone());
    cases
}

fn payload(sample: &Value) -> Value {
    Value::Object(
        sample["ports"]
            .as_array()
            .unwrap()
            .iter()
            .zip(sample["values"].as_array().unwrap())
            .map(|(port, value)| (port.as_str().unwrap().to_string(), value.clone()))
            .collect(),
    )
}

fn request(fixture: &Value, sample: &Value, envelope: Option<&str>, reverse: bool) -> Value {
    let mut request = fixture["request"].clone();
    request["input_artifacts"] = json!({"left": sample["values"][0], "right": sample["values"][1]});
    let port = |id: &Value| json!({"id":id,"artifact_type":"artifact/result_summary"});
    let nodes = request["graph"]["nodes"].as_array_mut().unwrap();
    let combine = nodes
        .iter_mut()
        .find(|node| node["id"] == "combine")
        .unwrap();
    combine["operator_id"] = sample["operator_id"].clone();
    combine["config"] = sample["config"].clone();
    combine["inputs"] = Value::Array(match envelope {
        Some(id) => vec![port(&json!(id))],
        None => sample["ports"]
            .as_array()
            .unwrap()
            .iter()
            .map(port)
            .collect(),
    });
    if envelope.is_some() {
        nodes.retain(|node| node["id"] != "right");
    }
    if reverse {
        nodes.reverse();
    }
    let edges = request["graph"]["edges"].as_array_mut().unwrap();
    edges[0]["to"]["port"] = envelope.map_or_else(|| sample["ports"][0].clone(), |id| json!(id));
    edges[1]["to"]["port"] = sample["ports"][1].clone();
    if envelope.is_some() {
        edges.retain(|edge| edge["id"] != "right-combine");
    }
    if reverse {
        edges.reverse();
    }
    if envelope.is_some() {
        request["graph"]["entry_nodes"] = json!(["left"]);
        request["input_artifacts"] = json!({"left":payload(sample)});
    }
    request
}

fn run(request: Value, projection: WorkflowArtifactProjection) -> Result<Value, String> {
    run_workflow_graph_with_options(
        serde_json::from_value(request).unwrap(),
        WorkflowGraphRunOptions {
            artifact_projection: projection,
        },
    )
    .map(|result| serde_json::to_value(result).unwrap())
}

#[test]
fn shared_named_inputs_match_direct_operators_in_full_and_outputs_modes() {
    let fixture = fixture();
    let registry = built_in_operator_registry(BuiltInOperatorRegistryKind::Transform);
    for sample in cases(&fixture) {
        let id = sample["operator_id"].as_str().unwrap();
        let direct = registry
            .run(OperatorRunRequest {
                operator_id: id.to_string(),
                input: json!({"payload":payload(&sample),"config":sample["config"]}),
                context: Default::default(),
            })
            .unwrap()
            .summary;
        for (key, value) in sample["expected"].as_object().unwrap() {
            // JSON's Rust integer/float representation differs; normalize numeric expectations.
            if value.is_number() {
                assert_eq!(direct[key].as_f64(), value.as_f64(), "{id}: {key}");
            } else {
                assert_eq!(&direct[key], value, "{id}: {key}");
            }
        }
        for reverse in [false, true] {
            for envelope in [None, Some("input"), Some("payload")] {
                for projection in [
                    WorkflowArtifactProjection::All,
                    WorkflowArtifactProjection::Outputs,
                ] {
                    let request = request(&fixture, &sample, envelope, reverse);
                    let raw = request["input_artifacts"]["left"].clone();
                    let result =
                        run(request, projection).unwrap_or_else(|error| panic!("{id}: {error}"));
                    assert_eq!(result["failed_nodes"], json!([]));
                    assert_eq!(result["artifacts"]["result.summary"], direct);
                    assert_eq!(result["artifacts"]["raw.payload"], raw);
                    let trace = result["node_runs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|trace| trace["node_id"] == "combine")
                        .unwrap();
                    let mut consumed: Vec<_> = trace["consumed_artifacts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap())
                        .collect();
                    consumed.sort();
                    assert_eq!(
                        consumed,
                        if envelope.is_some() {
                            vec!["left.payload"]
                        } else {
                            vec!["left.payload", "right.payload"]
                        }
                    );
                    if projection == WorkflowArtifactProjection::Outputs {
                        assert_eq!(result["artifacts"].as_object().unwrap().len(), 2);
                    }
                }
            }
        }
    }
}

#[test]
fn pair_target_ports_override_edge_order_and_nested_left_right_fields() {
    let fixture = fixture();
    for mut sample in pair_cases(&fixture) {
        sample["values"][0] =
            json!({"cost":10,"left":{"cost":1},"right":{"cost":99},"payload":{"cost":1}});
        for reverse in [false, true] {
            let request = request(&fixture, &sample, None, reverse);
            let result = run(request.clone(), WorkflowArtifactProjection::Outputs).unwrap();
            assert_eq!(
                result["artifacts"]["result.summary"]["benchmark_winner"],
                "right"
            );
            let mut swapped = request;
            for edge in swapped["graph"]["edges"].as_array_mut().unwrap() {
                if edge["to"]["node"] == "combine" {
                    edge["to"]["port"] = json!(if edge["to"]["port"] == "left" {
                        "right"
                    } else {
                        "left"
                    });
                }
            }
            let result = run(swapped, WorkflowArtifactProjection::Outputs).unwrap();
            assert_eq!(
                result["artifacts"]["result.summary"]["benchmark_winner"],
                "left"
            );
        }
    }
}

#[test]
fn invalid_second_inputs_never_publish_a_winner_and_replay_cleanly() {
    let fixture = fixture();
    for sample in pair_cases(&fixture) {
        for reverse in [false, true] {
            let mut bad = request(&fixture, &sample, None, reverse);
            bad["input_artifacts"]["right"] = Value::Null;
            assert!(
                run(bad.clone(), WorkflowArtifactProjection::All)
                    .unwrap_err()
                    .contains("combine")
            );
            let nodes = bad["graph"]["nodes"].as_array_mut().unwrap();
            nodes
                .iter_mut()
                .find(|node| node["id"] == "combine")
                .unwrap()["config"]["on_error"] = json!("skip");
            let result = run(bad, WorkflowArtifactProjection::Outputs).unwrap();
            assert_eq!(result["failed_nodes"], json!(["combine"]));
            assert_eq!(result["skipped_nodes"], json!(["result"]));
            assert_eq!(result["artifacts"].as_object().unwrap().len(), 1);
            assert_eq!(result["artifacts"]["raw.payload"], sample["values"][0]);
            assert!(
                !result["artifact_lineage"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|lineage| lineage["node_id"] == "combine")
            );
            let clean = run(
                request(&fixture, &sample, None, reverse),
                WorkflowArtifactProjection::All,
            )
            .unwrap();
            assert_eq!(clean["failed_nodes"], json!([]));
        }
    }
}

fn pair_cases(fixture: &Value) -> impl Iterator<Item = Value> {
    cases(fixture)
        .into_iter()
        .filter(|sample| sample["operator_id"] != "transform.join_parameter_sweep_results")
}

#[test]
fn same_source_fanout_preserves_both_bindings_under_transient_retention() {
    let fixture = fixture();
    for sample in pair_cases(&fixture) {
        for reverse in [false, true] {
            let mut request = request(&fixture, &sample, None, reverse);
            for edge in request["graph"]["edges"].as_array_mut().unwrap() {
                if edge["id"] == "right-combine" {
                    edge["from"]["node"] = json!("left");
                }
            }
            let result = run(request, WorkflowArtifactProjection::Outputs).unwrap();
            assert_eq!(
                result["artifacts"]["result.summary"]["benchmark_winner"],
                "tie"
            );
            assert_eq!(result["artifacts"]["raw.payload"], sample["values"][0]);
            let trace = result["node_runs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|trace| trace["node_id"] == "combine")
                .unwrap();
            assert_eq!(
                trace["consumed_artifacts"],
                json!(["left.payload", "left.payload"])
            );
        }
    }
}

#[test]
fn a_missing_named_edge_never_reinterprets_nested_fields_as_an_envelope() {
    let fixture = fixture();
    for sample in pair_cases(&fixture) {
        for reverse in [false, true] {
            let mut request = request(&fixture, &sample, None, reverse);
            request["input_artifacts"]["left"] =
                json!({"cost":10,"left":{"cost":1},"right":{"cost":99}});
            request["graph"]["edges"]
                .as_array_mut()
                .unwrap()
                .retain(|edge| edge["id"] != "right-combine");
            assert!(
                run(request, WorkflowArtifactProjection::Outputs)
                    .unwrap_err()
                    .contains("combine")
            );
        }
    }
}

#[test]
fn single_field_source_port_envelopes_preserve_legacy_graphs_and_raw_artifacts() {
    let fixture = fixture();
    for sample in cases(&fixture) {
        for reverse in [false, true] {
            let mut request = request(&fixture, &sample, None, reverse);
            for value in request["input_artifacts"]
                .as_object_mut()
                .unwrap()
                .values_mut()
            {
                *value = json!({"payload":value});
            }
            let raw = request["input_artifacts"]["left"].clone();
            let result = run(request, WorkflowArtifactProjection::Outputs).unwrap();
            assert_eq!(result["artifacts"]["raw.payload"], raw);
            for (key, expected) in sample["expected"].as_object().unwrap() {
                let value = &result["artifacts"]["result.summary"][key];
                if expected.is_number() {
                    assert_eq!(value.as_f64(), expected.as_f64());
                } else {
                    assert_eq!(value, expected);
                }
            }
        }
    }
}
