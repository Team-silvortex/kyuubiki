use crate::workflow_contract::validate_workflow_dataset_contract;
use crate::workflow_executor::{
    artifact_key, resolve_single_input_payload, run_export_operator, run_extract_operator,
    run_solve_operator, run_transform_operator,
};
use kyuubiki_protocol::{WorkflowGraphRunRequest, WorkflowGraphRunResult, WorkflowNodeKind};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

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
    let mut ordered_completed = Vec::new();
    let mut artifacts = BTreeMap::new();

    loop {
        let mut progressed = false;

        for node in &graph.nodes {
            if completed.contains(&node.id) {
                continue;
            }

            let incoming = graph
                .edges
                .iter()
                .filter(|edge| edge.to.node == node.id)
                .collect::<Vec<_>>();
            let ready = incoming.iter().all(|edge| {
                artifacts.contains_key(&artifact_key(&edge.from.node, &edge.from.port))
            });

            if node.kind != WorkflowNodeKind::Input && !ready {
                continue;
            }

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
                        artifacts.insert(artifact_key(&node.id, &output.id), value.clone());
                    }
                }
                WorkflowNodeKind::Solve => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow solve node {} is missing operator_id", node.id)
                    })?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_solve_operator(operator_id, payload)?;
                    for output in &node.outputs {
                        artifacts.insert(artifact_key(&node.id, &output.id), output_value.clone());
                    }
                }
                WorkflowNodeKind::Transform => {
                    let operator_id = node.operator_id.as_deref().ok_or_else(|| {
                        format!("workflow transform node {} is missing operator_id", node.id)
                    })?;
                    let payload = resolve_single_input_payload(node, &incoming, &artifacts)?;
                    let output_value = run_transform_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    for output in &node.outputs {
                        artifacts.insert(artifact_key(&node.id, &output.id), output_value.clone());
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
                        artifacts.insert(artifact_key(&node.id, &output.id), output_value.clone());
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
                        artifacts.insert(artifact_key(&node.id, &output.id), output_value.clone());
                    }
                }
                WorkflowNodeKind::Output => {
                    for edge in incoming {
                        let value = artifacts
                            .get(&artifact_key(&edge.from.node, &edge.from.port))
                            .cloned()
                            .ok_or_else(|| {
                                format!(
                                    "workflow output node {} could not read {}.{}",
                                    node.id, edge.from.node, edge.from.port
                                )
                            })?;
                        artifacts.insert(artifact_key(&node.id, &edge.to.port), value);
                    }
                }
                _ => {
                    return Err(format!(
                        "workflow node kind {:?} is not supported by the first headless executor",
                        node.kind
                    ));
                }
            }

            completed.insert(node.id.clone());
            ordered_completed.push(node.id.clone());
            progressed = true;
        }

        if completed.len() == graph.nodes.len() {
            break;
        }
        if !progressed {
            let pending = graph
                .nodes
                .iter()
                .filter(|node| !completed.contains(&node.id))
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
        artifacts,
    })
}
