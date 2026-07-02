use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_parameter_sweep_result_scoring_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: parameter_sweep_result_scoring_graph(5.0),
        input_artifacts: sweep_result_inputs(),
    })
    .expect("parameter sweep result scoring workflow should run");

    let joined = run
        .artifacts
        .get("join_results.joined")
        .expect("joined sweep results should exist");
    assert_eq!(joined["joined_summary_count"].as_u64(), Some(2));
    assert_eq!(joined["missing_summary_count"].as_u64(), Some(0));

    let summarized = run
        .artifacts
        .get("summarize_results.summary")
        .expect("summarized sweep rows should exist");
    assert_eq!(summarized["row_count"].as_u64(), Some(2));
    assert_eq!(
        summarized["numeric_columns"]["max_stress"]["min"].as_f64(),
        Some(84.0)
    );
    assert_eq!(
        summarized["rows"][1]["metadata"]["round"].as_str(),
        Some("seed")
    );
    assert_eq!(
        summarized["rows"][1]["metadata"]["source_candidate_id"].as_str(),
        Some("baseline")
    );

    let scored = run
        .artifacts
        .get("score_results.scored")
        .expect("scored sweep rows should exist");
    assert_eq!(scored["scored_count"].as_u64(), Some(2));
    assert_eq!(scored["best"]["case_id"].as_str(), Some("material_panel_1"));
    assert_eq!(scored["best"]["objective_feasible"].as_bool(), Some(true));
    assert_eq!(
        scored["scored_rows"][1]["case_id"].as_str(),
        Some("material_panel_0")
    );
    assert_eq!(
        scored["scored_rows"][1]["objective_feasible"].as_bool(),
        Some(false)
    );
    assert_eq!(scored["best"]["metadata"]["round"].as_str(), Some("seed"));

    let candidates = run
        .artifacts
        .get("map_quality_candidates.candidates")
        .expect("quality candidates should exist");
    assert_eq!(candidates["candidate_count"].as_u64(), Some(2));
    assert_eq!(
        candidates["source_best_case_id"].as_str(),
        Some("material_panel_1")
    );
    assert_eq!(
        candidates["candidates"]["material_panel_1"]["source_row"]["metadata"]
            ["source_candidate_id"]
            .as_str(),
        Some("baseline")
    );

    let ranking = run
        .artifacts
        .get("rank_quality_candidates.ranking")
        .expect("quality ranking should exist");
    assert_eq!(
        ranking["best_candidate_id"].as_str(),
        Some("material_panel_1")
    );
    assert_eq!(ranking["ready_candidate_count"].as_u64(), Some(1));

    let request = run
        .artifacts
        .get("prepare_next_round.request")
        .expect("next round request should exist");
    assert_eq!(request["action"].as_str(), Some("continue"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("material_panel_1")
    );

    let plan = run
        .artifacts
        .get("build_next_plan.plan")
        .expect("next sweep plan should exist");
    assert_eq!(plan["sweep_enabled"].as_bool(), Some(true));
    assert_eq!(
        plan["source_candidate_id"].as_str(),
        Some("material_panel_1")
    );
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(4));

    let next_cases = run
        .artifacts
        .get("expand_next_cases.cases")
        .expect("next sweep cases should exist");
    assert_eq!(next_cases["case_count"].as_u64(), Some(4));
    assert_eq!(
        next_cases["cases"][0]["id"].as_str(),
        Some("material_next_round_0")
    );
    assert_eq!(
        next_cases["cases"][3]["model"]["material"]["density"].as_f64(),
        Some(7800.0)
    );
    assert_eq!(
        next_cases["cases"][0]["metadata"]["source_candidate_id"].as_str(),
        Some("material_panel_1")
    );
    assert_eq!(
        next_cases["cases"][0]["metadata"]["source_plan_contract"].as_str(),
        Some("kyuubiki.quality_parameter_sweep_plan/v1")
    );
}

#[test]
fn stops_parameter_sweep_result_scoring_workflow_graph_when_target_is_met() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: parameter_sweep_result_scoring_graph(20.0),
        input_artifacts: sweep_result_inputs(),
    })
    .expect("parameter sweep result scoring workflow should stop cleanly");

    let request = run
        .artifacts
        .get("prepare_next_round.request")
        .expect("next round request should exist");
    assert_eq!(request["action"].as_str(), Some("stop"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("material_panel_1")
    );

    let plan = run
        .artifacts
        .get("build_next_plan.plan")
        .expect("disabled next sweep plan should exist");
    assert_eq!(plan["sweep_enabled"].as_bool(), Some(false));
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(0));
    assert_eq!(plan["sweep_action"].as_str(), Some("stop"));

    let next_cases = run
        .artifacts
        .get("expand_next_cases.cases")
        .expect("empty next cases should exist");
    assert_eq!(next_cases["case_count"].as_u64(), Some(0));
    assert_eq!(next_cases["sweep_enabled"].as_bool(), Some(false));
    assert_eq!(next_cases["sweep_action"].as_str(), Some("stop"));
}

fn sweep_result_inputs() -> BTreeMap<String, serde_json::Value> {
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

fn parameter_sweep_result_scoring_graph(target_score: f64) -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.parameter-sweep-result-scoring".to_string(),
        name: "Parameter sweep result scoring".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["sweep_cases".to_string(), "agent_results".to_string()],
        output_nodes: vec!["next_cases_output".to_string()],
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
                "next_cases_output",
                WorkflowNodeKind::Output,
                None,
                vec![sweep_port("cases")],
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
                "edge-expand-output",
                "expand_next_cases",
                "cases",
                "next_cases_output",
                "cases",
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
