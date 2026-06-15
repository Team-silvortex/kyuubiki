use crate::error::{SdkError, SdkResult};
use crate::workflow_contracts::{WorkflowGraphDefinition, WorkflowGraphNode};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowOutputArtifact {
    pub key: String,
    pub node_id: String,
    pub port_id: String,
    pub artifact_type: String,
    pub dataset_value: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowOutputManifest {
    pub graph_id: String,
    pub graph_version: String,
    pub outputs: Vec<WorkflowOutputArtifact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowValidatedArtifacts {
    pub graph_id: String,
    pub graph_version: String,
    pub manifest: WorkflowOutputManifest,
    pub workflow_runtime: WorkflowRuntimeSnapshot,
    pub artifacts: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRuntimeSnapshot {
    pub workflow_id: Option<String>,
    pub run_id: Option<String>,
    pub status: Option<String>,
    pub current_node: Option<String>,
    pub completed_nodes: Vec<String>,
    pub progress_events: Vec<Value>,
    pub failure: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowProgressSnapshot {
    pub index: usize,
    pub job_id: Option<String>,
    pub status: Option<String>,
    pub progress: Option<Value>,
    pub current_node: Option<String>,
    pub completed_nodes: Vec<String>,
    pub progress_events: Vec<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowProgression {
    pub snapshots: Vec<WorkflowProgressSnapshot>,
    pub latest: Option<Value>,
}

pub fn build_workflow_output_manifest(graph: &WorkflowGraphDefinition) -> SdkResult<WorkflowOutputManifest> {
    graph.validate()?;
    let outputs = graph
        .output_nodes
        .iter()
        .filter_map(|node_id| graph.nodes.iter().find(|node| &node.id == node_id))
        .flat_map(manifest_outputs_for_node)
        .collect::<Vec<_>>();
    Ok(WorkflowOutputManifest {
        graph_id: graph.id.clone(),
        graph_version: graph.version.clone(),
        outputs,
    })
}

pub fn validate_workflow_result_against_graph(
    graph: &WorkflowGraphDefinition,
    payload: &Value,
) -> SdkResult<WorkflowValidatedArtifacts> {
    let manifest = build_workflow_output_manifest(graph)?;
    let artifacts = extract_artifacts(payload)?;
    let mut normalized = serde_json::Map::new();
    let mut errors = Vec::new();

    for output in &manifest.outputs {
        let artifact = find_artifact(artifacts, output);
        match artifact {
            Some(artifact) => {
                if let Some(artifact_type) = artifact.get("artifact_type").and_then(Value::as_str) {
                    if artifact_type != output.artifact_type {
                        errors.push(format!(
                            "workflow result artifact {:?} has mismatched artifact_type",
                            output.key
                        ));
                    }
                }
                if let Some(expected) = output.dataset_value.as_deref() {
                    if let Some(actual) = artifact.get("dataset_value").and_then(Value::as_str) {
                        if actual != expected {
                            errors.push(format!(
                                "workflow result artifact {:?} has mismatched dataset_value",
                                output.key
                            ));
                        }
                    }
                }
                normalized.insert(output.key.clone(), artifact.clone());
            }
            None if output.required => errors.push(format!(
                "workflow result is missing required artifact for output {:?}",
                output.key
            )),
            None => {}
        }
    }

    if errors.is_empty() {
        Ok(WorkflowValidatedArtifacts {
            graph_id: manifest.graph_id.clone(),
            graph_version: manifest.graph_version.clone(),
            manifest,
            workflow_runtime: normalize_workflow_runtime(payload)?,
            artifacts: normalized,
        })
    } else {
        Err(SdkError::Validation { errors })
    }
}

pub fn normalize_workflow_runtime(payload: &Value) -> SdkResult<WorkflowRuntimeSnapshot> {
    let runtime = extract_runtime_payload(payload)?;
    let workflow_id = read_optional_string(runtime, "workflow_id", "workflow runtime workflow_id must be a string")?;
    let run_id = read_optional_string(runtime, "run_id", "workflow runtime run_id must be a string")?;
    let status = read_optional_string(runtime, "status", "workflow runtime status must be a string")?;
    let current_node = read_optional_string(runtime, "current_node", "workflow runtime current_node must be a string")?;
    let completed_nodes = read_string_list(runtime, "completed_nodes", "workflow runtime completed_nodes must be a list")?;
    let progress_events = read_value_list(runtime, "progress_events", "workflow runtime progress_events must be a list")?;
    let failure = match runtime.get("failure") {
        Some(value) if !value.is_object() => {
            return Err(SdkError::Validation {
                errors: vec!["workflow runtime failure must be an object".into()],
            })
        }
        Some(value) => Some(value.clone()),
        None => None,
    };
    Ok(WorkflowRuntimeSnapshot {
        workflow_id,
        run_id,
        status,
        current_node,
        completed_nodes,
        progress_events,
        failure,
    })
}

pub fn normalize_workflow_progression(
    history: &[Value],
    result_payload: Option<&Value>,
) -> SdkResult<WorkflowProgression> {
    let mut snapshots = Vec::new();
    for (index, payload) in history.iter().enumerate() {
        let Some(job) = payload.get("job").and_then(Value::as_object) else {
            continue;
        };
        let current_node = read_optional_string(job, "current_node", "workflow progression current_node must be a string")?;
        let completed_nodes = read_string_list(job, "completed_nodes", "workflow progression completed_nodes must be a list")?;
        let progress_events = read_value_list(job, "progress_events", "workflow progression progress_events must be a list")?;
        snapshots.push(WorkflowProgressSnapshot {
            index,
            job_id: read_optional_string(job, "job_id", "workflow progression job_id must be a string")?,
            status: read_optional_string(job, "status", "workflow progression status must be a string")?,
            progress: job.get("progress").cloned(),
            current_node,
            completed_nodes,
            progress_events,
        });
    }
    let latest = match result_payload {
        Some(payload) => Some(serde_json::to_value(normalize_workflow_runtime(payload)?)?),
        None => snapshots.last().map(serde_json::to_value).transpose()?,
    };
    Ok(WorkflowProgression { snapshots, latest })
}

fn manifest_outputs_for_node(node: &WorkflowGraphNode) -> Vec<WorkflowOutputArtifact> {
    node.inputs
        .iter()
        .map(|port| WorkflowOutputArtifact {
            key: format!("{}.{}", node.id, port.id),
            node_id: node.id.clone(),
            port_id: port.id.clone(),
            artifact_type: port.artifact_type.clone(),
            dataset_value: port.dataset_value.clone(),
            required: port.required.unwrap_or(true),
        })
        .collect()
}

fn extract_artifacts<'a>(payload: &'a Value) -> SdkResult<&'a serde_json::Map<String, Value>> {
    if let Some(artifacts) = payload.get("artifacts").and_then(Value::as_object) {
        return Ok(artifacts);
    }
    if let Some(artifacts) = payload
        .get("result")
        .and_then(Value::as_object)
        .and_then(|result| result.get("artifacts"))
        .and_then(Value::as_object)
    {
        return Ok(artifacts);
    }
    Err(SdkError::Validation {
        errors: vec!["workflow result payload must include an 'artifacts' object".into()],
    })
}

fn extract_runtime_payload<'a>(payload: &'a Value) -> SdkResult<&'a serde_json::Map<String, Value>> {
    let object = payload.as_object().ok_or_else(|| SdkError::Validation {
        errors: vec!["workflow result payload must be an object".into()],
    })?;
    if let Some(result) = object.get("result").and_then(Value::as_object) {
        if result.contains_key("workflow_id")
            || result.contains_key("run_id")
            || result.contains_key("status")
            || result.contains_key("current_node")
            || result.contains_key("completed_nodes")
            || result.contains_key("progress_events")
            || result.contains_key("failure")
        {
            return Ok(result);
        }
    }
    Ok(object)
}

fn find_artifact<'a>(
    artifacts: &'a serde_json::Map<String, Value>,
    output: &WorkflowOutputArtifact,
) -> Option<&'a Value> {
    for key in [
        Some(output.key.as_str()),
        output.dataset_value.as_deref(),
        Some(output.artifact_type.as_str()),
    ] {
        if let Some(key) = key {
            if let Some(value) = artifacts.get(key) {
                return Some(value);
            }
        }
    }
    None
}

fn read_optional_string(
    runtime: &serde_json::Map<String, Value>,
    key: &str,
    error: &str,
) -> SdkResult<Option<String>> {
    match runtime.get(key) {
        Some(value) => match value.as_str() {
            Some(value) => Ok(Some(value.to_string())),
            None => Err(SdkError::Validation {
                errors: vec![error.into()],
            }),
        },
        None => Ok(None),
    }
}

fn read_string_list(
    runtime: &serde_json::Map<String, Value>,
    key: &str,
    error: &str,
) -> SdkResult<Vec<String>> {
    match runtime.get(key) {
        Some(value) => match value.as_array() {
            Some(items) => Ok(items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()),
            None => Err(SdkError::Validation {
                errors: vec![error.into()],
            }),
        },
        None => Ok(Vec::new()),
    }
}

fn read_value_list(
    runtime: &serde_json::Map<String, Value>,
    key: &str,
    error: &str,
) -> SdkResult<Vec<Value>> {
    match runtime.get(key) {
        Some(value) => match value.as_array() {
            Some(items) => Ok(items.clone()),
            None => Err(SdkError::Validation {
                errors: vec![error.into()],
            }),
        },
        None => Ok(Vec::new()),
    }
}
