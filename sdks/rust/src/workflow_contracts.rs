use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub const WORKFLOW_DATASET_SCHEMA_VERSION: &str = "kyuubiki.workflow-dataset/v1";
pub const WORKFLOW_GRAPH_SCHEMA_VERSION: &str = "kyuubiki.workflow-graph/v1";
pub const WORKFLOW_DISPATCH_POLICIES: &[&str] = &[
    "orchestra_only",
    "central_fetch",
    "direct_mesh",
    "local_only",
];

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
