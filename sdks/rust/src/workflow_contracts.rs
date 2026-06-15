use crate::error::{SdkError, SdkResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub const WORKFLOW_DATASET_SCHEMA_VERSION: &str = "kyuubiki.workflow-dataset/v1";
pub const WORKFLOW_GRAPH_SCHEMA_VERSION: &str = "kyuubiki.workflow-graph/v1";
pub const WORKFLOW_DISPATCH_POLICIES: &[&str] = &["orchestra_only", "central_fetch", "direct_mesh", "local_only"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSchemaRef {
    pub schema: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowAxis {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub semantic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowShape {
    #[serde(default)]
    pub axes: Vec<WorkflowAxis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDatasetValue {
    pub id: String,
    pub data_class: String,
    pub element_type: String,
    pub shape: WorkflowShape,
    #[serde(default)]
    pub semantic_type: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub schema_ref: Option<WorkflowSchemaRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDatasetContract {
    pub schema_version: String,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub values: Vec<WorkflowDatasetValue>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraphPort {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub artifact_type: String,
    #[serde(default)]
    pub required: Option<bool>,
    #[serde(default)]
    pub cardinality: Option<String>,
    #[serde(default)]
    pub dataset_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraphNode {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub operator_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub config: Option<Value>,
    #[serde(default)]
    pub cache_policy: Option<String>,
    #[serde(default)]
    pub placement_tags: Vec<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    pub inputs: Vec<WorkflowGraphPort>,
    pub outputs: Vec<WorkflowGraphPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNodePortRef {
    pub node: String,
    pub port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraphEdge {
    pub id: String,
    pub from: WorkflowNodePortRef,
    pub to: WorkflowNodePortRef,
    pub artifact_type: String,
    #[serde(default)]
    pub dataset_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowDefaults {
    #[serde(default)]
    pub cache_policy: Option<String>,
    #[serde(default)]
    pub orchestrated: Option<bool>,
    #[serde(default)]
    pub dispatch_policy: Option<String>,
    #[serde(default)]
    pub placement_tags: Vec<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOperatorFetchEntry {
    pub node_id: String,
    pub operator_id: String,
    #[serde(default)]
    pub package_ref: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub integrity: Option<String>,
    #[serde(default)]
    pub cache_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraphDefinition {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub dataset_contract: Option<WorkflowDatasetContract>,
    pub entry_nodes: Vec<String>,
    #[serde(default)]
    pub output_nodes: Vec<String>,
    #[serde(default)]
    pub defaults: Option<WorkflowDefaults>,
    #[serde(default)]
    pub dispatch_policy: Option<String>,
    #[serde(default)]
    pub operator_fetch_plan: Vec<WorkflowOperatorFetchEntry>,
    #[serde(default)]
    pub placement_tags: Vec<String>,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    pub nodes: Vec<WorkflowGraphNode>,
    pub edges: Vec<WorkflowGraphEdge>,
}

impl WorkflowDatasetContract {
    pub fn validate(&self) -> SdkResult<()> {
        let mut errors = Vec::new();
        if self.schema_version != WORKFLOW_DATASET_SCHEMA_VERSION {
            errors.push(format!("dataset.schema_version must be {WORKFLOW_DATASET_SCHEMA_VERSION:?}"));
        }
        require_string(&self.id, "dataset.id", &mut errors);
        require_string(&self.version, "dataset.version", &mut errors);
        if self.values.is_empty() {
            errors.push("dataset.values must contain at least 1 item".into());
        }
        let mut ids = HashSet::new();
        for (index, value) in self.values.iter().enumerate() {
            let path = format!("dataset.values[{index}]");
            require_string(&value.id, &format!("{path}.id"), &mut errors);
            if !value.id.is_empty() && !ids.insert(value.id.as_str()) {
                errors.push(format!("dataset.values contains duplicate id {:?}", value.id));
            }
            require_string(&value.data_class, &format!("{path}.data_class"), &mut errors);
            require_string(&value.element_type, &format!("{path}.element_type"), &mut errors);
            if !matches!(value.data_class.as_str(), "study_model" | "result" | "field" | "table" | "report" | "export" | "scalar" | "metadata") {
                errors.push(format!("{path}.data_class is invalid"));
            }
            if let Some(encoding) = &value.encoding {
                if !matches!(encoding.as_str(), "json" | "json_lines" | "f64_le" | "f32_le" | "i64_le" | "i32_le" | "u8") {
                    errors.push(format!("{path}.encoding is invalid"));
                }
            }
            let mut axis_ids = HashSet::new();
            for (axis_index, axis) in value.shape.axes.iter().enumerate() {
                let axis_path = format!("{path}.shape.axes[{axis_index}]");
                require_string(&axis.id, &format!("{axis_path}.id"), &mut errors);
                if !axis.id.is_empty() && !axis_ids.insert(axis.id.as_str()) {
                    errors.push(format!("{path}.shape.axes contains duplicate id {:?}", axis.id));
                }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(SdkError::Validation { errors }) }
    }
}

impl WorkflowGraphDefinition {
    pub fn validate(&self) -> SdkResult<()> {
        let mut errors = Vec::new();
        if self.schema_version != WORKFLOW_GRAPH_SCHEMA_VERSION {
            errors.push(format!("graph.schema_version must be {WORKFLOW_GRAPH_SCHEMA_VERSION:?}"));
        }
        require_string(&self.id, "graph.id", &mut errors);
        require_string(&self.name, "graph.name", &mut errors);
        require_string(&self.version, "graph.version", &mut errors);
        if self.entry_nodes.is_empty() {
            errors.push("graph.entry_nodes must contain at least 1 item".into());
        }
        if self.nodes.is_empty() {
            errors.push("graph.nodes must contain at least 1 item".into());
        }
        let dataset_ids = match &self.dataset_contract {
            Some(contract) => {
                if let Err(SdkError::Validation { errors: nested }) = contract.validate() {
                    errors.extend(nested.into_iter().map(|item| format!("graph.dataset_contract: {item}")));
                }
                contract.values.iter().map(|value| value.id.as_str()).collect::<HashSet<_>>()
            }
            None => HashSet::new(),
        };
        validate_dispatch_policy(self.dispatch_policy.as_deref(), "graph.dispatch_policy", &mut errors);
        validate_string_list(&self.placement_tags, "graph.placement_tags", &mut errors);
        validate_string_list(&self.required_capabilities, "graph.required_capabilities", &mut errors);
        validate_operator_fetch_plan(&self.operator_fetch_plan, "graph.operator_fetch_plan", &mut errors);
        if let Some(defaults) = &self.defaults {
            if let Some(cache_policy) = defaults.cache_policy.as_deref() {
                if !matches!(cache_policy, "ephemeral" | "cached" | "persisted") {
                    errors.push("graph.defaults.cache_policy is invalid".into());
                }
            }
            validate_dispatch_policy(defaults.dispatch_policy.as_deref(), "graph.defaults.dispatch_policy", &mut errors);
            validate_string_list(&defaults.placement_tags, "graph.defaults.placement_tags", &mut errors);
            validate_string_list(&defaults.required_capabilities, "graph.defaults.required_capabilities", &mut errors);
        }
        let mut node_ids = HashSet::new();
        let mut input_ports = HashMap::new();
        let mut output_ports = HashMap::new();
        for (index, node) in self.nodes.iter().enumerate() {
            let path = format!("graph.nodes[{index}]");
            require_string(&node.id, &format!("{path}.id"), &mut errors);
            require_string(&node.kind, &format!("{path}.kind"), &mut errors);
            if !node.id.is_empty() && !node_ids.insert(node.id.as_str()) {
                errors.push(format!("graph.nodes contains duplicate id {:?}", node.id));
            }
            if !matches!(node.kind.as_str(), "input" | "solve" | "transform" | "extract" | "export" | "condition" | "output") {
                errors.push(format!("{path}.kind is invalid"));
            }
            if matches!(node.kind.as_str(), "solve" | "transform" | "extract" | "export" | "condition") {
                require_option_string(node.operator_id.as_deref(), &format!("{path}.operator_id"), &mut errors);
            }
            validate_string_list(&node.placement_tags, &format!("{path}.placement_tags"), &mut errors);
            validate_string_list(&node.required_capabilities, &format!("{path}.required_capabilities"), &mut errors);
            collect_ports(&node.id, &node.inputs, &dataset_ids, &mut input_ports, &mut errors, &format!("{path}.inputs"));
            collect_ports(&node.id, &node.outputs, &dataset_ids, &mut output_ports, &mut errors, &format!("{path}.outputs"));
        }
        for (index, node_id) in self.entry_nodes.iter().enumerate() {
            if !node_ids.contains(node_id.as_str()) {
                errors.push(format!("graph.entry_nodes[{index}] references unknown node {:?}", node_id));
            }
        }
        for (index, node_id) in self.output_nodes.iter().enumerate() {
            if !node_ids.contains(node_id.as_str()) {
                errors.push(format!("graph.output_nodes[{index}] references unknown node {:?}", node_id));
            }
        }
        let mut edge_ids = HashSet::new();
        for (index, edge) in self.edges.iter().enumerate() {
            let path = format!("graph.edges[{index}]");
            require_string(&edge.id, &format!("{path}.id"), &mut errors);
            if !edge.id.is_empty() && !edge_ids.insert(edge.id.as_str()) {
                errors.push(format!("graph.edges contains duplicate id {:?}", edge.id));
            }
            require_string(&edge.artifact_type, &format!("{path}.artifact_type"), &mut errors);
            let source_key = (edge.from.node.as_str(), edge.from.port.as_str());
            let target_key = (edge.to.node.as_str(), edge.to.port.as_str());
            let source = output_ports.get(&source_key);
            let target = input_ports.get(&target_key);
            if source.is_none() {
                errors.push(format!("{path}.from references unknown output port {:?}", source_key));
            }
            if target.is_none() {
                errors.push(format!("{path}.to references unknown input port {:?}", target_key));
            }
            if let Some(source_port) = source {
                if source_port.artifact_type != edge.artifact_type {
                    errors.push(format!("{path}.artifact_type does not match source output port artifact_type"));
                }
            }
            if let Some(target_port) = target {
                if target_port.artifact_type != edge.artifact_type {
                    errors.push(format!("{path}.artifact_type does not match target input port artifact_type"));
                }
            }
            if let (Some(source_port), Some(target_port)) = (source, target) {
                if source_port.artifact_type != target_port.artifact_type {
                    errors.push(format!("{path} connects ports with mismatched artifact_type values"));
                }
            }
            if let Some(dataset_value) = edge.dataset_value.as_deref() {
                if !dataset_ids.is_empty() && !dataset_ids.contains(dataset_value) {
                    errors.push(format!("{path}.dataset_value {:?} is not declared in graph.dataset_contract", dataset_value));
                }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(SdkError::Validation { errors }) }
    }
}

fn collect_ports<'a>(
    node_id: &'a str,
    ports: &'a [WorkflowGraphPort],
    dataset_ids: &HashSet<&'a str>,
    bucket: &mut HashMap<(&'a str, &'a str), &'a WorkflowGraphPort>,
    errors: &mut Vec<String>,
    path: &str,
) {
    let mut ids = HashSet::new();
    for (index, port) in ports.iter().enumerate() {
        let port_path = format!("{path}[{index}]");
        require_string(&port.id, &format!("{port_path}.id"), errors);
        require_string(&port.artifact_type, &format!("{port_path}.artifact_type"), errors);
        if !port.id.is_empty() && !ids.insert(port.id.as_str()) {
            errors.push(format!("{path} contains duplicate port id {:?}", port.id));
        }
        if let Some(cardinality) = &port.cardinality {
            if !matches!(cardinality.as_str(), "one" | "many") {
                errors.push(format!("{port_path}.cardinality is invalid"));
            }
        }
        if let Some(dataset_value) = port.dataset_value.as_deref() {
            if !dataset_ids.is_empty() && !dataset_ids.contains(dataset_value) {
                errors.push(format!("{port_path}.dataset_value {:?} is not declared in graph.dataset_contract", dataset_value));
            }
        }
        if !node_id.is_empty() && !port.id.is_empty() {
            bucket.insert((node_id, port.id.as_str()), port);
        }
    }
}

fn require_string(value: &str, path: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{path} must be a non-empty string"));
    }
}

fn require_option_string(value: Option<&str>, path: &str, errors: &mut Vec<String>) {
    match value {
        Some(value) if !value.trim().is_empty() => {}
        _ => errors.push(format!("{path} must be a non-empty string")),
    }
}

fn validate_dispatch_policy(value: Option<&str>, path: &str, errors: &mut Vec<String>) {
    if let Some(value) = value {
        if !WORKFLOW_DISPATCH_POLICIES.contains(&value) {
            errors.push(format!("{path} must be one of {:?}", WORKFLOW_DISPATCH_POLICIES));
        }
    }
}

fn validate_string_list(values: &[String], path: &str, errors: &mut Vec<String>) {
    for (index, value) in values.iter().enumerate() {
        if value.trim().is_empty() {
            errors.push(format!("{path}[{index}] must be a non-empty string"));
        }
    }
}

fn validate_operator_fetch_plan(entries: &[WorkflowOperatorFetchEntry], path: &str, errors: &mut Vec<String>) {
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = format!("{path}[{index}]");
        require_string(&entry.node_id, &format!("{entry_path}.node_id"), errors);
        require_string(&entry.operator_id, &format!("{entry_path}.operator_id"), errors);
        validate_optional_non_empty(entry.package_ref.as_deref(), &format!("{entry_path}.package_ref"), errors);
        validate_optional_non_empty(entry.version.as_deref(), &format!("{entry_path}.version"), errors);
        validate_optional_non_empty(entry.integrity.as_deref(), &format!("{entry_path}.integrity"), errors);
        validate_optional_non_empty(entry.cache_scope.as_deref(), &format!("{entry_path}.cache_scope"), errors);
    }
}

fn validate_optional_non_empty(value: Option<&str>, path: &str, errors: &mut Vec<String>) {
    if let Some(value) = value {
        if value.trim().is_empty() {
            errors.push(format!("{path} must be a non-empty string"));
        }
    }
}
