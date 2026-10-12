use super::workflow_quality_sweep_expansion_admission::fixtures;
use super::workflow_quality_sweep_request_recovery;
use crate::{run_workflow_graph, workflow_executor::run_transform_operator};
use kyuubiki_protocol::{WorkflowGraph, WorkflowGraphRunRequest};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn graph(policy: &str) -> WorkflowGraph {
    let mut graph = workflow_quality_sweep_request_recovery::graph(policy);
    graph
        .nodes
        .retain(|node| !["build_plan", "materialize"].contains(&node.id.as_str()));
    graph
        .edges
        .retain(|edge| !["build_plan", "materialize"].contains(&edge.to.node.as_str()));
    let edge = graph
        .edges
        .iter_mut()
        .find(|edge| edge.to.node == "expand_cases")
        .unwrap();
    edge.from.node = "quality_candidates".into();
    edge.from.port = "payload".into();
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "expand_cases")
        .unwrap()
        .config = Some(json!({"on_error":policy}));
    graph
}

#[test]
fn expansion_errors_block_dependents_and_fresh_repairs_keep_quality_evidence() {
    let fixtures = fixtures();
    for id in [
        "wrapped_missing_repair",
        "raw_missing_review",
        "wrapped_wrong_focus",
        "payload_null_masks_config",
        "wrapped_blocked_budget",
        "failed_body",
    ] {
        for policy in ["fail", "skip"] {
            let graph = graph(policy);
            let run = |payload| {
                run_workflow_graph(WorkflowGraphRunRequest {
                    graph: graph.clone(),
                    input_artifacts: BTreeMap::from([("quality_candidates".into(), payload)]),
                })
            };
            let bad = fixtures.iter().find(|row| row["id"] == id).unwrap()["payload"].clone();
            let result = run(bad);
            if policy == "fail" {
                assert!(result.is_err(), "{id}");
            } else {
                let rejected = result.unwrap();
                assert_eq!(rejected.failed_nodes, ["expand_cases"], "{id}");
                assert_eq!(rejected.completed_nodes, ["quality_candidates"]);
                assert!(!rejected.artifacts.contains_key("expand_cases.cases"));
            }
            let repaired =
                fixtures.iter().find(|row| row["id"] == "repair").unwrap()["payload"].clone();
            let fresh = run(repaired.clone()).unwrap();
            assert!(fresh.failed_nodes.is_empty());
            let cases = &fresh.artifacts["expand_cases.cases"];
            assert_eq!(cases["case_count"], 2);
            for case in cases["cases"].as_array().unwrap() {
                assert_eq!(case["metadata"], repaired["payload"]["case_metadata"]);
            }
        }
    }
}

#[test]
fn expansion_metadata_alias_matrix_cannot_mask_quality_evidence() {
    let source = fixtures()
        .into_iter()
        .find(|row| row["id"] == "repair")
        .unwrap()["payload"]
        .clone();
    let choices = [
        None,
        Some(Value::Null),
        Some(source["payload"]["case_metadata"].clone()),
        Some(
            json!({"coupled_readiness":{"coupled_readiness_state":"warn"},"optimization_hint":{"action":"review_coupled_readiness"}}),
        ),
        Some(json!("invalid")),
    ];
    for a in &choices {
        for b in &choices {
            for c in &choices {
                let mut payload = source.clone();
                payload["payload"]
                    .as_object_mut()
                    .unwrap()
                    .remove("case_metadata");
                let mut config = json!({});
                if let Some(value) = a {
                    payload["payload"]["case_metadata"] = value.clone();
                }
                if let Some(value) = b {
                    payload["config"]["case_metadata"] = value.clone();
                }
                if let Some(value) = c {
                    config["case_metadata"] = value.clone();
                }
                let aliases: Vec<_> = [a, b, c].into_iter().filter_map(|v| v.as_ref()).collect();
                let accepted = aliases.iter().all(|v| v.is_null() || v.is_object())
                    && aliases.windows(2).all(|pair| pair[0] == pair[1]);
                let exchanged =
                    serde_json::from_slice(&serde_json::to_vec(&payload).unwrap()).unwrap();
                let result =
                    run_transform_operator("transform.expand_parameter_sweep", exchanged, config);
                assert_eq!(result.is_ok(), accepted, "{aliases:?}: {result:?}");
                if accepted {
                    for case in result.unwrap()["cases"].as_array().unwrap() {
                        assert_eq!(
                            &case["metadata"],
                            aliases.first().copied().unwrap_or(&json!({}))
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn generated_expansions_reject_corrupted_evidence_and_keep_valid_repair_controls() {
    for state in ["ready", "warn", "block"] {
        let report = json!({"coupled_readiness_state":state});
        let decision = run_transform_operator("transform.prepare_quality_next_round_request",
            json!({"ranking":[{"candidate_id":"a","score":1,"ready":true,"coupled_readiness":report}]}),
            json!({"target_score":0.1,"search_space":{"temperature_delta":[120,160]}})).unwrap();
        let plan = run_transform_operator(
            "transform.build_quality_parameter_sweep_plan",
            decision,
            json!({}),
        )
        .unwrap();
        let expansion = run_transform_operator(
            "transform.materialize_quality_sweep_expansion",
            plan,
            json!({}),
        )
        .unwrap();
        for (field, value) in [
            ("converged", json!(false)),
            ("repair_strategy", json!("invalid")),
            ("repair_focus", json!({"domain":"mismatched"})),
            ("source_ranking_complete", json!(false)),
            ("sweep_enabled", json!(false)),
            ("sweep_budget", json!({"case_budget_exceeded":true})),
        ] {
            let mut changed = expansion.clone();
            changed["payload"]["case_metadata"][field] = value;
            assert!(
                run_transform_operator("transform.expand_parameter_sweep", changed, json!({}))
                    .is_err(),
                "{state}:{field}"
            );
        }
        let result = run_transform_operator(
            "transform.expand_parameter_sweep",
            expansion.clone(),
            json!({}),
        )
        .unwrap();
        assert_eq!(result["case_count"], 2);
        for case in result["cases"].as_array().unwrap() {
            assert_eq!(case["metadata"], expansion["payload"]["case_metadata"]);
        }
    }
}
