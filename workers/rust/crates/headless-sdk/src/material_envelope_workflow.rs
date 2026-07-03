use serde_json::{Value, json};

pub(crate) fn material_study_envelope_graph_payload() -> Value {
    json!({
        "schema_version": "kyuubiki.workflow-graph/v1",
        "id": "workflow.material-study-envelope",
        "name": "Material study envelope ranking",
        "version": "1.0.0",
        "description": "Compose material study envelopes, rank candidates, and extract Pareto frontier.",
        "entry_nodes": ["material_rows"],
        "output_nodes": ["ranking_output", "pareto_output"],
        "defaults": {},
        "nodes": [
            input_node("material_rows", "rows"),
            transform_node(
                "compose_envelopes",
                "transform.compose_material_study_envelope",
                ["rows"],
                ["envelopes"],
                Value::Null
            ),
            transform_node(
                "rank_envelopes",
                "transform.rank_material_candidates",
                ["envelopes"],
                ["ranking"],
                json!({
                    "margin_prefix": "material_envelope",
                    "include_best_summary": false
                })
            ),
            transform_node(
                "pareto_envelopes",
                "transform.extract_material_pareto_frontier",
                ["envelopes"],
                ["pareto"],
                json!({
                    "feasible_field": "material_envelope_status",
                    "objectives": [
                        { "field": "material_envelope_score", "goal": "min" },
                        { "field": "material_envelope_safety_factor", "goal": "max" }
                    ]
                })
            ),
            output_node("ranking_output", "ranking"),
            output_node("pareto_output", "pareto")
        ],
        "edges": [
            edge("edge-rows-envelope", "material_rows", "rows", "compose_envelopes", "rows"),
            edge("edge-envelope-rank", "compose_envelopes", "envelopes", "rank_envelopes", "envelopes"),
            edge("edge-envelope-pareto", "compose_envelopes", "envelopes", "pareto_envelopes", "envelopes"),
            edge("edge-rank-output", "rank_envelopes", "ranking", "ranking_output", "ranking"),
            edge("edge-pareto-output", "pareto_envelopes", "pareto", "pareto_output", "pareto")
        ]
    })
}

pub(crate) fn material_study_envelope_input_artifacts() -> Value {
    json!({
        "material_rows": {
            "rows": [
                {
                    "case_id": "cool_stiff",
                    "summaries": {
                        "thermal": { "max_temperature": 82.0 },
                        "structural": { "max_stress": 100.0 }
                    }
                },
                {
                    "case_id": "warm_safe",
                    "summaries": {
                        "thermal": { "max_temperature": 90.0 },
                        "structural": { "max_stress": 120.0 }
                    }
                },
                {
                    "case_id": "hot_light",
                    "summaries": {
                        "thermal": { "max_temperature": 140.0 },
                        "structural": { "max_stress": 110.0 }
                    }
                }
            ]
        }
    })
}

fn input_node(id: &str, output: &str) -> Value {
    json!({
        "id": id,
        "kind": "input",
        "inputs": [],
        "outputs": [port(output)]
    })
}

fn transform_node<const IN: usize, const OUT: usize>(
    id: &str,
    operator_id: &str,
    inputs: [&str; IN],
    outputs: [&str; OUT],
    config: Value,
) -> Value {
    let mut node = json!({
        "id": id,
        "kind": "transform",
        "operator_id": operator_id,
        "inputs": inputs.into_iter().map(port).collect::<Vec<_>>(),
        "outputs": outputs.into_iter().map(port).collect::<Vec<_>>()
    });
    if !config.is_null() {
        node["config"] = config;
    }
    node
}

fn output_node(id: &str, input: &str) -> Value {
    json!({
        "id": id,
        "kind": "output",
        "inputs": [port(input)],
        "outputs": []
    })
}

fn port(id: &str) -> Value {
    json!({
        "id": id,
        "artifact_type": "artifact/result_summary",
        "dataset_value": "material_study"
    })
}

fn edge(id: &str, from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> Value {
    json!({
        "id": id,
        "from": { "node": from_node, "port": from_port },
        "to": { "node": to_node, "port": to_port },
        "artifact_type": "artifact/result_summary",
        "dataset_value": "material_study"
    })
}
