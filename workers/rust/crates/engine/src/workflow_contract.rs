use kyuubiki_protocol::WorkflowGraph;
use std::collections::{HashMap, HashSet};

pub fn validate_workflow_dataset_contract(graph: &WorkflowGraph) -> Result<(), String> {
    let Some(contract) = graph.dataset_contract.as_ref() else {
        return Ok(());
    };

    validate_dataset_contract_identity(graph.id.as_str(), contract)?;
    validate_dataset_value_ids(graph.id.as_str(), contract)?;

    let value_map = contract
        .values
        .iter()
        .map(|value| (value.id.as_str(), value))
        .collect::<HashMap<_, _>>();

    for node in &graph.nodes {
        for port in node.inputs.iter().chain(node.outputs.iter()) {
            let Some(dataset_value) = port.dataset_value.as_deref() else {
                continue;
            };
            let value = value_map.get(dataset_value).ok_or_else(|| {
                format!(
                    "workflow port {}.{} references unknown dataset value {}",
                    node.id, port.id, dataset_value
                )
            })?;

            if let Some(semantic_type) = value.semantic_type.as_deref() {
                if semantic_type != port.artifact_type {
                    return Err(format!(
                        "workflow port {}.{} declares artifact_type {} but dataset value {} uses semantic_type {}",
                        node.id, port.id, port.artifact_type, dataset_value, semantic_type
                    ));
                }
            }
        }
    }

    for edge in &graph.edges {
        let from_node = graph
            .nodes
            .iter()
            .find(|node| node.id == edge.from.node)
            .ok_or_else(|| {
                format!(
                    "workflow edge {} references unknown from node {}",
                    edge.id, edge.from.node
                )
            })?;
        let to_node = graph
            .nodes
            .iter()
            .find(|node| node.id == edge.to.node)
            .ok_or_else(|| {
                format!(
                    "workflow edge {} references unknown to node {}",
                    edge.id, edge.to.node
                )
            })?;
        let from_port = from_node
            .outputs
            .iter()
            .find(|port| port.id == edge.from.port)
            .ok_or_else(|| {
                format!(
                    "workflow edge {} references unknown output port {}.{}",
                    edge.id, edge.from.node, edge.from.port
                )
            })?;
        let to_port = to_node
            .inputs
            .iter()
            .find(|port| port.id == edge.to.port)
            .ok_or_else(|| {
                format!(
                    "workflow edge {} references unknown input port {}.{}",
                    edge.id, edge.to.node, edge.to.port
                )
            })?;

        if from_port.artifact_type != edge.artifact_type {
            return Err(format!(
                "workflow edge {} artifact_type {} does not match from port {}.{} artifact_type {}",
                edge.id,
                edge.artifact_type,
                edge.from.node,
                edge.from.port,
                from_port.artifact_type
            ));
        }
        if to_port.artifact_type != edge.artifact_type {
            return Err(format!(
                "workflow edge {} artifact_type {} does not match to port {}.{} artifact_type {}",
                edge.id, edge.artifact_type, edge.to.node, edge.to.port, to_port.artifact_type
            ));
        }

        let referenced_dataset_value = edge
            .dataset_value
            .as_deref()
            .or(from_port.dataset_value.as_deref())
            .or(to_port.dataset_value.as_deref());

        if let Some(dataset_value) = referenced_dataset_value {
            let value = value_map.get(dataset_value).ok_or_else(|| {
                format!(
                    "workflow edge {} references unknown dataset value {}",
                    edge.id, dataset_value
                )
            })?;

            if let Some(from_dataset_value) = from_port.dataset_value.as_deref() {
                if from_dataset_value != dataset_value {
                    return Err(format!(
                        "workflow edge {} dataset value {} does not match from port dataset value {}",
                        edge.id, dataset_value, from_dataset_value
                    ));
                }
            }
            if let Some(to_dataset_value) = to_port.dataset_value.as_deref() {
                if to_dataset_value != dataset_value {
                    return Err(format!(
                        "workflow edge {} dataset value {} does not match to port dataset value {}",
                        edge.id, dataset_value, to_dataset_value
                    ));
                }
            }

            if let Some(semantic_type) = value.semantic_type.as_deref() {
                if semantic_type != edge.artifact_type {
                    return Err(format!(
                        "workflow edge {} artifact_type {} does not match dataset value {} semantic_type {}",
                        edge.id, edge.artifact_type, dataset_value, semantic_type
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_dataset_contract_identity(
    graph_id: &str,
    contract: &kyuubiki_protocol::WorkflowDatasetContract,
) -> Result<(), String> {
    if contract.id.trim().is_empty() {
        return Err(format!(
            "workflow {graph_id} dataset contract has an empty id"
        ));
    }
    if contract.version.trim().is_empty() {
        return Err(format!(
            "workflow {graph_id} dataset contract has an empty version"
        ));
    }
    Ok(())
}

fn validate_dataset_value_ids(
    graph_id: &str,
    contract: &kyuubiki_protocol::WorkflowDatasetContract,
) -> Result<(), String> {
    let mut value_ids = HashSet::new();
    for value in &contract.values {
        if value.id.trim().is_empty() {
            return Err(format!(
                "workflow {graph_id} dataset contract contains an empty dataset value id"
            ));
        }
        if !value_ids.insert(value.id.as_str()) {
            return Err(format!(
                "workflow {graph_id} dataset contract contains duplicate dataset value id {}",
                value.id
            ));
        }
        validate_dataset_value_metadata(graph_id, value)?;
    }
    Ok(())
}

fn validate_dataset_value_metadata(
    graph_id: &str,
    value: &kyuubiki_protocol::WorkflowDatasetValueInfo,
) -> Result<(), String> {
    if !kyuubiki_protocol::WORKFLOW_DATASET_DATA_CLASSES.contains(&value.data_class.as_str()) {
        return Err(format!(
            "workflow {graph_id} dataset value {} uses unsupported data_class {}",
            value.id, value.data_class
        ));
    }
    if value.element_type.trim().is_empty() {
        return Err(format!(
            "workflow {graph_id} dataset value {} has an empty element_type",
            value.id
        ));
    }
    if value
        .semantic_type
        .as_deref()
        .is_some_and(|semantic_type| semantic_type.trim().is_empty())
    {
        return Err(format!(
            "workflow {graph_id} dataset value {} has an empty semantic_type",
            value.id
        ));
    }
    if value
        .unit
        .as_deref()
        .is_some_and(|unit| unit.trim().is_empty())
    {
        return Err(format!(
            "workflow {graph_id} dataset value {} has an empty unit",
            value.id
        ));
    }
    if let Some(schema_ref) = &value.schema_ref {
        if schema_ref.schema.trim().is_empty() || schema_ref.version.trim().is_empty() {
            return Err(format!(
                "workflow {graph_id} dataset value {} has an empty schema_ref",
                value.id
            ));
        }
    }
    validate_dataset_shape(graph_id, value)
}

fn validate_dataset_shape(
    graph_id: &str,
    value: &kyuubiki_protocol::WorkflowDatasetValueInfo,
) -> Result<(), String> {
    let mut axis_ids = HashSet::new();
    for axis in &value.shape.axes {
        if axis.id.trim().is_empty() {
            return Err(format!(
                "workflow {graph_id} dataset value {} has an empty shape axis id",
                value.id
            ));
        }
        if !axis_ids.insert(axis.id.as_str()) {
            return Err(format!(
                "workflow {graph_id} dataset value {} has duplicate shape axis id {}",
                value.id, axis.id
            ));
        }
        if axis
            .label
            .as_deref()
            .is_some_and(|label| label.trim().is_empty())
        {
            return Err(format!(
                "workflow {graph_id} dataset value {} has an empty shape axis label",
                value.id
            ));
        }
        if axis
            .semantic
            .as_deref()
            .is_some_and(|semantic| semantic.trim().is_empty())
        {
            return Err(format!(
                "workflow {graph_id} dataset value {} has an empty shape axis semantic",
                value.id
            ));
        }
    }
    Ok(())
}
