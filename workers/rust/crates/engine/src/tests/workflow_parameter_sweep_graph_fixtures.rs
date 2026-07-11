use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowNode, WorkflowNodeKind,
    WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

pub(super) fn sweep_result_inputs() -> BTreeMap<String, serde_json::Value> {
    BTreeMap::from([
        (
            "sweep_cases".to_string(),
            serde_json::json!([
                {
                    "id": "material_panel_0",
                    "parameters": { "thickness": 0.01, "density": 2700.0 },
                    "metadata": {
                        "round": "seed",
                        "source_candidate_id": "baseline"
                    },
                    "model": { "tag": "thin-light" }
                },
                {
                    "id": "material_panel_1",
                    "parameters": { "thickness": 0.02, "density": 7800.0 },
                    "metadata": {
                        "round": "seed",
                        "source_candidate_id": "baseline"
                    },
                    "model": { "tag": "balanced-heavy" }
                }
            ]),
        ),
        (
            "agent_results".to_string(),
            serde_json::json!([
                {
                    "case_id": "material_panel_0",
                    "status": "ok",
                    "summary": {
                        "max_stress": 142.0,
                        "mass": 2.7
                    }
                },
                {
                    "case_id": "material_panel_1",
                    "status": "ok",
                    "summary": {
                        "max_stress": 84.0,
                        "mass": 7.8
                    }
                }
            ]),
        ),
    ])
}

pub(super) fn parameter_sweep_result_scoring_graph(target_score: f64) -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.parameter-sweep-result-scoring".to_string(),
        name: "Parameter sweep result scoring".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["sweep_cases".to_string(), "agent_results".to_string()],
        output_nodes: vec![
            "next_cases_output".to_string(),
            "lineage_output".to_string(),
        ],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            sweep_node(
                "sweep_cases",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![sweep_port("cases")],
                None,
            ),
            sweep_node(
                "agent_results",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![sweep_port("results")],
                None,
            ),
            sweep_node(
                "join_results",
                WorkflowNodeKind::Transform,
                Some("transform.join_parameter_sweep_results"),
                vec![sweep_port("cases"), sweep_port("results")],
                vec![sweep_port("joined")],
                Some(serde_json::json!({
                    "strict": true
                })),
            ),
            sweep_node(
                "summarize_results",
                WorkflowNodeKind::Transform,
                Some("transform.summarize_parameter_sweep"),
                vec![sweep_port("joined")],
                vec![sweep_port("summary")],
                Some(serde_json::json!({
                    "fields": ["max_stress", "mass"]
                })),
            ),
            sweep_node(
                "score_results",
                WorkflowNodeKind::Transform,
                Some("transform.score_parameter_sweep"),
                vec![sweep_port("summary")],
                vec![sweep_port("scored")],
                Some(serde_json::json!({
                    "objectives": [
                        {
                            "field": "mass",
                            "goal": "min",
                            "weight": 1.0
                        },
                        {
                            "field": "max_stress",
                            "goal": "min",
                            "weight": 0.02,
                            "max_allowed": 100.0
                        }
                    ]
                })),
            ),
            sweep_node(
                "map_quality_candidates",
                WorkflowNodeKind::Transform,
                Some("transform.map_parameter_sweep_scores_to_quality_candidates"),
                vec![sweep_port("scored")],
                vec![sweep_port("candidates")],
                Some(serde_json::json!({
                    "quality_domain": "material_sweep"
                })),
            ),
            sweep_node(
                "rank_quality_candidates",
                WorkflowNodeKind::Transform,
                Some("transform.rank_quality_candidates"),
                vec![sweep_port("candidates")],
                vec![sweep_port("ranking")],
                None,
            ),
            sweep_node(
                "prepare_next_round",
                WorkflowNodeKind::Transform,
                Some("transform.prepare_quality_next_round_request"),
                vec![sweep_port("ranking")],
                vec![sweep_port("request")],
                Some(serde_json::json!({
                    "target_score": target_score,
                    "max_candidates": 4,
                    "search_space": {
                        "elements.0.thickness": [0.01, 0.02],
                        "material.density": [2700.0, 7800.0]
                    }
                })),
            ),
            sweep_node(
                "build_next_plan",
                WorkflowNodeKind::Transform,
                Some("transform.build_quality_parameter_sweep_plan"),
                vec![sweep_port("request")],
                vec![sweep_port("plan")],
                Some(serde_json::json!({
                    "samples_per_axis": 2,
                    "id_prefix": "material_next_round",
                    "base": {
                        "elements": [{"thickness": 0.02}],
                        "material": {"density": 7800.0}
                    }
                })),
            ),
            sweep_node(
                "materialize_next_plan",
                WorkflowNodeKind::Transform,
                Some("transform.materialize_quality_sweep_expansion"),
                vec![sweep_port("plan")],
                vec![sweep_port("expansion")],
                None,
            ),
            sweep_node(
                "expand_next_cases",
                WorkflowNodeKind::Transform,
                Some("transform.expand_parameter_sweep"),
                vec![sweep_port("expansion")],
                vec![sweep_port("cases")],
                None,
            ),
            sweep_node(
                "compose_lineage",
                WorkflowNodeKind::Transform,
                Some("transform.compose_quality_lineage_report"),
                vec![
                    sweep_port("ranking"),
                    sweep_port("request"),
                    sweep_port("plan"),
                    sweep_port("cases"),
                ],
                vec![sweep_port("lineage")],
                None,
            ),
            sweep_node(
                "next_cases_output",
                WorkflowNodeKind::Output,
                None,
                vec![sweep_port("cases")],
                vec![],
                None,
            ),
            sweep_node(
                "lineage_output",
                WorkflowNodeKind::Output,
                None,
                vec![sweep_port("lineage")],
                vec![],
                None,
            ),
        ],
        edges: vec![
            sweep_edge(
                "edge-cases-join",
                "sweep_cases",
                "cases",
                "join_results",
                "cases",
            ),
            sweep_edge(
                "edge-results-join",
                "agent_results",
                "results",
                "join_results",
                "results",
            ),
            sweep_edge(
                "edge-join-summary",
                "join_results",
                "joined",
                "summarize_results",
                "joined",
            ),
            sweep_edge(
                "edge-summary-score",
                "summarize_results",
                "summary",
                "score_results",
                "summary",
            ),
            sweep_edge(
                "edge-score-map-quality",
                "score_results",
                "scored",
                "map_quality_candidates",
                "scored",
            ),
            sweep_edge(
                "edge-map-quality-rank",
                "map_quality_candidates",
                "candidates",
                "rank_quality_candidates",
                "candidates",
            ),
            sweep_edge(
                "edge-rank-next-round",
                "rank_quality_candidates",
                "ranking",
                "prepare_next_round",
                "ranking",
            ),
            sweep_edge(
                "edge-next-round-plan",
                "prepare_next_round",
                "request",
                "build_next_plan",
                "request",
            ),
            sweep_edge(
                "edge-plan-materialize",
                "build_next_plan",
                "plan",
                "materialize_next_plan",
                "plan",
            ),
            sweep_edge(
                "edge-materialize-expand",
                "materialize_next_plan",
                "expansion",
                "expand_next_cases",
                "expansion",
            ),
            sweep_edge(
                "edge-ranking-lineage",
                "rank_quality_candidates",
                "ranking",
                "compose_lineage",
                "ranking",
            ),
            sweep_edge(
                "edge-request-lineage",
                "prepare_next_round",
                "request",
                "compose_lineage",
                "request",
            ),
            sweep_edge(
                "edge-plan-lineage",
                "build_next_plan",
                "plan",
                "compose_lineage",
                "plan",
            ),
            sweep_edge(
                "edge-cases-lineage",
                "expand_next_cases",
                "cases",
                "compose_lineage",
                "cases",
            ),
            sweep_edge(
                "edge-expand-output",
                "expand_next_cases",
                "cases",
                "next_cases_output",
                "cases",
            ),
            sweep_edge(
                "edge-lineage-output",
                "compose_lineage",
                "lineage",
                "lineage_output",
                "lineage",
            ),
        ],
    }
}

pub(super) fn material_study_envelope_inputs() -> BTreeMap<String, serde_json::Value> {
    BTreeMap::from([(
        "material_rows".to_string(),
        serde_json::json!({
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
        }),
    )])
}

pub(super) fn material_study_envelope_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.material-study-envelope".to_string(),
        name: "Material study envelope ranking".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Compose material study envelopes, rank candidates, and extract Pareto frontier."
                .to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["material_rows".to_string()],
        output_nodes: vec!["ranking_output".to_string(), "pareto_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            sweep_node(
                "material_rows",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![sweep_port("rows")],
                None,
            ),
            sweep_node(
                "compose_envelopes",
                WorkflowNodeKind::Transform,
                Some("transform.compose_material_study_envelope"),
                vec![sweep_port("rows")],
                vec![sweep_port("envelopes")],
                None,
            ),
            sweep_node(
                "rank_envelopes",
                WorkflowNodeKind::Transform,
                Some("transform.rank_material_candidates"),
                vec![sweep_port("envelopes")],
                vec![sweep_port("ranking")],
                Some(serde_json::json!({
                    "margin_prefix": "material_envelope",
                    "include_best_summary": false
                })),
            ),
            sweep_node(
                "pareto_envelopes",
                WorkflowNodeKind::Transform,
                Some("transform.extract_material_pareto_frontier"),
                vec![sweep_port("envelopes")],
                vec![sweep_port("pareto")],
                Some(serde_json::json!({
                    "feasible_field": "material_envelope_status",
                    "objectives": [
                        { "field": "material_envelope_score", "goal": "min" },
                        { "field": "material_envelope_safety_factor", "goal": "max" }
                    ]
                })),
            ),
            sweep_node(
                "ranking_output",
                WorkflowNodeKind::Output,
                None,
                vec![sweep_port("ranking")],
                vec![],
                None,
            ),
            sweep_node(
                "pareto_output",
                WorkflowNodeKind::Output,
                None,
                vec![sweep_port("pareto")],
                vec![],
                None,
            ),
        ],
        edges: vec![
            sweep_edge(
                "edge-rows-envelope",
                "material_rows",
                "rows",
                "compose_envelopes",
                "rows",
            ),
            sweep_edge(
                "edge-envelope-rank",
                "compose_envelopes",
                "envelopes",
                "rank_envelopes",
                "envelopes",
            ),
            sweep_edge(
                "edge-envelope-pareto",
                "compose_envelopes",
                "envelopes",
                "pareto_envelopes",
                "envelopes",
            ),
            sweep_edge(
                "edge-rank-output",
                "rank_envelopes",
                "ranking",
                "ranking_output",
                "ranking",
            ),
            sweep_edge(
                "edge-pareto-output",
                "pareto_envelopes",
                "pareto",
                "pareto_output",
                "pareto",
            ),
        ],
    }
}

fn sweep_node(
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

fn sweep_port(id: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: "artifact/result_summary".to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: Some("parameter_sweep".to_string()),
    }
}

fn sweep_edge(
    id: &str,
    from_node: &str,
    from_port: &str,
    to_node: &str,
    to_port: &str,
) -> WorkflowEdge {
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
        dataset_value: Some("parameter_sweep".to_string()),
    }
}
