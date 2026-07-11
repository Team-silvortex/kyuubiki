use crate::{
    run_workflow_graph, workflow_executor::run_transform_operator,
    workflow_quality_sweep_plan::materialize_quality_sweep_expansion,
    workflow_quality_sweep_request::build_quality_parameter_sweep_plan,
};
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn builds_quality_parameter_sweep_plan_from_next_round_request() {
    let plan = build_quality_parameter_sweep_plan(
        serde_json::json!({
            "quality_next_round_contract": "kyuubiki.quality_next_round_request/v1",
            "action": "continue",
            "selected_candidate_id": "candidate_b",
            "target_score": 2.0,
            "request_payload": {
                "max_candidates": 12,
                "seed_metadata": {
                    "source_candidate_id": "seed_candidate",
                    "round": "previous"
                },
                "search_space": {
                    "elements.0.thickness": {"min": 0.01, "max": 0.03},
                    "material.density": [2700.0, 7800.0]
                }
            }
        }),
        serde_json::json!({
            "samples_per_axis": 3,
            "id_prefix": "quality_candidate",
            "base": {"elements": [{"thickness": 0.02}]}
        }),
    )
    .expect("quality parameter sweep plan should build");

    assert_eq!(
        plan["quality_parameter_sweep_plan_contract"].as_str(),
        Some("kyuubiki.quality_parameter_sweep_plan/v1")
    );
    assert_eq!(plan["sweep_enabled"].as_bool(), Some(true));
    assert_eq!(plan["source_candidate_id"].as_str(), Some("candidate_b"));
    assert_eq!(
        plan["seed_metadata"]["source_candidate_id"].as_str(),
        Some("seed_candidate")
    );
    assert_eq!(plan["id_prefix"].as_str(), Some("quality_candidate"));
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(6));
    assert_eq!(plan["axes"].as_array().map(Vec::len), Some(2));
}

#[test]
fn runs_quality_parameter_sweep_plan_through_transform_executor() {
    let plan = run_transform_operator(
        "transform.build_quality_parameter_sweep_plan",
        serde_json::json!({
            "action": "continue",
            "selected_candidate_id": "candidate_a",
            "request_payload": {
                "search_space": {
                    "model.thickness": {"values": [0.01, 0.02]}
                }
            }
        }),
        serde_json::json!({"base": {"model": {"thickness": 0.01}}}),
    )
    .expect("quality parameter sweep plan should run through executor");

    assert_eq!(plan["sweep_enabled"].as_bool(), Some(true));
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(2));
    assert_eq!(plan["axes"][0]["path"].as_str(), Some("model.thickness"));
}

#[test]
fn prioritizes_quality_sweep_axes_from_optimization_hint() {
    let plan = build_quality_parameter_sweep_plan(
        serde_json::json!({
            "action": "continue",
            "selected_candidate_id": "candidate_hint",
            "request_payload": {
                "optimization_hint": {
                    "action": "reduce_dominant_term",
                    "focus_domain": "structural",
                    "focus_field": "stiffness_margin"
                },
                "search_space": {
                    "material.density": [2700.0, 7800.0],
                    "model.stiffness_margin": {"values": [1.1, 1.4]},
                    "elements.0.thickness": [0.01, 0.02]
                }
            }
        }),
        serde_json::json!({
            "samples_per_axis": 2,
            "max_axes": 1
        }),
    )
    .expect("hint-focused quality sweep plan should build");

    assert_eq!(plan["axes"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        plan["axes"][0]["path"].as_str(),
        Some("model.stiffness_margin")
    );
    assert_eq!(
        plan["focused_axis_path"].as_str(),
        Some("model.stiffness_margin")
    );
    assert_eq!(
        plan["optimization_hint"]["focus_field"].as_str(),
        Some("stiffness_margin")
    );
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(2));
}

#[test]
fn materializes_quality_sweep_expansion_payload() {
    let expansion = materialize_quality_sweep_expansion(
        serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": true,
            "source_candidate_id": "candidate_b",
            "id_prefix": "quality_candidate",
            "max_cases": 12,
            "case_count_estimate": 2,
            "optimization_hint": {
                "action": "reduce_dominant_term",
                "focus_field": "model.thickness"
            },
            "focused_axis_path": "model.thickness",
            "seed_metadata": {
                "source_candidate_id": "candidate_a",
                "round": "seed"
            },
            "base": {"model": {"thickness": 0.01}},
            "axes": [{
                "label": "thickness",
                "path": "model.thickness",
                "values": [0.01, 0.02]
            }]
        }),
        serde_json::json!({}),
    )
    .expect("quality sweep expansion should materialize");

    assert_eq!(
        expansion["quality_sweep_expansion_contract"].as_str(),
        Some("kyuubiki.quality_sweep_expansion/v1")
    );
    assert_eq!(expansion["expansion_enabled"].as_bool(), Some(true));
    assert_eq!(
        expansion["payload"]["axes"][0]["path"].as_str(),
        Some("model.thickness")
    );
    assert_eq!(
        expansion["payload"]["case_metadata"]["optimization_hint"]["focus_field"].as_str(),
        Some("model.thickness")
    );
    assert_eq!(
        expansion["payload"]["case_metadata"]["focused_axis_path"].as_str(),
        Some("model.thickness")
    );
    assert_eq!(
        expansion["payload"]["case_metadata"]["seed_metadata"]["round"].as_str(),
        Some("seed")
    );
    assert_eq!(
        expansion["config"]["id_prefix"].as_str(),
        Some("quality_candidate")
    );
    approx_eq(expansion["config"]["max_cases"].as_f64(), 12.0);
}

#[test]
fn runs_quality_sweep_expansion_through_transform_executor() {
    let expansion = run_transform_operator(
        "transform.materialize_quality_sweep_expansion",
        serde_json::json!({
            "sweep_enabled": true,
            "base": {"model": {"thickness": 0.01}},
            "axes": [{"path": "model.thickness", "values": [0.01, 0.02]}]
        }),
        serde_json::json!({"id_prefix": "q"}),
    )
    .expect("quality sweep expansion should run through executor");

    assert_eq!(expansion["expansion_enabled"].as_bool(), Some(true));
    assert_eq!(expansion["config"]["id_prefix"].as_str(), Some("q"));
}

#[test]
fn expands_materialized_quality_sweep_through_parameter_sweep_operator() {
    let expansion = run_transform_operator(
        "transform.materialize_quality_sweep_expansion",
        serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": true,
            "id_prefix": "quality_candidate",
            "max_cases": 4,
            "optimization_hint": {
                "action": "reduce_dominant_term",
                "focus_field": "elements.0.thickness"
            },
            "focused_axis_path": "elements.0.thickness",
            "seed_metadata": {
                "source_candidate_id": "candidate_a",
                "focused_axis_path": "previous.axis"
            },
            "base": {
                "elements": [{"thickness": 0.01}],
                "material": {"density": 2700.0}
            },
            "axes": [
                {"path": "elements.0.thickness", "values": [0.01, 0.02]},
                {"path": "material.density", "values": [2700.0, 7800.0]}
            ]
        }),
        serde_json::json!({}),
    )
    .expect("quality sweep expansion should materialize");

    let expanded = run_transform_operator(
        "transform.expand_parameter_sweep",
        expansion,
        serde_json::json!({}),
    )
    .expect("materialized quality sweep should expand directly");

    assert_eq!(expanded["case_count"].as_u64(), Some(4));
    assert_eq!(expanded["axis_count"].as_u64(), Some(2));
    assert_eq!(
        expanded["cases"][0]["id"].as_str(),
        Some("quality_candidate_0")
    );
    assert_eq!(
        expanded["cases"][3]["model"]["material"]["density"].as_f64(),
        Some(7800.0)
    );
    assert_eq!(
        expanded["cases"][0]["metadata"]["optimization_hint"]["focus_field"].as_str(),
        Some("elements.0.thickness")
    );
    assert_eq!(
        expanded["cases"][0]["metadata"]["focused_axis_path"].as_str(),
        Some("elements.0.thickness")
    );
    assert_eq!(
        expanded["cases"][0]["metadata"]["seed_metadata"]["source_candidate_id"].as_str(),
        Some("candidate_a")
    );
}

#[test]
fn composes_quality_lineage_report_from_request_plan_and_cases() {
    let report = run_transform_operator(
        "transform.compose_quality_lineage_report",
        serde_json::json!({
            "request": {
                "quality_next_round_contract": "kyuubiki.quality_next_round_request/v1",
                "selected_candidate_id": "candidate_b",
                "selected_candidate_ready": true,
                "request_payload": {
                    "seed_metadata": {
                        "source_candidate_id": "candidate_a",
                        "round": "previous"
                    },
                    "optimization_hint": {
                        "action": "reduce_dominant_term",
                        "focus_field": "elements.0.thickness"
                    }
                }
            },
            "plan": {
                "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
                "source_candidate_id": "candidate_b",
                "focused_axis_path": "elements.0.thickness",
                "case_count_estimate": 2
            },
            "cases": {
                "case_count": 2,
                "cases": [{
                    "id": "quality_candidate_0",
                    "metadata": {
                        "source_candidate_id": "candidate_b",
                        "focused_axis_path": "elements.0.thickness"
                    }
                }]
            }
        }),
        serde_json::json!({}),
    )
    .expect("quality lineage report should compose");

    assert_eq!(
        report["quality_lineage_report_contract"].as_str(),
        Some("kyuubiki.quality_lineage_report/v1")
    );
    assert_eq!(report["lineage_complete"].as_bool(), Some(true));
    assert_eq!(
        report["selected_candidate_id"].as_str(),
        Some("candidate_b")
    );
    assert_eq!(
        report["seed_metadata"]["source_candidate_id"].as_str(),
        Some("candidate_a")
    );
    assert_eq!(
        report["optimization_hint"]["focus_field"].as_str(),
        Some("elements.0.thickness")
    );
    assert_eq!(
        report["first_case_metadata"]["source_candidate_id"].as_str(),
        Some("candidate_b")
    );
}

#[test]
fn runs_quality_next_round_to_parameter_sweep_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: quality_sweep_graph(),
        input_artifacts: BTreeMap::from([(
            "next_round".to_string(),
            serde_json::json!({
                "quality_next_round_contract": "kyuubiki.quality_next_round_request/v1",
                "action": "continue",
                "selected_candidate_id": "candidate_b",
                "target_score": 2.0,
                "request_payload": {
                    "max_candidates": 4,
                    "search_space": {
                        "elements.0.thickness": [0.01, 0.02],
                        "material.density": [2700.0, 7800.0]
                    }
                }
            }),
        )]),
    })
    .expect("quality sweep workflow should run");

    let plan = run
        .artifacts
        .get("build_plan.plan")
        .expect("sweep plan should exist");
    assert_eq!(
        plan["quality_parameter_sweep_plan_contract"].as_str(),
        Some("kyuubiki.quality_parameter_sweep_plan/v1")
    );
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(4));

    let expanded = run
        .artifacts
        .get("expand_cases.cases")
        .expect("expanded cases should exist");
    assert_eq!(expanded["case_count"].as_u64(), Some(4));
    assert_eq!(
        expanded["cases"][0]["id"].as_str(),
        Some("quality_candidate_0")
    );
    assert_eq!(
        expanded["cases"][3]["model"]["material"]["density"].as_f64(),
        Some(7800.0)
    );
}

fn quality_sweep_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.quality-next-round-to-sweep".to_string(),
        name: "Quality next round to sweep".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["next_round".to_string()],
        output_nodes: vec!["sweep_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            node(
                "next_round",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![port("request", "artifact/result_summary")],
                None,
            ),
            node(
                "build_plan",
                WorkflowNodeKind::Transform,
                Some("transform.build_quality_parameter_sweep_plan"),
                vec![port("request", "artifact/result_summary")],
                vec![port("plan", "artifact/result_summary")],
                Some(serde_json::json!({
                    "samples_per_axis": 2,
                    "id_prefix": "quality_candidate",
                    "base": {
                        "elements": [{"thickness": 0.01}],
                        "material": {"density": 2700.0}
                    }
                })),
            ),
            node(
                "materialize",
                WorkflowNodeKind::Transform,
                Some("transform.materialize_quality_sweep_expansion"),
                vec![port("plan", "artifact/result_summary")],
                vec![port("expansion", "artifact/result_summary")],
                None,
            ),
            node(
                "expand_cases",
                WorkflowNodeKind::Transform,
                Some("transform.expand_parameter_sweep"),
                vec![port("expansion", "artifact/result_summary")],
                vec![port("cases", "artifact/result_summary")],
                None,
            ),
            node(
                "sweep_output",
                WorkflowNodeKind::Output,
                None,
                vec![port("cases", "artifact/result_summary")],
                vec![],
                None,
            ),
        ],
        edges: vec![
            edge(
                "edge-request-plan",
                "next_round",
                "request",
                "build_plan",
                "request",
            ),
            edge(
                "edge-plan-materialize",
                "build_plan",
                "plan",
                "materialize",
                "plan",
            ),
            edge(
                "edge-materialize-expand",
                "materialize",
                "expansion",
                "expand_cases",
                "expansion",
            ),
            edge(
                "edge-expand-output",
                "expand_cases",
                "cases",
                "sweep_output",
                "cases",
            ),
        ],
    }
}

fn node(
    id: &str,
    kind: WorkflowNodeKind,
    operator_id: Option<&str>,
    inputs: Vec<WorkflowPort>,
    outputs: Vec<WorkflowPort>,
    config: Option<serde_json::Value>,
) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind,
        operator_id: operator_id.map(str::to_string),
        name: None,
        description: None,
        config,
        cache_policy: None,
        inputs,
        outputs,
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: Some("quality_sweep".to_string()),
    }
}

fn edge(id: &str, from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: from_node.to_string(),
            port: from_port.to_string(),
        },
        to: WorkflowNodePortRef {
            node: to_node.to_string(),
            port: to_port.to_string(),
        },
        artifact_type: "artifact/result_summary".to_string(),
        dataset_value: Some("quality_sweep".to_string()),
    }
}
