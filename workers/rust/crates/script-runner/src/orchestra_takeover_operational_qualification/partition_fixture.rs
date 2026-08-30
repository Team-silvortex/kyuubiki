use serde_json::{Value, json};

pub(super) fn stale_owner_write_probe() -> Value {
    json!({
        "graph": {
            "schema_version": "kyuubiki.workflow-graph/v1",
            "id": "workflow.partition-fencing-probe",
            "entry_nodes": ["input_node"],
            "output_nodes": ["output_node"],
            "nodes": [
                {
                    "id": "input_node",
                    "kind": "input",
                    "outputs": [{"id": "value", "artifact_type": "export/json"}]
                },
                {
                    "id": "output_node",
                    "kind": "output",
                    "inputs": [{"id": "value", "artifact_type": "export/json"}],
                    "outputs": []
                }
            ],
            "edges": [{
                "id": "e0",
                "from": {"node": "input_node", "port": "value"},
                "to": {"node": "output_node", "port": "value"},
                "artifact_type": "export/json"
            }]
        },
        "input_artifacts": {"input_node": {"partition_probe": true}}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_is_a_valid_compact_input_output_graph_shape() {
        let probe = stale_owner_write_probe();
        assert_eq!(
            probe.pointer("/graph/nodes/0/kind").and_then(Value::as_str),
            Some("input")
        );
        assert_eq!(
            probe.pointer("/graph/nodes/1/kind").and_then(Value::as_str),
            Some("output")
        );
        assert_eq!(
            probe.pointer("/graph/edges/0/id").and_then(Value::as_str),
            Some("e0")
        );
    }
}
