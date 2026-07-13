use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn coupled_readiness_ranking_drives_next_round_sweep_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: coupled_readiness_roundtrip_graph(),
        input_artifacts: BTreeMap::from([(
            "ranking".to_string(),
            json!({
                "quality_candidate_ranking_contract": "kyuubiki.quality_candidate_ranking/v1",
                "ranking": [{
                    "rank": 1,
                    "candidate_id": "candidate_coupled",
                    "score": 1.2,
                    "ready": true,
                    "metadata": {"round": "seed"},
                    "objective": {
                        "composite_quality_contract": "kyuubiki.composite_quality_objective/v1",
                        "composite_quality_score": 1.2,
                        "composite_quality_ready": true,
                        "composite_quality_dominant_term": {
                            "domain": "thermal",
                            "source": "thermal_quality"
                        },
                        "composite_quality_blocking_terms": []
                    },
                    "coupled_readiness": {
                        "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
                        "coupled_readiness_state": "block",
                        "coupled_readiness_recommendation": "hold_and_repair_inputs",
                        "coupled_readiness_blocking_domains": ["electrostatic"],
                        "coupled_readiness_required_missing": ["thermo"],
                        "coupled_readiness_warning_domains": []
                    }
                }]
            }),
        )]),
    })
    .expect("coupled readiness next-round graph should run");

    let request = run
        .artifacts
        .get("next_round.request")
        .expect("next-round request should exist");
    assert_eq!(request["action"].as_str(), Some("replan"));
    assert_eq!(
        request["selected_iteration_hint"]["action"].as_str(),
        Some("fix_coupled_readiness")
    );

    let plan = run
        .artifacts
        .get("build_plan.plan")
        .expect("sweep plan should exist");
    assert_eq!(
        plan["repair_strategy"].as_str(),
        Some("repair_coupled_readiness_sweep")
    );
    assert_eq!(
        plan["focused_axis_path"].as_str(),
        Some("electrostatic.voltage")
    );

    let cases = run
        .artifacts
        .get("expand_cases.cases")
        .expect("expanded cases should exist");
    assert_eq!(cases["case_count"].as_u64(), Some(2));
    assert_eq!(
        cases["cases"][0]["metadata"]["coupled_readiness"]["coupled_readiness_state"].as_str(),
        Some("block")
    );

    let report = run
        .artifacts
        .get("lineage.report")
        .expect("lineage report should exist");
    assert_eq!(report["lineage_complete"].as_bool(), Some(true));
    assert_eq!(
        report["repair_plan"]["repair_action"].as_str(),
        Some("fix_coupled_readiness")
    );
    assert_eq!(
        report["repair_plan"]["coupled_required_missing"][0].as_str(),
        Some("thermo")
    );
}

fn coupled_readiness_roundtrip_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.coupled-readiness-roundtrip".to_string(),
        name: "Coupled readiness roundtrip".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["ranking".to_string()],
        output_nodes: vec!["lineage_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            node(
                "ranking",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![port("ranking")],
                None,
            ),
            node(
                "next_round",
                WorkflowNodeKind::Transform,
                Some("transform.prepare_quality_next_round_request"),
                vec![port("ranking")],
                vec![port("request")],
                Some(json!({
                    "target_score": 2.0,
                    "max_candidates": 4,
                    "search_space": {
                        "thermal.temperature": [290.0, 310.0],
                        "electrostatic.voltage": [3.0, 5.0],
                        "material.density": [2700.0, 7800.0]
                    }
                })),
            ),
            node(
                "build_plan",
                WorkflowNodeKind::Transform,
                Some("transform.build_quality_parameter_sweep_plan"),
                vec![port("request")],
                vec![port("plan")],
                Some(json!({
                    "max_axes": 1,
                    "samples_per_axis": 2,
                    "id_prefix": "coupled_repair",
                    "base": {"electrostatic": {"voltage": 3.0}}
                })),
            ),
            node(
                "materialize",
                WorkflowNodeKind::Transform,
                Some("transform.materialize_quality_sweep_expansion"),
                vec![port("plan")],
                vec![port("expansion")],
                None,
            ),
            node(
                "expand_cases",
                WorkflowNodeKind::Transform,
                Some("transform.expand_parameter_sweep"),
                vec![port("expansion")],
                vec![port("cases")],
                None,
            ),
            node(
                "lineage",
                WorkflowNodeKind::Transform,
                Some("transform.compose_quality_lineage_report"),
                vec![port("request"), port("plan"), port("cases")],
                vec![port("report")],
                None,
            ),
            node(
                "lineage_output",
                WorkflowNodeKind::Output,
                None,
                vec![port("report")],
                vec![],
                None,
            ),
        ],
        edges: vec![
            edge(
                "ranking-next",
                "ranking",
                "ranking",
                "next_round",
                "ranking",
            ),
            edge(
                "next-plan",
                "next_round",
                "request",
                "build_plan",
                "request",
            ),
            edge(
                "plan-materialize",
                "build_plan",
                "plan",
                "materialize",
                "plan",
            ),
            edge(
                "materialize-expand",
                "materialize",
                "expansion",
                "expand_cases",
                "expansion",
            ),
            edge(
                "request-lineage",
                "next_round",
                "request",
                "lineage",
                "request",
            ),
            edge("plan-lineage", "build_plan", "plan", "lineage", "plan"),
            edge("cases-lineage", "expand_cases", "cases", "lineage", "cases"),
            edge(
                "lineage-output",
                "lineage",
                "report",
                "lineage_output",
                "report",
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

fn port(id: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: "artifact/result_summary".to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: Some("coupled_readiness_roundtrip".to_string()),
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
        dataset_value: Some("coupled_readiness_roundtrip".to_string()),
    }
}
