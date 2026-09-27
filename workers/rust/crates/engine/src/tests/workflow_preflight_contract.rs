use crate::workflow_security::validate_workflow_security;
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-graph-preflight.json"
    ))
    .unwrap()
}

fn set_path(value: &mut Value, path: &[Value], replacement: Value, append: bool) {
    if let Some((key, rest)) = path.split_first() {
        let child = match key.as_str() {
            Some(key) => &mut value[key],
            None => &mut value[key.as_u64().unwrap() as usize],
        };
        set_path(child, rest, replacement, append);
    } else if append {
        value.as_array_mut().unwrap().push(replacement);
    } else {
        *value = replacement;
    }
}

fn validate(request: Value) -> Result<(), String> {
    let request: WorkflowGraphRunRequest =
        serde_json::from_value(request).map_err(|error| error.to_string())?;
    validate_workflow_security(&request)
}

#[test]
fn shared_graph_preflight_structure() {
    let fixture = fixture();
    for case in fixture["cases"].as_array().unwrap() {
        let mut request = fixture["request"].clone();
        for change in case["changes"].as_array().unwrap() {
            set_path(
                &mut request,
                change["path"].as_array().unwrap(),
                change["value"].clone(),
                change["op"] == "append",
            );
        }
        for reverse in [false, true] {
            if reverse {
                if let Some(nodes) = request["graph"]["nodes"].as_array_mut() {
                    nodes.reverse();
                }
            }
            let result = validate(request.clone());
            assert_eq!(
                result.is_ok(),
                case["accept"] == true,
                "{}: {result:?}",
                case["id"]
            );
        }
    }
}

#[test]
fn shared_graph_preflight_budgets() {
    let fixture = fixture();
    for budget in fixture["budgets"].as_array().unwrap() {
        let mut request = fixture["request"].clone();
        let size = budget["size"].as_u64().unwrap() as usize;
        let kind = budget["kind"].as_str().unwrap();
        if let Some(path) = budget["path"].as_array() {
            let value = match kind {
                "array" => json!(vec![0; size]),
                "nested" => (0..size).fold(json!(0), |inner, _| json!([inner])),
                "string" => json!("x".repeat(size)),
                "key" => json!({"x".repeat(size): 0}),
                _ => panic!("unknown fixture kind {kind}"),
            };
            set_path(&mut request, path, value, false);
        } else {
            topology_budget(&mut request, kind, size);
        }
        let result = validate(request);
        assert_eq!(
            result.is_ok(),
            budget["accept"] == true,
            "{}: {result:?}",
            budget["id"]
        );
    }
}

fn topology_budget(request: &mut Value, kind: &str, size: usize) {
    match kind {
        "nodes" => {
            for i in 4..=size {
                request["graph"]["nodes"].as_array_mut().unwrap().push(
                    json!({"id": format!("n{i}"), "kind": "output", "inputs": [], "outputs": []}),
                );
            }
        }
        "inputs" | "outputs" => {
            for i in 2..=size {
                request["graph"]["nodes"][1][kind]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"id": format!("p{i}"), "artifact_type": "artifact/json"}));
            }
        }
        "edges" => {
            let mut nodes = vec![request["graph"]["nodes"][0].clone()];
            let ports: Vec<_> = (0..32)
                .map(|i| json!({"id": format!("p{i}"), "artifact_type": "artifact/json"}))
                .collect();
            for i in 0..=(size - 1) / 32 {
                nodes.push(json!({"id": format!("n{i}"), "kind": "output", "inputs": ports, "outputs": []}));
            }
            let edges: Vec<_> = (0..size)
                .map(|i| {
                    json!({"id": format!("e{i}"),
                "from": {"node": "input", "port": "payload"},
                "to": {"node": format!("n{}", i / 32), "port": format!("p{}", i % 32)},
                "artifact_type": "artifact/json"})
                })
                .collect();
            request["graph"]["nodes"] = json!(nodes);
            request["graph"]["edges"] = json!(edges);
            request["graph"]["output_nodes"] = json!([]);
        }
        _ => panic!("unknown topology fixture {kind}"),
    }
}
