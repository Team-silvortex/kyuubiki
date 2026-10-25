use kyuubiki_engine::{
    BuiltInOperatorRegistryKind, built_in_operator_registry, run_workflow_graph_with_options,
};
use kyuubiki_protocol::{OperatorRunRequest, WorkflowArtifactProjection, WorkflowGraphRunOptions};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-summary-admission.json"
    ))
    .unwrap()
}

fn direct(sample: &Value, payload: Value) -> Result<Value, String> {
    let kind = if sample["kind"] == "extract" {
        BuiltInOperatorRegistryKind::Extract
    } else {
        BuiltInOperatorRegistryKind::Transform
    };
    built_in_operator_registry(kind)
        .run(OperatorRunRequest {
            operator_id: sample["operator_id"].as_str().unwrap().to_owned(),
            input: json!({"payload":payload,"config":sample["config"]}),
            context: Default::default(),
        })
        .map(|result| result.summary)
        .map_err(|error| error.to_string())
}

fn locations(sample: &Value) -> Vec<Option<&str>> {
    if sample["shape"] == "single" {
        vec![None]
    } else {
        vec![None, Some("left"), Some("right")]
    }
}

fn marked(sample: &Value, location: Option<&str>, marker: &Value) -> Value {
    let mut payload = sample["payload"].clone();
    let target = match location {
        Some(key) => &mut payload[key],
        None => &mut payload,
    };
    target
        .as_object_mut()
        .unwrap()
        .extend(marker.as_object().unwrap().clone());
    payload
}

#[test]
fn shared_summary_admission_rejects_invalid_final_status_at_each_contract_location() {
    let fixture = fixture();
    for sample in fixture["cases"].as_array().unwrap() {
        for marker in fixture["invalid_markers"].as_array().unwrap() {
            for location in locations(sample) {
                let error = direct(sample, marked(sample, location, marker)).unwrap_err();
                assert!(
                    error.contains(sample["operator_id"].as_str().unwrap()),
                    "{error}"
                );
                let path = location.map_or("payload.".to_owned(), |key| format!("payload.{key}."));
                assert!(error.contains(&path), "{error}");
            }
        }
    }
}

#[test]
fn shared_summary_admission_keeps_valid_metrics_and_trial_history() {
    let fixture = fixture();
    for sample in fixture["cases"].as_array().unwrap() {
        for marker in fixture["valid_markers"].as_array().unwrap() {
            for location in locations(sample) {
                let mut marker = marker.clone();
                if sample["shape"] == "collection" && location.is_none() {
                    marker
                        .as_object_mut()
                        .unwrap()
                        .retain(|key, _| ["converged", "stability_result"].contains(&key.as_str()));
                }
                let result = direct(sample, marked(sample, location, &marker)).unwrap();
                for (key, expected) in sample["expected"].as_object().unwrap() {
                    if expected.is_number() {
                        assert_eq!(result[key].as_f64(), expected.as_f64());
                    } else {
                        assert_eq!(&result[key], expected);
                    }
                }
            }
        }
    }
}

fn request(sample: &Value, marker: &Value, source: &str, skip: bool) -> Value {
    let base: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-named-input-contract.json"
    ))
    .unwrap();
    let mut request = base["request"].clone();
    let single = sample["shape"] == "single";
    request["input_artifacts"] = if single {
        json!({"left":sample["payload"]})
    } else {
        sample["payload"].clone()
    };
    request["input_artifacts"][source]
        .as_object_mut()
        .unwrap()
        .extend(marker.as_object().unwrap().clone());
    let nodes = request["graph"]["nodes"].as_array_mut().unwrap();
    if single {
        nodes.retain(|node| node["id"] != "right");
    }
    let assess = nodes
        .iter_mut()
        .find(|node| node["id"] == "combine")
        .unwrap();
    assess["kind"] = sample["kind"].clone();
    assess["operator_id"] = sample["operator_id"].clone();
    assess["config"] = sample["config"].clone();
    if skip {
        assess["config"]["on_error"] = json!("skip");
    }
    if single {
        assess["inputs"] = json!([{"id":"payload","artifact_type":"artifact/result_summary"}]);
        request["graph"]["entry_nodes"] = json!(["left"]);
    }
    let edges = request["graph"]["edges"].as_array_mut().unwrap();
    if single {
        edges.retain(|edge| edge["id"] != "right-combine");
        edges
            .iter_mut()
            .find(|edge| edge["id"] == "left-combine")
            .unwrap()["to"]["port"] = json!("payload");
    }
    edges
        .iter_mut()
        .find(|edge| edge["id"] == "left-raw")
        .unwrap()["from"]["node"] = json!(source);
    request
}

#[test]
fn shared_summary_admission_failures_isolate_without_erasing_raw_evidence() {
    let fixture = fixture();
    for sample in fixture["cases"].as_array().unwrap() {
        for marker in fixture["invalid_markers"].as_array().unwrap() {
            let sources: &[&str] = if sample["shape"] == "single" {
                &["left"]
            } else {
                &["left", "right"]
            };
            for source in sources {
                let fail_fast = request(sample, marker, source, false);
                assert!(
                    run_workflow_graph_with_options(
                        serde_json::from_value(fail_fast).unwrap(),
                        Default::default()
                    )
                    .unwrap_err()
                    .contains("combine")
                );
                for projection in [
                    WorkflowArtifactProjection::All,
                    WorkflowArtifactProjection::Outputs,
                    WorkflowArtifactProjection::None,
                ] {
                    let bad = request(sample, marker, source, true);
                    let raw = bad["input_artifacts"][source].clone();
                    let result = run_workflow_graph_with_options(
                        serde_json::from_value(bad).unwrap(),
                        WorkflowGraphRunOptions {
                            artifact_projection: projection,
                        },
                    )
                    .unwrap();
                    assert_eq!(result.failed_nodes, ["combine"]);
                    assert_eq!(result.skipped_nodes, ["result"]);
                    assert!(!result.artifacts.contains_key("combine.summary"));
                    assert!(
                        !result
                            .artifact_lineage
                            .iter()
                            .any(|entry| entry.node_id == "combine")
                    );
                    if projection != WorkflowArtifactProjection::None {
                        assert_eq!(result.artifacts["raw.payload"], raw);
                    }
                }
                let clean = run_workflow_graph_with_options(
                    serde_json::from_value(request(sample, &json!({}), source, false)).unwrap(),
                    Default::default(),
                )
                .unwrap();
                assert!(clean.failed_nodes.is_empty());
                for (key, expected) in sample["expected"].as_object().unwrap() {
                    let actual = &clean.artifacts["result.summary"][key];
                    if expected.is_number() {
                        assert_eq!(actual.as_f64(), expected.as_f64());
                    } else {
                        assert_eq!(actual, expected);
                    }
                }
            }
        }
    }
}

#[test]
fn shared_summary_admission_status_metadata_is_not_a_candidate() {
    let fixture = fixture();
    for sample in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|sample| sample["shape"] == "collection")
    {
        assert!(
            direct(
                sample,
                json!({"converged":true,"stability_result":{"converged":true,"cost":0}})
            )
            .is_err()
        );
    }
}

#[test]
fn shared_summary_admission_preserves_raw_failed_result_export() {
    let payload = json!({"converged":false,"cost":1,"achieved_load_factor":0.25});
    let exported = built_in_operator_registry(BuiltInOperatorRegistryKind::Export)
        .run(OperatorRunRequest {
            operator_id: "export.summary_json".to_owned(),
            input: json!({"payload":payload,"config":{}}),
            context: Default::default(),
        })
        .unwrap()
        .summary;
    assert_eq!(
        serde_json::from_str::<Value>(exported["content"].as_str().unwrap()).unwrap(),
        payload
    );
}
