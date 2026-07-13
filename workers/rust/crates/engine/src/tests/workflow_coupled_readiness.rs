use crate::{run_workflow_graph, workflow_executor::run_transform_operator};
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_coupled_readiness_transform_through_sdk_registry() {
    let summary = run_transform_operator(
        "transform.evaluate_coupled_readiness",
        serde_json::json!({
            "domains": {
                "thermal": {
                    "thermal_quality_ready": true,
                    "thermal_quality_grade": "good",
                    "thermal_quality_score": 0.18
                },
                "structural": {
                    "structural_quality_ready": true,
                    "structural_quality_grade": "watch",
                    "structural_quality_score": 0.42
                },
                "electrostatic": {
                    "electrostatic_quality_ready": false,
                    "electrostatic_quality_grade": "block",
                    "electrostatic_quality_score": 1.4
                }
            }
        }),
        serde_json::json!({
            "required_domains": ["thermal", "structural", "electrostatic"],
            "domains": {
                "electrostatic": { "max_score": 1.0 }
            }
        }),
    )
    .expect("coupled readiness transform should run through registry");

    assert_eq!(
        summary["coupled_readiness_contract"].as_str(),
        Some("kyuubiki.coupled_readiness/v1")
    );
    assert_eq!(summary["coupled_readiness_ready"].as_bool(), Some(false));
    assert_eq!(summary["coupled_readiness_state"].as_str(), Some("block"));
    assert_eq!(summary["coupled_readiness_present_count"].as_u64(), Some(3));
    assert_eq!(summary["coupled_readiness_ready_count"].as_u64(), Some(2));
    assert_eq!(
        summary["coupled_readiness_recommendation"].as_str(),
        Some("hold_and_repair_inputs")
    );
    assert_eq!(
        summary["coupled_readiness_blocking_domains"][0].as_str(),
        Some("electrostatic")
    );
    assert_eq!(
        summary["coupled_readiness_warning_domains"][0].as_str(),
        Some("structural")
    );
}

#[test]
fn blocks_coupled_readiness_when_required_domain_is_missing() {
    let summary = run_transform_operator(
        "transform.evaluate_coupled_readiness",
        serde_json::json!({
            "thermal": {
                "thermal_quality_ready": true,
                "thermal_quality_grade": "good",
                "thermal_quality_score": 0.1
            }
        }),
        serde_json::json!({
            "required_domains": ["thermal", "thermo"]
        }),
    )
    .expect("missing required domain should produce a readiness report");

    assert_eq!(summary["coupled_readiness_ready"].as_bool(), Some(false));
    assert_eq!(summary["coupled_readiness_state"].as_str(), Some("block"));
    assert_eq!(
        summary["coupled_readiness_recommendation"].as_str(),
        Some("hold_and_repair_inputs")
    );
    assert_eq!(
        summary["coupled_readiness_required_missing"][0].as_str(),
        Some("thermo")
    );
}

#[test]
fn runs_coupled_readiness_as_multi_input_workflow_gate() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.coupled-readiness-gate".to_string(),
        name: "Coupled readiness gate".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Gate a coupled heat/electrostatic chain before optimization.".to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec![
            "thermal_quality".to_string(),
            "electrostatic_quality".to_string(),
        ],
        output_nodes: vec!["readiness_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            input_node("thermal_quality"),
            input_node("electrostatic_quality"),
            WorkflowNode {
                id: "gate".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.evaluate_coupled_readiness".to_string()),
                name: Some("Evaluate coupled readiness".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "required_domains": ["thermal", "electrostatic"],
                    "domains": {
                        "thermal": { "max_score": 1.0 },
                        "electrostatic": { "max_score": 1.0 }
                    }
                })),
                cache_policy: None,
                inputs: vec![
                    port("thermal", "artifact/json"),
                    port("electrostatic", "artifact/json"),
                ],
                outputs: vec![port("result", "artifact/json")],
            },
            WorkflowNode {
                id: "readiness_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("Readiness output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("result", "artifact/json")],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "thermal-input",
                "thermal_quality",
                "summary",
                "gate",
                "thermal",
            ),
            edge(
                "electrostatic-input",
                "electrostatic_quality",
                "summary",
                "gate",
                "electrostatic",
            ),
            edge(
                "gate-output",
                "gate",
                "result",
                "readiness_output",
                "result",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([
            (
                "thermal_quality".to_string(),
                serde_json::json!({
                    "thermal_quality_ready": true,
                    "thermal_quality_grade": "good",
                    "thermal_quality_score": 0.2
                }),
            ),
            (
                "electrostatic_quality".to_string(),
                serde_json::json!({
                    "electrostatic_quality_ready": true,
                    "electrostatic_quality_grade": "good",
                    "electrostatic_quality_score": 0.35
                }),
            ),
        ]),
    })
    .expect("coupled readiness workflow should run");

    assert_eq!(run.workflow_id, "workflow.coupled-readiness-gate");
    assert_eq!(run.completed_nodes.len(), 4);
    let readiness = run
        .artifacts
        .get("gate.result")
        .expect("readiness result should be emitted");
    assert_eq!(readiness["coupled_readiness_ready"].as_bool(), Some(true));
    assert_eq!(readiness["coupled_readiness_state"].as_str(), Some("pass"));
    assert_eq!(
        readiness["coupled_readiness_recommendation"].as_str(),
        Some("continue_to_next_round")
    );
    assert_eq!(readiness["coupled_readiness_ready_count"].as_u64(), Some(2));
}

fn input_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: Some(id.to_string()),
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("summary", "artifact/json")],
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
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
        artifact_type: "artifact/json".to_string(),
        dataset_value: None,
    }
}
