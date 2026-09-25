use kyuubiki_engine::run_workflow_graph;
use serde_json::Value;

fn request(hotspots: bool) -> Value {
    let mut request: Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-report-contract.json"
    ))
    .unwrap();
    if hotspots {
        node(&mut request, "field")["operator_id"] = "extract.field_hotspots".into();
        node(&mut request, "guard")["config"]["rules"][0]["field"] = "v_hotspot_max".into();
    }
    request
}

fn node<'a>(request: &'a mut Value, id: &str) -> &'a mut Value {
    request["graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

#[test]
fn shared_reporting_graph_retains_unknown_counts_and_publishes_real_guard_decisions() {
    for hotspots in [false, true] {
        for threshold in [10, 2] {
            let mut request = request(hotspots);
            node(&mut request, "guard")["config"]["rules"][0]["threshold"] = threshold.into();
            let result =
                run_workflow_graph(serde_json::from_value(request.clone()).unwrap()).unwrap();
            let report: Value = serde_json::from_str(
                result.artifacts["output.summary"]["content"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(
                report["report_guard_status"],
                if threshold == 10 { "pass" } else { "block" }
            );
            assert_eq!(report["guard_payload"]["guard_checked_rule_count"], 1);
            assert_eq!(report["bundle_total_node_count"], Value::Null);
            assert_eq!(
                result.artifacts["raw.payload"],
                request["input_artifacts"]["input"]
            );
        }
    }
}

#[test]
fn shared_reporting_graph_rejects_corrupt_evidence_before_export_and_replays_cleanly() {
    for hotspots in [false, true] {
        for stage in ["field", "bundle", "guard"] {
            let mut bad = request(hotspots);
            let path = match stage {
                "field" => {
                    bad["input_artifacts"]["input"]["nodes"][1]["v"] = Value::Null;
                    "nodes[1].v"
                }
                "bundle" => {
                    bad["input_artifacts"]["extra"]["converged"] = false.into();
                    "payload.extra.converged"
                }
                _ => {
                    node(&mut bad, "guard")["config"]["rules"][0]["field"] = "missing".into();
                    "payload.bundle_payloads.field.missing"
                }
            };
            let error = run_workflow_graph(serde_json::from_value(bad).unwrap()).unwrap_err();
            assert!(error.contains(stage) && error.contains(path), "{error}");
            let result =
                run_workflow_graph(serde_json::from_value(request(hotspots)).unwrap()).unwrap();
            assert!(result.failed_nodes.is_empty());
            assert!(result.artifacts.contains_key("output.summary"));
        }
    }
}
