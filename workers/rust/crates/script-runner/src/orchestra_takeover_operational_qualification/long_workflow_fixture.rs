use serde_json::{Value, json};

pub(super) fn workflow_probe(case_id: &str, retry_safety: &str) -> Value {
    json!({
        "graph": {
            "schema_version": "kyuubiki.workflow-graph/v1",
            "id": format!("workflow.long-running-takeover-{case_id}"),
            "name": format!("Long-running takeover {case_id}"),
            "version": "1.0.0",
            "entry_nodes": ["bar_model"],
            "output_nodes": ["bar_output"],
            "defaults": {"orchestrated": true},
            "recovery_policy": {"retry_safety": retry_safety},
            "nodes": [
                {
                    "id": "bar_model",
                    "kind": "input",
                    "outputs": [{"id": "model", "artifact_type": "model/bar_1d"}]
                },
                {
                    "id": "bar_solve",
                    "kind": "solve",
                    "operator_id": "solve.bar_1d",
                    "retry_safety": retry_safety,
                    "inputs": [{"id": "model", "artifact_type": "model/bar_1d"}],
                    "outputs": [{"id": "result", "artifact_type": "result/bar_1d"}]
                },
                {
                    "id": "bar_output",
                    "kind": "output",
                    "inputs": [{"id": "result", "artifact_type": "result/bar_1d"}],
                    "outputs": []
                }
            ],
            "edges": [
                {
                    "id": "edge.model.solve",
                    "from": {"node": "bar_model", "port": "model"},
                    "to": {"node": "bar_solve", "port": "model"},
                    "artifact_type": "model/bar_1d"
                },
                {
                    "id": "edge.solve.output",
                    "from": {"node": "bar_solve", "port": "result"},
                    "to": {"node": "bar_output", "port": "result"},
                    "artifact_type": "result/bar_1d"
                }
            ]
        },
        "input_artifacts": {
            "bar_model": {
                "length": 1.0,
                "area": 2.0,
                "youngs_modulus": 1000.0,
                "elements": 2,
                "tip_force": 20.0
            }
        },
        "response_options": {
            "response_mode": "full",
            "include_artifacts": true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_exposes_explicit_replay_policy_and_real_solver() {
        let fixture = workflow_probe("idempotent", "idempotent");
        assert_eq!(
            fixture
                .pointer("/graph/recovery_policy/retry_safety")
                .and_then(Value::as_str),
            Some("idempotent")
        );
        assert_eq!(
            fixture
                .pointer("/graph/nodes/1/operator_id")
                .and_then(Value::as_str),
            Some("solve.bar_1d")
        );
        assert_eq!(
            fixture
                .pointer("/input_artifacts/bar_model/elements")
                .and_then(Value::as_u64),
            Some(2)
        );
    }
}
