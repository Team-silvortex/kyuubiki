use kyuubiki_protocol::{DurableAgentTask, RpcMethod};
use serde_json::Value;

#[test]
fn agent_protocol_matches_standalone_sdk_digest_vectors() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../sdks/rust/fixtures/durable-execution-digests.json"
    ))
    .unwrap();
    let actual: Vec<_> = vectors
        .iter()
        .map(|case| {
            let method: RpcMethod = serde_json::from_value(case["method"].clone()).unwrap();
            (
                case["name"].as_str().unwrap(),
                DurableAgentTask::payload_digest(&method, &case["params"]).unwrap(),
            )
        })
        .collect();
    let expected: Vec<_> = vectors
        .iter()
        .map(|case| {
            (
                case["name"].as_str().unwrap(),
                case["digest"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(actual, expected);
}
