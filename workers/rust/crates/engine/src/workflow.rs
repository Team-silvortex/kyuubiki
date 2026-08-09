use crate::workflow_artifact_retention::WorkflowArtifactRetention;
use crate::workflow_contract::validate_workflow_dataset_contract;
use crate::workflow_execution_plan::{WorkflowExecutionPlan, WorkflowPlannedEdge};
use crate::workflow_executor::{
    artifact_key, evaluate_condition_operator, run_export_operator, run_extract_operator,
    run_solve_operator, run_transform_operator, transform_operator_accepts_partial_inputs,
    transform_operator_requires_port_map,
};
use crate::workflow_security::{validate_workflow_artifact_budget, validate_workflow_security};
use kyuubiki_protocol::{
    JobStatus, WorkflowArtifactLineage, WorkflowBranchDecision, WorkflowGraphRunRequest,
    WorkflowGraphRunResult, WorkflowNodeKind, WorkflowNodeRunStatus, WorkflowNodeRunTrace,
    WorkflowProgressEvent,
};
use serde_json::Value;
use std::any::Any;
use std::collections::{BTreeMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

fn node_allows_skip_on_error(config: Option<&Value>) -> bool {
    let Some(config) = config else {
        return false;
    };
    config
        .get("on_error")
        .or_else(|| config.pointer("/recovery/on_error"))
        .and_then(Value::as_str)
        .is_some_and(|value| value == "skip")
}

pub(crate) fn run_with_panic_boundary<F>(node_id: &str, run: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    match catch_unwind(AssertUnwindSafe(run)) {
        Ok(result) => result,
        Err(payload) => Err(format!(
            "workflow node {node_id} panicked: {}",
            panic_payload_message(payload.as_ref())
        )),
    }
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}

fn workflow_progress_event(
    stage: JobStatus,
    progress: f32,
    message: impl Into<String>,
    node_id: Option<&str>,
    kind: Option<WorkflowNodeKind>,
) -> WorkflowProgressEvent {
    WorkflowProgressEvent {
        stage,
        progress,
        message: Some(message.into()),
        node_id: node_id.map(ToOwned::to_owned),
        kind: kind.map(|value| match value {
            WorkflowNodeKind::Input => "input".to_string(),
            WorkflowNodeKind::Solve => "solve".to_string(),
            WorkflowNodeKind::Transform => "transform".to_string(),
            WorkflowNodeKind::Extract => "extract".to_string(),
            WorkflowNodeKind::Export => "export".to_string(),
            WorkflowNodeKind::Condition => "condition".to_string(),
            WorkflowNodeKind::Output => "output".to_string(),
        }),
        emitted_at: OffsetDateTime::now_utc().format(&Rfc3339).ok(),
    }
}

pub fn run_workflow_graph(
    request: WorkflowGraphRunRequest,
) -> Result<WorkflowGraphRunResult, String> {
    validate_workflow_security(&request)?;
    let graph = request.graph;
    validate_workflow_dataset_contract(&graph)?;
    let execution_plan = WorkflowExecutionPlan::compile(&graph)?;
    let mut artifact_retention = WorkflowArtifactRetention::compile(&graph);
    let node_count = graph.nodes.len();
    let mut ordered_completed = Vec::with_capacity(node_count);
    let mut ordered_skipped = Vec::new();
    let mut ordered_failed = Vec::new();
    let mut branch_decisions = Vec::new();
    let mut node_runs = Vec::with_capacity(node_count);
    let mut artifact_lineage =
        Vec::with_capacity(graph.edges.len().saturating_add(graph.entry_nodes.len()));
    let mut progress_events = Vec::with_capacity(node_count.saturating_add(3));
    progress_events.push(workflow_progress_event(
        JobStatus::Preprocessing,
        0.0,
        format!("workflow {} accepted", graph.id),
        None,
        None,
    ));
    let mut artifacts = BTreeMap::new();
    let mut output_budget_validated_artifacts = HashSet::with_capacity(graph.edges.len());
    let total_nodes = node_count.max(1) as f32;

    for &node_index in execution_plan.node_order() {
        let node = &graph.nodes[node_index];
        let incoming = execution_plan.incoming(node_index);
        let incoming_state = execution_plan.resolve_incoming(node_index, &artifacts);
        let supports_partial_inputs = node
            .operator_id
            .as_deref()
            .is_some_and(transform_operator_accepts_partial_inputs);
        let ready = if supports_partial_inputs {
            incoming_state.any_resolved
        } else {
            incoming_state.all_resolved
        };

        if node.kind != WorkflowNodeKind::Input && !ready {
            remove_releasable_artifacts(
                artifact_retention.finish_node(incoming, &[]),
                &mut artifacts,
                &mut output_budget_validated_artifacts,
            );
            ordered_skipped.push(node.id.clone());
            node_runs.push(WorkflowNodeRunTrace {
                node_id: node.id.clone(),
                kind: node.kind,
                operator_id: node.operator_id.clone(),
                status: WorkflowNodeRunStatus::Skipped,
                consumed_artifacts: incoming_state.resolved_artifact_keys,
                produced_artifacts: Vec::new(),
                error_message: None,
            });
            progress_events.push(workflow_progress_event(
                JobStatus::Partitioning,
                (ordered_completed.len() + ordered_skipped.len() + ordered_failed.len()) as f32
                    / total_nodes,
                format!("skipped node {}", node.id),
                Some(&node.id),
                Some(node.kind),
            ));
            continue;
        }

        let consumed_artifacts = incoming_state.resolved_artifact_keys;
        let mut produced_artifacts = Vec::with_capacity(node.outputs.len().max(incoming.len()));
        let artifact_lineage_checkpoint = artifact_lineage.len();
        let branch_decision_checkpoint = branch_decisions.len();

        let run_result = run_with_panic_boundary(&node.id, || -> Result<(), String> {
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
                    let payload = resolve_single_input_payload_for_execution(
                        node,
                        incoming,
                        &mut artifacts,
                        &artifact_retention,
                    )?;
                    let output_value = run_solve_operator(operator_id, payload)?;
                    validate_workflow_artifact_budget(
                        &format!("workflow node {} output", node.id),
                        &output_value,
                    )?;
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), output_value.clone());
                        output_budget_validated_artifacts.insert(key.clone());
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
                        resolve_first_available_input_payload_for_execution(
                            node,
                            incoming,
                            &mut artifacts,
                            &artifact_retention,
                        )?
                    } else if transform_operator_requires_port_map(operator_id) {
                        resolve_named_input_payloads_for_execution(
                            node,
                            incoming,
                            &mut artifacts,
                            &artifact_retention,
                        )?
                    } else {
                        resolve_single_input_payload_for_execution(
                            node,
                            incoming,
                            &mut artifacts,
                            &artifact_retention,
                        )?
                    };
                    let output_value = run_transform_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    let reuses_validated_artifact = operator_id == "transform.first_available"
                        && consumed_artifacts
                            .first()
                            .is_some_and(|key| output_budget_validated_artifacts.contains(key));
                    if !reuses_validated_artifact {
                        validate_workflow_artifact_budget(
                            &format!("workflow node {} output", node.id),
                            &output_value,
                        )?;
                    }
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), output_value.clone());
                        output_budget_validated_artifacts.insert(key.clone());
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
                    let payload = resolve_single_input_payload_for_execution(
                        node,
                        incoming,
                        &mut artifacts,
                        &artifact_retention,
                    )?;
                    let output_value = run_extract_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    validate_workflow_artifact_budget(
                        &format!("workflow node {} output", node.id),
                        &output_value,
                    )?;
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), output_value.clone());
                        output_budget_validated_artifacts.insert(key.clone());
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
                    let payload = resolve_single_input_payload_for_execution(
                        node,
                        incoming,
                        &mut artifacts,
                        &artifact_retention,
                    )?;
                    let output_value = run_export_operator(
                        operator_id,
                        payload,
                        node.config.clone().unwrap_or(Value::Null),
                    )?;
                    validate_workflow_artifact_budget(
                        &format!("workflow node {} output", node.id),
                        &output_value,
                    )?;
                    for output in &node.outputs {
                        let key = artifact_key(&node.id, &output.id);
                        artifacts.insert(key.clone(), output_value.clone());
                        output_budget_validated_artifacts.insert(key.clone());
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
                    for planned_edge in incoming {
                        let edge = planned_edge.edge();
                        let source_key = planned_edge.source_key();
                        let source_is_validated =
                            output_budget_validated_artifacts.contains(source_key);
                        let value = artifacts.get(source_key).cloned().ok_or_else(|| {
                            format!(
                                "workflow output node {} could not read {}.{}",
                                node.id, edge.from.node, edge.from.port
                            )
                        })?;
                        let key = artifact_key(&node.id, &edge.to.port);
                        artifacts.insert(key.clone(), value);
                        if source_is_validated {
                            output_budget_validated_artifacts.insert(key.clone());
                        }
                        produced_artifacts.push(key.clone());
                        artifact_lineage.push(WorkflowArtifactLineage {
                            artifact_key: key,
                            node_id: node.id.clone(),
                            port_id: edge.to.port.clone(),
                            source_artifacts: vec![source_key.to_owned()],
                        });
                    }
                }
                WorkflowNodeKind::Condition => {
                    let payload = resolve_single_input_payload_for_execution(
                        node,
                        incoming,
                        &mut artifacts,
                        &artifact_retention,
                    )?;
                    let source_is_validated = consumed_artifacts
                        .first()
                        .is_some_and(|key| output_budget_validated_artifacts.contains(key));
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
                    if source_is_validated {
                        output_budget_validated_artifacts.insert(key.clone());
                    }
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
            Ok(())
        });

        if let Err(error) = run_result {
            for key in &produced_artifacts {
                artifacts.remove(key);
                output_budget_validated_artifacts.remove(key);
            }
            artifact_lineage.truncate(artifact_lineage_checkpoint);
            branch_decisions.truncate(branch_decision_checkpoint);

            if node_allows_skip_on_error(node.config.as_ref()) {
                remove_releasable_artifacts(
                    artifact_retention.finish_node(incoming, &[]),
                    &mut artifacts,
                    &mut output_budget_validated_artifacts,
                );
                ordered_failed.push(node.id.clone());
                node_runs.push(WorkflowNodeRunTrace {
                    node_id: node.id.clone(),
                    kind: node.kind,
                    operator_id: node.operator_id.clone(),
                    status: WorkflowNodeRunStatus::Failed,
                    consumed_artifacts,
                    produced_artifacts: Vec::new(),
                    error_message: Some(error.clone()),
                });
                progress_events.push(workflow_progress_event(
                    JobStatus::Partitioning,
                    (ordered_completed.len() + ordered_skipped.len() + ordered_failed.len()) as f32
                        / total_nodes,
                    format!("recovered node {} failure: {}", node.id, error),
                    Some(&node.id),
                    Some(node.kind),
                ));
                continue;
            }
            return Err(format!("workflow node {} failed: {}", node.id, error));
        }

        remove_releasable_artifacts(
            artifact_retention.finish_node(incoming, &produced_artifacts),
            &mut artifacts,
            &mut output_budget_validated_artifacts,
        );
        ordered_completed.push(node.id.clone());
        node_runs.push(WorkflowNodeRunTrace {
            node_id: node.id.clone(),
            kind: node.kind,
            operator_id: node.operator_id.clone(),
            status: WorkflowNodeRunStatus::Completed,
            consumed_artifacts,
            produced_artifacts,
            error_message: None,
        });
        progress_events.push(workflow_progress_event(
            JobStatus::Solving,
            (ordered_completed.len() + ordered_skipped.len() + ordered_failed.len()) as f32
                / total_nodes,
            format!("completed node {}", node.id),
            Some(&node.id),
            Some(node.kind),
        ));
    }

    progress_events.push(workflow_progress_event(
        JobStatus::Postprocessing,
        1.0,
        format!("validated {} output node(s)", graph.output_nodes.len()),
        None,
        None,
    ));
    progress_events.push(workflow_progress_event(
        JobStatus::Completed,
        1.0,
        format!("workflow {} completed", graph.id),
        None,
        None,
    ));

    Ok(WorkflowGraphRunResult {
        workflow_id: graph.id,
        completed_nodes: ordered_completed,
        skipped_nodes: ordered_skipped,
        failed_nodes: ordered_failed,
        progress_events,
        branch_decisions,
        node_runs,
        artifact_lineage,
        artifacts,
    })
}

fn remove_releasable_artifacts(
    keys: Vec<String>,
    artifacts: &mut BTreeMap<String, Value>,
    output_budget_validated_artifacts: &mut HashSet<String>,
) {
    for key in keys {
        artifacts.remove(&key);
        output_budget_validated_artifacts.remove(&key);
    }
}

fn resolve_single_input_payload_for_execution(
    node: &kyuubiki_protocol::WorkflowNode,
    incoming: &[WorkflowPlannedEdge<'_>],
    artifacts: &mut BTreeMap<String, Value>,
    retention: &WorkflowArtifactRetention,
) -> Result<Value, String> {
    let planned_edge = incoming.first().ok_or_else(|| {
        format!(
            "workflow node {} requires at least one input artifact in the first executor",
            node.id
        )
    })?;
    if let Some(value) = retention.take_if_last_ephemeral(planned_edge.source_key(), artifacts) {
        return Ok(value);
    }
    artifacts
        .get(planned_edge.source_key())
        .cloned()
        .ok_or_else(|| {
            let edge = planned_edge.edge();
            format!(
                "workflow node {} could not resolve input from {}.{}",
                node.id, edge.from.node, edge.from.port
            )
        })
}

fn resolve_first_available_input_payload_for_execution(
    node: &kyuubiki_protocol::WorkflowNode,
    incoming: &[WorkflowPlannedEdge<'_>],
    artifacts: &mut BTreeMap<String, Value>,
    retention: &WorkflowArtifactRetention,
) -> Result<Value, String> {
    for edge in incoming {
        if !artifacts.contains_key(edge.source_key()) {
            continue;
        }
        if let Some(value) = retention.take_if_last_ephemeral(edge.source_key(), artifacts) {
            return Ok(value);
        }
        return artifacts.get(edge.source_key()).cloned().ok_or_else(|| {
            format!(
                "workflow node {} requires at least one resolved input artifact",
                node.id
            )
        });
    }
    Err(format!(
        "workflow node {} requires at least one resolved input artifact",
        node.id
    ))
}

fn resolve_named_input_payloads_for_execution(
    node: &kyuubiki_protocol::WorkflowNode,
    incoming: &[WorkflowPlannedEdge<'_>],
    artifacts: &mut BTreeMap<String, Value>,
    retention: &WorkflowArtifactRetention,
) -> Result<Value, String> {
    if incoming.is_empty() {
        return Err(format!(
            "workflow node {} requires at least one resolved named input artifact",
            node.id
        ));
    }
    let mut payload = serde_json::Map::new();
    for planned_edge in incoming {
        let edge = planned_edge.edge();
        let artifact = retention
            .take_if_last_ephemeral(planned_edge.source_key(), artifacts)
            .or_else(|| artifacts.get(planned_edge.source_key()).cloned())
            .ok_or_else(|| {
                format!(
                    "workflow node {} could not resolve input from {}.{}",
                    node.id, edge.from.node, edge.from.port
                )
            })?;
        payload.insert(edge.to.port.clone(), artifact);
    }
    Ok(Value::Object(payload))
}
