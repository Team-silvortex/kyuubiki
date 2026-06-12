use crate::workflow_contract::validate_workflow_dataset_contract;
use crate::workflow_executor::{
    artifact_key, evaluate_condition_operator, resolve_first_available_input_payload,
    resolve_named_input_payloads, resolve_single_input_payload, run_export_operator,
    run_extract_operator, run_solve_operator, run_transform_operator,
    transform_operator_accepts_partial_inputs, transform_operator_requires_port_map,
};
use kyuubiki_protocol::{
    WorkflowArtifactLineage, WorkflowBranchDecision, WorkflowGraphRunRequest,
    WorkflowGraphRunResult, WorkflowNodeKind, WorkflowNodeRunStatus, WorkflowNodeRunTrace,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

fn incoming_artifact_keys(
    incoming: &[&kyuubiki_protocol::WorkflowEdge],
    artifacts: &BTreeMap<String, Value>,
) -> Vec<String> {
    incoming
        .iter()
        .map(|edge| artifact_key(&edge.from.node, &edge.from.port))
        .filter(|key| artifacts.contains_key(key))
        .collect()
}

pub fn run_workflow_graph(
    request: WorkflowGraphRunRequest,
) -> Result<WorkflowGraphRunResult, String> {
    let graph = request.graph;
    validate_workflow_dataset_contract(&graph)?;
    let node_map = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect::<HashMap<_, _>>();
    let mut completed = HashSet::new();
    let mut skipped = HashSet::new();
    let mut ordered_completed = Vec::new();
    let mut ordered_skipped = Vec::new();
    let mut branch_decisions = Vec::new();
    let mut node_runs = Vec::new();
    let mut artifact_lineage = Vec::new();
    let mut artifacts = BTreeMap::new();

    loop {
        let mut progressed = false;

        for node in &graph.nodes {
            if completed.contains(&node.id) || skipped.contains(&node.id) {
                continue;
            }

            let incoming = graph
                .edges
                .iter()
                .filter(|edge| edge.to.node == node.id)
                .collect::<Vec<_>>();
            let supports_partial_inputs = node
                .operator_id
                .as_deref()
                .is_some_and(transform_operator_accepts_partial_inputs);
            let ready = if supports_partial_inputs {
                incoming.iter().any(|edge| {
                    artifacts.contains_key(&artifact_key(&edge.from.node, &edge.from.port))
                })
            } else {
                incoming.iter().all(|edge| {
                    artifacts.contains_key(&artifact_key(&edge.from.node, &edge.from.port))
                })
            };
            let unresolved_missing_inputs = incoming.iter().any(|edge| {
                let key = artifact_key(&edge.from.node, &edge.from.port);
                !artifacts.contains_key(&key)
            }) && incoming.iter().all(|edge| {
                let key = artifact_key(&edge.from.node, &edge.from.port);
                artifacts.contains_key(&key)
                    || completed.contains(&edge.from.node)
                    || skipped.contains(&edge.from.node)
            });

            if node.kind != WorkflowNodeKind::Input && !ready {
                if unresolved_missing_inputs {
                    skipped.insert(node.id.clone());
                    ordered_skipped.push(node.id.clone());
                    node_runs.push(WorkflowNodeRunTrace {
                        node_id: node.id.clone(),
                        kind: node.kind,
                        operator_id: node.operator_id.clone(),
                        status: WorkflowNodeRunStatus::Skipped,
                        consumed_artifacts: incoming_artifact_keys(&incoming, &artifacts),
                        produced_artifacts: Vec::new(),
                    });
                    progressed = true;
                }
                continue;
            }

            let consumed_artifacts = incoming_artifact_keys(&incoming, &artifacts);
            let mut produced_artifacts = Vec::new();

            match node.kind {
                WorkflowNodeKind::Input => {
                    let value =
                        request
                            .input_artifacts
                            .get(&node.id)
                            .cloned()
                            .ok_or_else(|| {
                                format!("missing workflow input artifact for node {}", node.id)
                            })?;
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), value.clone());
                        produced_artifacts.push(key.clone());
                        artifact_lineage.push(WorkflowArtifactLineage {
                            artifact_key: key,
                            node_id: node.id.clone(),
                            port_id: output.id.clone(),
                            source_artifacts: Vec::new(),
                        });
                    }
                }
                WorkflowNodeKind::Solve => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow solve node {} is missing operator_id", node.id)
                    })?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_solve_operator(operator_id, payload)?;
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), output_value.clone());
                        produced_artifacts.push(key.clone());
                        artifact_lineage.push(WorkflowArtifactLineage {
                            artifact_key: key,
                            node_id: node.id.clone(),
                            port_id: output.id.clone(),
                            source_artifacts: consumed_artifacts.clone(),
                        });
                    }
                }
                WorkflowNodeKind::Transform => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow transform node {} is missing operator_id", node.id)
                    })?;
                    let payload = if transform_operator_accepts_partial_inputs(operator_id) {
                        resolve_first_available_input_payload(node, &incoming, &artifacts)?
                    } else if transform_operator_requires_port_map(operator_id) {
                        resolve_named_input_payloads(node, &incoming, &artifacts)?
                    } else {
                        resolve_single_input_payload(node, &incoming, &artifacts)?
                    };
                    let output_value = run_transform_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), output_value.clone());
                        produced_artifacts.push(key.clone());
                        artifact_lineage.push(WorkflowArtifactLineage {
                            artifact_key: key,
                            node_id: node.id.clone(),
                            port_id: output.id.clone(),
                            source_artifacts: consumed_artifacts.clone(),
                        });
                    }
                }
                WorkflowNodeKind::Extract => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow extract node {} is missing operator_id", node.id)
                    })?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_extract_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), output_value.clone());
                        produced_artifacts.push(key.clone());
                        artifact_lineage.push(WorkflowArtifactLineage {
                            artifact_key: key,
                            node_id: node.id.clone(),
                            port_id: output.id.clone(),
                            source_artifacts: consumed_artifacts.clone(),
                        });
                    }
                }
                WorkflowNodeKind::Export => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow export node {} is missing operator_id", node.id)
                    })?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_export_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), output_value.clone());
                        produced_artifacts.push(key.clone());
                        artifact_lineage.push(WorkflowArtifactLineage {
                            artifact_key: key,
                            node_id: node.id.clone(),
                            port_id: output.id.clone(),
                            source_artifacts: consumed_artifacts.clone(),
                        });
                    }
                }
                WorkflowNodeKind::Output => {
                    for edge in incoming {
                        let source_key = artifact_key(&edge.from.node, &edge.from.port);
                        let value = artifacts.get(&source_key).cloned().ok_or_else(|| {
                            format!(
                                "workflow output node {} could not read {}.{}",
                                node.id, edge.from.node, edge.from.port
                            )
                        })?;
                        let key = artifact_key(&node.id, &edge.to.port);
                        artifacts.insert(key.clone(), value);
                        produced_artifacts.push(key.clone());
                        artifact_lineage.push(WorkflowArtifactLineage {
                            artifact_key: key,
                            node_id: node.id.clone(),
                            port_id: edge.to.port.clone(),
                            source_artifacts: vec![source_key],
                        });
                    }
                }
                WorkflowNodeKind::Condition => {
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let predicate_result = evaluate_condition_operator(
                        &payload,
                        &node.config.clone().unwrap_or(Value::Null),
                    )?;
                    let chosen_output = node
                        .outputs
                        .iter()
                        .find(|output| {
                            (predicate_result && (output.id == "if_true" || output.id == "true"))
                                || (!predicate_result
                                    && (output.id == "if_false" || output.id == "false"))
                        })
                        .or_else(|| {
                            if predicate_result {
                                node.outputs.first()
                            } else {
                                node.outputs.get(1).or_else(|| node.outputs.first())
                            }
                        })
                        .ok_or_else(|| {
                            format!(
                                "workflow condition node {} requires branch output ports",
                                node.id
                            )
                        })?;
                    let key = artifact_key(&node.id, &chosen_output.id);
                    artifacts.insert(key.clone(), payload);
                    produced_artifacts.push(key.clone());
                    artifact_lineage.push(WorkflowArtifactLineage {
                        artifact_key: key,
                        node_id: node.id.clone(),
                        port_id: chosen_output.id.clone(),
                        source_artifacts: consumed_artifacts.clone(),
                    });
                    branch_decisions.push(WorkflowBranchDecision {
                        node_id: node.id.clone(),
                        chosen_output: chosen_output.id.clone(),
                        predicate_result,
                    });
                }
            }

            completed.insert(node.id.clone());
            ordered_completed.push(node.id.clone());
            node_runs.push(WorkflowNodeRunTrace {
                node_id: node.id.clone(),
                kind: node.kind,
                operator_id: node.operator_id.clone(),
                status: WorkflowNodeRunStatus::Completed,
                consumed_artifacts,
                produced_artifacts,
            });
            progressed = true;
        }

        if completed.len() + skipped.len() == graph.nodes.len() {
            break;
        }
        if !progressed {
            let pending = graph
                .nodes
                .iter()
                .filter(|node| !completed.contains(&node.id) && !skipped.contains(&node.id))
                .map(|node| node.id.clone())
                .collect::<Vec<_>>();
            return Err(format!(
                "workflow graph could not make progress; pending nodes: {}",
                pending.join(", ")
            ));
        }
    }

    for node_id in &graph.output_nodes {
        if !node_map.contains_key(node_id) {
            return Err(format!("workflow output node {} is not defined", node_id));
        }
    }

    Ok(WorkflowGraphRunResult {
        workflow_id: graph.id,
        completed_nodes: ordered_completed,
        skipped_nodes: ordered_skipped,
        branch_decisions,
        node_runs,
        artifact_lineage,
        artifacts,
    })
}
