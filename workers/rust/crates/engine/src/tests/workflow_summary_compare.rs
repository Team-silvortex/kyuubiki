use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_summary_compare_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.summary-compare".to_string(),
        name: "Summary compare".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Compare baseline and candidate summary artifacts.".to_string()),
        dataset_contract: None,
        entry_nodes: vec![
            "baseline_summary".to_string(),
            "candidate_summary".to_string(),
        ],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            input_node("baseline_summary"),
            input_node("candidate_summary"),
            WorkflowNode {
                id: "compare_summary".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.compare_summary_pair".to_string()),
                name: Some("Compare summary pair".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "left_prefix": "baseline",
                    "right_prefix": "candidate",
                    "delta_prefix": "delta",
                    "ratio_prefix": "ratio",
                    "percent_prefix": "percent_change",
                    "include_originals": true,
                    "include_delta": true,
                    "include_ratio": true,
                    "include_percent_change": true,
                    "include_shared_field_count": true
                })),
                cache_policy: None,
                inputs: vec![
                    port("left", "artifact/result_summary"),
                    port("right", "artifact/result_summary"),
                ],
                outputs: vec![port("merged", "artifact/result_summary")],
            },
            WorkflowNode {
                id: "export_json".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export JSON".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("summary", "artifact/result_summary")],
                outputs: vec![port("json", "artifact/json")],
            },
            WorkflowNode {
                id: "json_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("JSON output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("json", "artifact/json")],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "edge-baseline",
                "baseline_summary",
                "summary",
                "compare_summary",
                "left",
                "artifact/result_summary",
            ),
            edge(
                "edge-candidate",
                "candidate_summary",
                "summary",
                "compare_summary",
                "right",
                "artifact/result_summary",
            ),
            edge(
                "edge-compared-summary",
                "compare_summary",
                "merged",
                "export_json",
                "summary",
                "artifact/result_summary",
            ),
            edge(
                "edge-json-output",
                "export_json",
                "json",
                "json_output",
                "json",
                "artifact/json",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([
            (
                "baseline_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 100.0,
                    "max_heat_flux": 50.0,
                    "max_displacement": 2.0
                }),
            ),
            (
                "candidate_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 120.0,
                    "max_heat_flux": 40.0,
                    "max_displacement": 3.0
                }),
            ),
        ]),
    })
    .expect("summary compare workflow should run");

    let compared = run
        .artifacts
        .get("compare_summary.merged")
        .cloned()
        .expect("compared summary should exist");
    assert_eq!(
        compared["baseline_max_temperature"],
        serde_json::json!(100.0)
    );
    assert_eq!(
        compared["candidate_max_temperature"],
        serde_json::json!(120.0)
    );
    assert_eq!(compared["delta_max_temperature"], serde_json::json!(20.0));
    assert_eq!(compared["ratio_max_temperature"], serde_json::json!(1.2));
    assert_eq!(
        compared["percent_change_max_heat_flux"],
        serde_json::json!(-20.0)
    );
    assert_eq!(
        compared["summary_shared_numeric_field_count"],
        serde_json::json!(3)
    );

    let exported = run
        .artifacts
        .get("json_output.json")
        .cloned()
        .expect("json export artifact should exist");
    assert_eq!(exported["format"], serde_json::json!("json"));
    let content = exported["content"]
        .as_str()
        .expect("json content should be a string");
    assert!(content.contains("delta_max_temperature"));
    assert!(content.contains("ratio_max_temperature"));
    assert!(content.contains("percent_change_max_heat_flux"));
}

#[test]
fn compare_summary_skips_ratio_when_baseline_is_zero() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.summary-compare-zero-baseline".to_string(),
        name: "Summary compare zero baseline".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Compare summaries while skipping divide-by-zero ratios.".to_string()),
        dataset_contract: None,
        entry_nodes: vec![
            "baseline_summary".to_string(),
            "candidate_summary".to_string(),
        ],
        output_nodes: vec!["summary_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("baseline_summary"),
            input_node("candidate_summary"),
            WorkflowNode {
                id: "compare_summary".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.compare_summary_pair".to_string()),
                name: Some("Compare summary pair".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "fields": ["max_heat_flux"],
                    "left_prefix": "baseline",
                    "right_prefix": "candidate",
                    "include_originals": true,
                    "include_delta": true,
                    "include_ratio": true,
                    "include_percent_change": true
                })),
                cache_policy: None,
                inputs: vec![
                    port("left", "artifact/result_summary"),
                    port("right", "artifact/result_summary"),
                ],
                outputs: vec![port("merged", "artifact/result_summary")],
            },
            output_node("summary_output"),
        ],
        edges: vec![
            edge(
                "edge-baseline",
                "baseline_summary",
                "summary",
                "compare_summary",
                "left",
                "artifact/result_summary",
            ),
            edge(
                "edge-candidate",
                "candidate_summary",
                "summary",
                "compare_summary",
                "right",
                "artifact/result_summary",
            ),
            edge(
                "edge-output",
                "compare_summary",
                "merged",
                "summary_output",
                "summary",
                "artifact/result_summary",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([
            (
                "baseline_summary".to_string(),
                serde_json::json!({ "max_heat_flux": 0.0 }),
            ),
            (
                "candidate_summary".to_string(),
                serde_json::json!({ "max_heat_flux": 40.0 }),
            ),
        ]),
    })
    .expect("zero-baseline comparison should run");

    let compared = run
        .artifacts
        .get("summary_output.summary")
        .cloned()
        .expect("compared summary should exist");
    assert_eq!(compared["delta_max_heat_flux"], serde_json::json!(40.0));
    assert!(compared.get("ratio_max_heat_flux").is_none());
    assert!(compared.get("percent_change_max_heat_flux").is_none());
}

#[test]
fn validates_summary_pair_against_tolerances() {
    let validation = crate::workflow_executor::run_transform_operator(
        "transform.validate_summary_tolerance",
        serde_json::json!({
            "left": {
                "max_temperature": 100.0,
                "max_displacement": 2.0
            },
            "right": {
                "max_temperature": 100.00002,
                "max_displacement": 2.004
            }
        }),
        serde_json::json!({
            "fields": ["max_temperature", "max_displacement"],
            "absolute_tolerance": 0.01,
            "relative_tolerance": 0.001
        }),
    )
    .expect("summary tolerance validation should run");

    assert_eq!(validation["validation_passed"].as_bool(), Some(true));
    assert_eq!(validation["validation_grade"].as_str(), Some("pass"));
    assert_eq!(
        validation["validation_checked_field_count"].as_u64(),
        Some(2)
    );
    assert_eq!(
        validation["validation_failed_field_count"].as_u64(),
        Some(0)
    );
}

#[test]
fn blocks_summary_pair_when_tolerance_or_required_field_fails() {
    let validation = crate::workflow_executor::run_transform_operator(
        "transform.validate_summary_tolerance",
        serde_json::json!({
            "left": {
                "max_temperature": 100.0,
                "max_displacement": 2.0
            },
            "right": {
                "max_temperature": 103.0
            }
        }),
        serde_json::json!({
            "fields": ["max_temperature", "max_displacement"],
            "absolute_tolerance": 0.1,
            "relative_tolerance": 0.001,
            "fail_on_missing": true
        }),
    )
    .expect("summary tolerance validation should report failures");

    assert_eq!(validation["validation_passed"].as_bool(), Some(false));
    assert_eq!(validation["validation_grade"].as_str(), Some("block"));
    assert_eq!(
        validation["validation_failed_field_count"].as_u64(),
        Some(1)
    );
    assert_eq!(
        validation["validation_missing_field_count"].as_u64(),
        Some(1)
    );
    assert_eq!(
        validation["validation_failures"][0]["field"].as_str(),
        Some("max_temperature")
    );
    assert_eq!(
        validation["validation_missing_fields"][0].as_str(),
        Some("max_displacement")
    );
}

#[test]
fn runs_summary_tolerance_validation_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.summary-tolerance-validation".to_string(),
        name: "Summary tolerance validation".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Validate repeated solver summaries before accepting a result.".to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec![
            "reference_summary".to_string(),
            "candidate_summary".to_string(),
        ],
        output_nodes: vec!["validation_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("reference_summary"),
            input_node("candidate_summary"),
            WorkflowNode {
                id: "validate_summary".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.validate_summary_tolerance".to_string()),
                name: Some("Validate summary tolerance".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "fields": ["max_temperature", "max_heat_flux"],
                    "absolute_tolerance": 0.05,
                    "relative_tolerance": 0.001
                })),
                cache_policy: None,
                inputs: vec![
                    port("left", "artifact/result_summary"),
                    port("right", "artifact/result_summary"),
                ],
                outputs: vec![port("summary", "artifact/result_summary")],
            },
            output_node("validation_output"),
        ],
        edges: vec![
            edge(
                "edge-reference",
                "reference_summary",
                "summary",
                "validate_summary",
                "left",
                "artifact/result_summary",
            ),
            edge(
                "edge-candidate",
                "candidate_summary",
                "summary",
                "validate_summary",
                "right",
                "artifact/result_summary",
            ),
            edge(
                "edge-validation",
                "validate_summary",
                "summary",
                "validation_output",
                "summary",
                "artifact/result_summary",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([
            (
                "reference_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 100.0,
                    "max_heat_flux": 50.0
                }),
            ),
            (
                "candidate_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 100.02,
                    "max_heat_flux": 49.98
                }),
            ),
        ]),
    })
    .expect("summary tolerance validation workflow should run");

    let validation = run
        .artifacts
        .get("validation_output.summary")
        .cloned()
        .expect("validation output should exist");
    assert_eq!(
        validation["validation_contract"].as_str(),
        Some("kyuubiki.summary_tolerance_validation/v1")
    );
    assert_eq!(validation["validation_passed"].as_bool(), Some(true));
}

fn input_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: Some("Summary input".to_string()),
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("summary", "artifact/result_summary")],
    }
}

fn output_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: Some("Summary output".to_string()),
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("summary", "artifact/result_summary")],
        outputs: vec![],
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: Some("result_summary".to_string()),
    }
}

fn edge(
    id: &str,
    from_node: &str,
    from_port: &str,
    to_node: &str,
    to_port: &str,
    artifact_type: &str,
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
        artifact_type: artifact_type.to_string(),
        dataset_value: Some("result_summary".to_string()),
    }
}
