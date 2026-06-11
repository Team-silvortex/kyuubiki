use kyuubiki_protocol::WorkflowGraph;
use std::collections::HashMap;

pub fn validate_workflow_dataset_contract(graph: &WorkflowGraph) -> Result<(), String> {
    let Some(contract) = graph.dataset_contract.as_ref() else {
        return Ok(());
    };

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
