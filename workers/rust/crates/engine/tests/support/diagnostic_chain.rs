use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

pub fn request(
    domain: &str,
    quality_operator: &str,
    quality_config: Value,
    input: Value,
    recover: bool,
) -> WorkflowGraphRunRequest {
    let artifact = "artifact/result_summary";
    let port = |id| json!({"id":id,"artifact_type":artifact});
    let edge = |from: &str, output: &str, to: &str, input: &str| {
        json!({"id":format!("{from}-{to}"),"artifact_type":artifact,
            "from":{"node":from,"port":output},"to":{"node":to,"port":input}})
    };
    serde_json::from_value(json!({
        "graph":{
            "schema_version":"kyuubiki.workflow-graph/v1","id":"domain-diagnostic-integrity",
            "name":"Complete domain diagnostics","version":"1.0.0",
            "entry_nodes":["input","independent"],"output_nodes":["raw","decision","independent_output"],
            "nodes":[
                {"id":"input","kind":"input","inputs":[],"outputs":[port("payload")]},
                {"id":"diagnose","kind":"extract","operator_id":format!("extract.{domain}_result_diagnostics"),
                 "config":if recover {json!({"on_error":"skip"})} else {json!({})},
                 "inputs":[port("payload")],"outputs":[port("summary")]},
                {"id":"quality","kind":"transform","operator_id":quality_operator,
                 "config":quality_config,"inputs":[port("payload")],"outputs":[port("summary")]},
                {"id":"objective","kind":"transform","operator_id":"transform.compose_quality_objective",
                 "inputs":[port("quality")],"outputs":[port("summary")]},
                {"id":"decision","kind":"output","inputs":[port("summary")],"outputs":[]},
                {"id":"raw","kind":"output","inputs":[port("payload")],"outputs":[]},
                {"id":"independent","kind":"input","inputs":[],"outputs":[port("value")]},
                {"id":"independent_output","kind":"output","inputs":[port("value")],"outputs":[]}
            ],
            "edges":[edge("input","payload","diagnose","payload"),edge("input","payload","raw","payload"),
                     edge("diagnose","summary","quality","payload"),edge("quality","summary","objective","quality"),
                     edge("objective","summary","decision","summary"),edge("independent","value","independent_output","value")]
        },
        "input_artifacts":{"input":input,"independent":{"value":7}}
    })).unwrap()
}

pub fn with_solver(request: WorkflowGraphRunRequest, operator: &str) -> WorkflowGraphRunRequest {
    let mut request = serde_json::to_value(request).unwrap();
    let artifact = "artifact/result_summary";
    let port = |id| json!({"id":id,"artifact_type":artifact});
    request["graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id":"solve","kind":"solve","operator_id":operator,
            "inputs":[port("model")],"outputs":[port("payload")]
        }));
    let edges = request["graph"]["edges"].as_array_mut().unwrap();
    for edge in edges
        .iter_mut()
        .filter(|edge| edge["from"]["node"] == "input")
    {
        edge["from"]["node"] = json!("solve");
    }
    edges.push(json!({"id":"input-solve","artifact_type":artifact,
        "from":{"node":"input","port":"payload"},"to":{"node":"solve","port":"model"}}));
    serde_json::from_value(request).unwrap()
}
