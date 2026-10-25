use kyuubiki_engine::{
    BuiltInOperatorRegistryKind, built_in_operator_registry, run_workflow_graph_with_options,
};
use kyuubiki_protocol::{OperatorRunRequest, WorkflowArtifactProjection, WorkflowGraphRunOptions};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-summary-comparison-range.json"
    ))
    .unwrap()
}

fn direct(sample: &Value) -> Result<Value, String> {
    built_in_operator_registry(BuiltInOperatorRegistryKind::Transform)
        .run(OperatorRunRequest {
            operator_id: "transform.compare_summary_pair".into(),
            input: json!({"payload":sample["payload"],"config":sample["config"]}),
            context: Default::default(),
        })
        .map(|result| result.summary)
        .map_err(|error| error.to_string())
}

#[test]
fn shared_summary_comparison_preserves_enabled_representable_metrics() {
    let fixture = fixture();
    for sample in fixture["cases"].as_array().unwrap() {
        if sample.get("error").is_some() {
            continue;
        }
        let result = direct(sample).unwrap();
        for (key, expected) in sample["expected"].as_object().unwrap() {
            assert_eq!(
                result[key].as_f64(),
                expected.as_f64(),
                "{}: {key}",
                sample["id"]
            );
        }
        for absent in sample["absent"].as_array().unwrap() {
            assert!(result.get(absent.as_str().unwrap()).is_none());
        }
    }
}

#[test]
fn shared_summary_comparison_rejects_overflow_instead_of_publishing_null() {
    let fixture = fixture();
    for sample in fixture["cases"].as_array().unwrap() {
        let Some(field) = sample.get("error") else {
            continue;
        };
        let error = direct(sample).unwrap_err();
        assert!(error.contains("transform.compare_summary_pair"), "{error}");
        assert!(error.contains(field.as_str().unwrap()), "{error}");
    }
}

fn request(sample: &Value, skip: bool) -> Value {
    let base: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-named-input-contract.json"
    ))
    .unwrap();
    let mut request = base["request"].clone();
    request["input_artifacts"] = sample["payload"].clone();
    let node = request["graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "combine")
        .unwrap();
    node["operator_id"] = json!("transform.compare_summary_pair");
    node["config"] = sample["config"].clone();
    if skip {
        node["config"]["on_error"] = json!("skip");
    }
    request
}

#[test]
fn shared_summary_comparison_overflow_isolates_without_partial_metrics_or_lineage() {
    let fixture = fixture();
    for sample in fixture["cases"].as_array().unwrap() {
        let raw = sample["payload"]["left"].clone();
        let fail_fast = run_workflow_graph_with_options(
            serde_json::from_value(request(sample, false)).unwrap(),
            Default::default(),
        );
        if sample.get("error").is_none() {
            let result = fail_fast.unwrap();
            assert_eq!(result.artifacts["result.summary"], direct(sample).unwrap());
            assert_eq!(result.artifacts["raw.payload"], raw);
            continue;
        }
        assert!(fail_fast.unwrap_err().contains("combine"));
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
            assert_eq!(failed.failed_nodes, ["combine"]);
            assert_eq!(failed.skipped_nodes, ["result"]);
            assert!(!failed.artifacts.contains_key("combine.summary"));
            assert!(!failed.artifacts.contains_key("result.summary"));
            assert!(
                !failed
                    .artifact_lineage
                    .iter()
                    .any(|entry| entry.node_id == "combine")
            );
            if projection == WorkflowArtifactProjection::None {
                assert!(failed.artifacts.is_empty());
            } else {
                assert_eq!(failed.artifacts["raw.payload"], raw);
            }
        }
        let fresh = run_workflow_graph_with_options(
            serde_json::from_value(request(&fixture["cases"][0], false)).unwrap(),
            Default::default(),
        )
        .unwrap();
        assert!(fresh.failed_nodes.is_empty());
        assert_eq!(
            fresh.artifacts["result.summary"]["delta_metric"],
            json!(4.0)
        );
        assert!(
            fresh.artifacts["result.summary"]
                .get("ratio_metric")
                .is_none()
        );
    }
}

#[test]
fn shared_summary_comparison_zero_baseline_options_do_not_evaluate_undefined_metrics() {
    let fixture = fixture();
    for mask in 0..32 {
        let originals = mask & 1 != 0;
        let delta = mask & 2 != 0;
        let count = mask & 16 != 0;
        let mut sample = fixture["cases"][0].clone();
        sample["config"] = json!({
            "include_originals": originals, "include_delta": delta,
            "include_ratio":mask & 4 != 0, "include_percent_change":mask & 8 != 0,
            "include_shared_field_count":count,
        });
        if originals || delta {
            let result = direct(&sample).unwrap();
            assert!(result.get("ratio_metric").is_none());
            assert!(result.get("percent_change_metric").is_none());
            assert_eq!(result.get("left_metric").is_some(), originals);
            assert_eq!(result.get("delta_metric").is_some(), delta);
            assert_eq!(
                result.get("summary_shared_numeric_field_count").is_some(),
                count
            );
        } else {
            assert!(direct(&sample).is_err());
        }
    }
}

#[test]
fn shared_summary_comparison_relative_metrics_survive_bounded_binary_rescaling() {
    let fixture = fixture();
    for exponent in (-1000..=1000).step_by(100) {
        for sign in [-1.0, 1.0] {
            let scale = sign * 2.0_f64.powi(exponent);
            let mut sample = fixture["cases"][0].clone();
            sample["payload"] = json!({
                "left":{"metric":1.25 * scale}, "right":{"metric":1.5 * scale},
            });
            let result = direct(&sample).unwrap();
            assert_eq!(result["delta_metric"].as_f64(), Some(0.25 * scale));
            assert_eq!(result["ratio_metric"].as_f64(), Some(1.2));
            assert_eq!(result["percent_change_metric"].as_f64(), Some(20.0));
        }
    }
}
