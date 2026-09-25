use kyuubiki_engine::run_workflow_graph;
use serde_json::{Value, json};

fn request() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-report-contract.json"
    ))
    .unwrap()
}

fn node<'a>(request: &'a mut Value, id: &str) -> &'a mut Value {
    request["graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn failure(case: usize) -> (Value, &'static str, &'static str) {
    let mut request = request();
    let (id, path) = match case {
        0 => {
            request["input_artifacts"]["input"]["nodes"][1]["v"] = Value::Null;
            ("field", "nodes[1].v")
        }
        1 => {
            request["input_artifacts"]["extra"]["converged"] = false.into();
            ("bundle", "payload.extra.converged")
        }
        2 => {
            request["input_artifacts"]["extra"]["diagnostic_node_count"] = (-1).into();
            ("bundle", "payload.extra.diagnostic_node_count")
        }
        3 => {
            node(&mut request, "guard")["config"]["rules"][0]["field"] = "missing".into();
            ("guard", "payload.bundle_payloads.field.missing")
        }
        4 => {
            node(&mut request, "guard")["config"]["rules"][0]["severity"] = "fatal".into();
            ("guard", "config.rules[0].severity")
        }
        _ => {
            node(&mut request, "bundle")["config"]["include_payloads"] = false.into();
            ("guard", "bundle_payloads")
        }
    };
    (request, id, path)
}

#[test]
fn shared_recovery_policies_isolate_only_explicitly_recoverable_failures() {
    let policies: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/workflow-recovery-policies.json"
    ))
    .unwrap();
    for policy in policies {
        for case in 0..6 {
            for reverse in [false, true] {
                let (mut request, id, path) = failure(case);
                node(&mut request, id)["config"]
                    .as_object_mut()
                    .unwrap()
                    .extend(policy["config"].as_object().unwrap().clone());
                if reverse {
                    request["graph"]["nodes"].as_array_mut().unwrap().reverse();
                }
                let run = run_workflow_graph(serde_json::from_value(request.clone()).unwrap());
                if policy["recover"] == false {
                    let error = run.expect_err("default and malformed policies must fail fast");
                    let path = policy["error_path"].as_str().unwrap_or(path);
                    assert!(
                        error.contains(id) && error.contains(path),
                        "{policy}: {error}"
                    );
                    continue;
                }
                let result = run.unwrap();
                assert_eq!(result.failed_nodes, [id]);
                assert_eq!(result.node_runs.len(), 9);
                assert!(result.skipped_nodes.iter().any(|n| n == "output"));
                assert!(!result.skipped_nodes.iter().any(|n| n == id));
                assert!(!result.completed_nodes.iter().any(|n| n == id));
                assert!(!result.artifacts.contains_key("output.summary"));
                assert!(!result.artifacts.contains_key(&format!("{id}.summary")));
                assert_eq!(
                    result.artifacts["raw.payload"],
                    request["input_artifacts"]["input"]
                );
                let trace = result
                    .node_runs
                    .iter()
                    .find(|trace| trace.node_id == id)
                    .unwrap();
                assert!(trace.error_message.as_deref().unwrap().contains(path));
                assert!(trace.produced_artifacts.is_empty());
                assert!(
                    !result
                        .artifact_lineage
                        .iter()
                        .any(|entry| entry.node_id == id)
                );
            }
        }
    }
}

#[test]
fn shared_fallback_selection_uses_declared_priority_after_dependencies_resolve() {
    for fail in [false, true] {
        for reverse in [false, true] {
            let request = fallback_request(fail, reverse);
            let result =
                run_workflow_graph(serde_json::from_value(request.clone()).unwrap()).unwrap();
            assert_eq!(
                result.artifacts["raw.payload"],
                request["input_artifacts"][if fail { "extra" } else { "input" }]
            );
            assert_eq!(
                result.failed_nodes,
                if fail { vec!["primary"] } else { vec![] }
            );
        }
    }
}

#[test]
fn shared_failed_input_and_exhausted_fallback_emit_no_invalid_output() {
    let mut bad = request();
    node(&mut bad, "input")["config"] = json!({"on_error": "skip"});
    bad["input_artifacts"]
        .as_object_mut()
        .unwrap()
        .remove("input");
    let result = run_workflow_graph(serde_json::from_value(bad).unwrap()).unwrap();
    assert_eq!(result.failed_nodes, ["input"]);
    assert_eq!(result.completed_nodes, ["extra"]);
    assert_eq!(result.skipped_nodes.len(), 7);
    assert_eq!(result.artifacts.len(), 1);

    let mut bad = fallback_request(true, false);
    bad["input_artifacts"]
        .as_object_mut()
        .unwrap()
        .remove("extra");
    node(&mut bad, "extra")["config"] = json!({"on_error": "skip"});
    let result = run_workflow_graph(serde_json::from_value(bad).unwrap()).unwrap();
    assert_eq!(result.failed_nodes.len(), 2);
    assert_eq!(result.skipped_nodes, ["fallback", "raw"]);
    assert!(!result.artifacts.contains_key("raw.payload"));

    let clean = run_workflow_graph(serde_json::from_value(request()).unwrap()).unwrap();
    assert!(clean.failed_nodes.is_empty());
    assert!(clean.artifacts.contains_key("output.summary"));
}

fn fallback_request(fail: bool, reverse: bool) -> Value {
    let mut request = request();
    let input = node(&mut request, "input").clone();
    let extra = node(&mut request, "extra").clone();
    let port = json!({"id": "payload", "artifact_type": "artifact/json"});
    request["graph"]["nodes"] = json!([
        input, extra,
        {"id": "fallback", "kind": "transform", "operator_id": "transform.first_available",
         "inputs": [{"id": "primary", "artifact_type": "artifact/json"}, {"id": "backup", "artifact_type": "artifact/json"}], "outputs": [port]},
        {"id": "raw", "kind": "output", "inputs": [port], "outputs": []},
        {"id": "primary", "kind": "condition", "inputs": [port], "outputs": [port],
         "config": {"on_error": "skip", "predicate": {"operator": if fail { "unsupported" } else { "truthy" }}}}
    ]);
    request["graph"]["output_nodes"] = json!(["raw"]);
    request["graph"]["edges"] = [
        ("primary", "fallback"),
        ("extra", "fallback"),
        ("input", "primary"),
        ("fallback", "raw"),
    ]
    .iter()
    .map(|(from, to)| {
        json!({
            "id": format!("{from}-{to}"), "artifact_type": "artifact/json",
        "from": {"node": from, "port": "payload"}, "to": {"node": to, "port": if *to == "fallback" { if *from == "primary" { "primary" } else { "backup" } } else { "payload" }}
        })
    })
    .collect();
    if reverse {
        request["graph"]["nodes"].as_array_mut().unwrap().reverse();
    }
    request
}
