use crate::workflow_executor::is_supported_workflow_operator;
use kyuubiki_protocol::{WorkflowGraph, WorkflowGraphRunRequest, WorkflowNodeKind};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const MAX_WORKFLOW_NODES: usize = 2048;
const MAX_WORKFLOW_EDGES: usize = 4096;
const MAX_PORTS_PER_NODE: usize = 32;
const MAX_ID_LEN: usize = 128;
const MAX_ARTIFACT_TYPE_LEN: usize = 160;
const MAX_NODE_CONFIG_JSON_NODES: usize = 20_000;
const MAX_INPUT_ARTIFACT_JSON_NODES: usize = 500_000;
const MAX_OUTPUT_ARTIFACT_JSON_NODES: usize = 500_000;
const MAX_WORKFLOW_JSON_DEPTH: usize = 64;
const MAX_WORKFLOW_JSON_STRING_LEN: usize = 1_000_000;
const MAX_OUTPUT_ARTIFACT_JSON_STRING_LEN: usize = 500_000;
const MAX_WORKFLOW_JSON_KEY_LEN: usize = 256;
const WORKFLOW_GRAPH_SCHEMA_VERSION: &str = "kyuubiki.workflow-graph/v1";

#[derive(Clone, Copy)]
struct JsonBudget {
    max_nodes: usize,
    max_depth: usize,
    max_string_len: usize,
    max_key_len: usize,
}

pub fn validate_workflow_security(request: &WorkflowGraphRunRequest) -> Result<(), String> {
    let graph = &request.graph;
    if graph.schema_version != WORKFLOW_GRAPH_SCHEMA_VERSION {
        return Err(format!(
            "workflow graph schema_version must be {WORKFLOW_GRAPH_SCHEMA_VERSION}"
        ));
    }
    validate_identifier("workflow id", &graph.id, true)?;
    if graph.nodes.is_empty() {
        return Err("workflow graph must contain at least one node".to_string());
    }
    if graph.nodes.len() > MAX_WORKFLOW_NODES {
        return Err(format!(
            "workflow graph exceeds node security budget: {} > {}",
            graph.nodes.len(),
            MAX_WORKFLOW_NODES
        ));
    }
    if graph.edges.len() > MAX_WORKFLOW_EDGES {
        return Err(format!(
            "workflow graph exceeds edge security budget: {} > {}",
            graph.edges.len(),
            MAX_WORKFLOW_EDGES
        ));
    }

    let node_ports = validate_nodes(graph)?;
    validate_edges(graph, &node_ports)?;
    validate_entry_and_output_nodes(graph, &node_ports)?;
    validate_acyclic_graph(graph, &node_ports)?;
    validate_input_artifacts(request, &node_ports)
}

pub fn validate_workflow_artifact_budget(label: &str, artifact: &Value) -> Result<(), String> {
    validate_json_budget(
        label,
        artifact,
        JsonBudget {
            max_nodes: MAX_OUTPUT_ARTIFACT_JSON_NODES,
            max_depth: MAX_WORKFLOW_JSON_DEPTH,
            max_string_len: MAX_OUTPUT_ARTIFACT_JSON_STRING_LEN,
            max_key_len: MAX_WORKFLOW_JSON_KEY_LEN,
        },
    )
}

fn validate_nodes(graph: &WorkflowGraph) -> Result<HashMap<String, NodePorts>, String> {
    let mut node_ports = HashMap::new();
    for node in &graph.nodes {
        validate_identifier("workflow node id", &node.id, false)?;
        if node_ports.contains_key(&node.id) {
            return Err(format!("duplicate workflow node id {}", node.id));
        }
        validate_operator(&node.kind, node.operator_id.as_deref(), &node.id)?;
        if node.inputs.len() > MAX_PORTS_PER_NODE || node.outputs.len() > MAX_PORTS_PER_NODE {
            return Err(format!(
                "workflow node {} exceeds per-node port security budget",
                node.id
            ));
        }
        if let Some(config) = &node.config {
            validate_json_budget(
                &format!("workflow node {} config", node.id),
                config,
                JsonBudget {
                    max_nodes: MAX_NODE_CONFIG_JSON_NODES,
                    max_depth: MAX_WORKFLOW_JSON_DEPTH,
                    max_string_len: MAX_WORKFLOW_JSON_STRING_LEN,
                    max_key_len: MAX_WORKFLOW_JSON_KEY_LEN,
                },
            )?;
        }

        let mut inputs = HashMap::new();
        let mut outputs = HashMap::new();
        for port in &node.inputs {
            validate_port(&node.id, &port.id, &port.artifact_type)?;
            if inputs
                .insert(port.id.clone(), port.artifact_type.clone())
                .is_some()
            {
                return Err(format!(
                    "workflow node {} has duplicate input port {}",
                    node.id, port.id
                ));
            }
        }
        for port in &node.outputs {
            validate_port(&node.id, &port.id, &port.artifact_type)?;
            if outputs
                .insert(port.id.clone(), port.artifact_type.clone())
                .is_some()
            {
                return Err(format!(
                    "workflow node {} has duplicate output port {}",
                    node.id, port.id
                ));
            }
        }
        node_ports.insert(
            node.id.clone(),
            NodePorts {
                kind: node.kind,
                inputs,
                outputs,
            },
        );
    }
    Ok(node_ports)
}

fn validate_edges(
    graph: &WorkflowGraph,
    node_ports: &HashMap<String, NodePorts>,
) -> Result<(), String> {
    let mut edge_ids = HashSet::new();
    let mut target_ports = HashSet::new();
    for edge in &graph.edges {
        validate_identifier("workflow edge id", &edge.id, true)?;
        validate_artifact_type("workflow edge artifact_type", &edge.artifact_type)?;
        if !edge_ids.insert(edge.id.clone()) {
            return Err(format!("duplicate workflow edge id {}", edge.id));
        }
        let from = node_ports.get(&edge.from.node).ok_or_else(|| {
            format!(
                "workflow edge {} references missing source node {}",
                edge.id, edge.from.node
            )
        })?;
        let source_artifact_type = from.outputs.get(&edge.from.port).ok_or_else(|| {
            format!(
                "workflow edge {} references missing source port {}.{}",
                edge.id, edge.from.node, edge.from.port
            )
        })?;
        if source_artifact_type != &edge.artifact_type {
            return Err(format!(
                "workflow edge {} artifact_type {} does not match source port {}.{} type {}",
                edge.id, edge.artifact_type, edge.from.node, edge.from.port, source_artifact_type
            ));
        }
        let to = node_ports.get(&edge.to.node).ok_or_else(|| {
            format!(
                "workflow edge {} references missing target node {}",
                edge.id, edge.to.node
            )
        })?;
        let target_artifact_type = to.inputs.get(&edge.to.port).ok_or_else(|| {
            format!(
                "workflow edge {} references missing target port {}.{}",
                edge.id, edge.to.node, edge.to.port
            )
        })?;
        if target_artifact_type != &edge.artifact_type {
            return Err(format!(
                "workflow edge {} artifact_type {} does not match target port {}.{} type {}",
                edge.id, edge.artifact_type, edge.to.node, edge.to.port, target_artifact_type
            ));
        }
        if !target_ports.insert((edge.to.node.clone(), edge.to.port.clone())) {
            return Err(format!(
                "workflow edge {} creates duplicate incoming edge for target port {}.{}",
                edge.id, edge.to.node, edge.to.port
            ));
        }
    }
    Ok(())
}

fn validate_entry_and_output_nodes(
    graph: &WorkflowGraph,
    node_ports: &HashMap<String, NodePorts>,
) -> Result<(), String> {
    for node_id in &graph.entry_nodes {
        validate_identifier("workflow entry/output node id", node_id, false)?;
        let Some(node) = node_ports.get(node_id) else {
            return Err(format!(
                "workflow entry/output node {} is not defined",
                node_id
            ));
        };
        if node.kind != WorkflowNodeKind::Input {
            return Err(format!(
                "workflow entry node {node_id} must be an input node"
            ));
        }
    }
    for node_id in &graph.output_nodes {
        validate_identifier("workflow entry/output node id", node_id, false)?;
        let Some(node) = node_ports.get(node_id) else {
            return Err(format!(
                "workflow entry/output node {} is not defined",
                node_id
            ));
        };
        if node.kind != WorkflowNodeKind::Output {
            return Err(format!(
                "workflow output node {node_id} must be an output node"
            ));
        }
    }
    Ok(())
}

fn validate_acyclic_graph(
    graph: &WorkflowGraph,
    node_ports: &HashMap<String, NodePorts>,
) -> Result<(), String> {
    let mut incoming_counts = node_ports
        .keys()
        .map(|node_id| (node_id.clone(), 0usize))
        .collect::<HashMap<_, _>>();
    let mut outgoing = node_ports
        .keys()
        .map(|node_id| (node_id.clone(), Vec::<String>::new()))
        .collect::<HashMap<_, _>>();

    for edge in &graph.edges {
        *incoming_counts
            .get_mut(&edge.to.node)
            .expect("edge target should be validated") += 1;
        outgoing
            .get_mut(&edge.from.node)
            .expect("edge source should be validated")
            .push(edge.to.node.clone());
    }

    let mut ready = incoming_counts
        .iter()
        .filter_map(|(node_id, count)| {
            if *count == 0 {
                Some(node_id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let mut visited = 0usize;

    while let Some(node_id) = ready.pop() {
        visited += 1;
        for target in outgoing.remove(&node_id).unwrap_or_default() {
            let count = incoming_counts
                .get_mut(&target)
                .expect("edge target should be tracked");
            *count -= 1;
            if *count == 0 {
                ready.push(target);
            }
        }
    }

    if visited != node_ports.len() {
        let mut cyclic_nodes = incoming_counts
            .into_iter()
            .filter_map(|(node_id, count)| if count > 0 { Some(node_id) } else { None })
            .collect::<Vec<_>>();
        cyclic_nodes.sort();
        return Err(format!(
            "workflow graph must be acyclic; cycle involves {}",
            cyclic_nodes.join(", ")
        ));
    }
    Ok(())
}

fn validate_input_artifacts(
    request: &WorkflowGraphRunRequest,
    node_ports: &HashMap<String, NodePorts>,
) -> Result<(), String> {
    for (node_id, artifact) in &request.input_artifacts {
        validate_identifier("workflow input artifact node id", node_id, false)?;
        match node_ports.get(node_id) {
            Some(NodePorts {
                kind: WorkflowNodeKind::Input,
                ..
            }) => {}
            Some(_) => {
                return Err(format!(
                    "workflow input artifact {} must target an input node",
                    node_id
                ));
            }
            None => {
                return Err(format!(
                    "workflow input artifact {} targets an unknown node",
                    node_id
                ));
            }
        }
        validate_json_budget(
            &format!("workflow input artifact {node_id}"),
            artifact,
            JsonBudget {
                max_nodes: MAX_INPUT_ARTIFACT_JSON_NODES,
                max_depth: MAX_WORKFLOW_JSON_DEPTH,
                max_string_len: MAX_WORKFLOW_JSON_STRING_LEN,
                max_key_len: MAX_WORKFLOW_JSON_KEY_LEN,
            },
        )?;
    }
    Ok(())
}

fn validate_operator(
    kind: &WorkflowNodeKind,
    operator_id: Option<&str>,
    node_id: &str,
) -> Result<(), String> {
    match kind {
        WorkflowNodeKind::Solve
        | WorkflowNodeKind::Transform
        | WorkflowNodeKind::Extract
        | WorkflowNodeKind::Export => {
            let operator_id = operator_id
                .ok_or_else(|| format!("workflow node {node_id} is missing operator_id"))?;
            validate_identifier("workflow operator_id", operator_id, true)?;
            if !is_supported_workflow_operator(operator_id) {
                return Err(format!(
                    "workflow node {node_id} uses unsupported operator {operator_id}"
                ));
            }
        }
        WorkflowNodeKind::Input | WorkflowNodeKind::Condition | WorkflowNodeKind::Output => {
            if let Some(operator_id) = operator_id {
                validate_identifier("workflow operator_id", operator_id, true)?;
            }
        }
    }
    Ok(())
}

fn validate_port(node_id: &str, port_id: &str, artifact_type: &str) -> Result<(), String> {
    validate_identifier(&format!("workflow node {node_id} port id"), port_id, false)?;
    validate_artifact_type(
        &format!("workflow node {node_id} port artifact_type"),
        artifact_type,
    )
}

fn validate_identifier(label: &str, value: &str, allow_dot: bool) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_ID_LEN {
        return Err(format!("{label} length must be between 1 and {MAX_ID_LEN}"));
    }
    let valid = value.chars().all(|entry| {
        entry.is_ascii_alphanumeric() || entry == '_' || entry == '-' || (allow_dot && entry == '.')
    });
    if !valid {
        return Err(format!("{label} contains unsupported characters"));
    }
    Ok(())
}

fn validate_artifact_type(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_ARTIFACT_TYPE_LEN {
        return Err(format!(
            "{label} length must be between 1 and {MAX_ARTIFACT_TYPE_LEN}"
        ));
    }
    let valid = value.chars().all(|entry| {
        entry.is_ascii_alphanumeric()
            || entry == '_'
            || entry == '-'
            || entry == '.'
            || entry == '/'
    });
    if !valid {
        return Err(format!("{label} contains unsupported characters"));
    }
    Ok(())
}

fn validate_json_budget(label: &str, value: &Value, budget: JsonBudget) -> Result<(), String> {
    let mut node_count = 0;
    validate_json_budget_inner(label, value, 0, &mut node_count, budget)
}

fn validate_json_budget_inner(
    label: &str,
    value: &Value,
    depth: usize,
    node_count: &mut usize,
    budget: JsonBudget,
) -> Result<(), String> {
    if depth > budget.max_depth {
        return Err(format!(
            "{label} exceeds JSON depth security budget: {} > {}",
            depth, budget.max_depth
        ));
    }
    *node_count += 1;
    if *node_count > budget.max_nodes {
        return Err(format!(
            "{label} exceeds JSON node security budget: {} > {}",
            *node_count, budget.max_nodes
        ));
    }

    match value {
        Value::String(text) => validate_json_text(label, "string", text, budget.max_string_len)?,
        Value::Array(items) => {
            for item in items {
                validate_json_budget_inner(label, item, depth + 1, node_count, budget)?;
            }
        }
        Value::Object(object) => {
            for (key, item) in object {
                validate_json_text(label, "object key", key, budget.max_key_len)?;
                validate_json_budget_inner(label, item, depth + 1, node_count, budget)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn validate_json_text(label: &str, kind: &str, value: &str, max_len: usize) -> Result<(), String> {
    if value.len() > max_len {
        return Err(format!(
            "{label} {kind} exceeds length security budget: {} > {}",
            value.len(),
            max_len
        ));
    }
    if value.contains('\0') {
        return Err(format!("{label} {kind} contains NUL characters"));
    }
    Ok(())
}

struct NodePorts {
    kind: WorkflowNodeKind,
    inputs: HashMap<String, String>,
    outputs: HashMap<String, String>,
}
