use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorKind {
    Solver,
    Transform,
    Extract,
    Export,
    WorkflowBridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorOrigin {
    BuiltIn,
    ExternalLocal,
    ExternalRemote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorSchemaRef {
    pub schema: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorValidationStatus {
    Verified,
    Partial,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPortDescriptor {
    pub id: String,
    pub artifact_type: String,
    pub description: String,
    #[serde(default)]
    pub dataset_value: Option<String>,
    #[serde(default)]
    pub schema_ref: Option<OperatorSchemaRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorValidationProfile {
    pub baseline_status: OperatorValidationStatus,
    #[serde(default)]
    pub baseline_cases: Vec<String>,
    #[serde(default)]
    pub smoke_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowDatasetEncoding {
    Json,
    JsonLines,
    F64Le,
    F32Le,
    I64Le,
    I32Le,
    U8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDatasetAxis {
    pub id: String,
    pub label: Option<String>,
    pub size: Option<u64>,
    pub semantic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkflowDatasetShape {
    #[serde(default)]
    pub axes: Vec<WorkflowDatasetAxis>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowDatasetValueInfo {
    pub id: String,
    pub data_class: String,
    pub element_type: String,
    pub shape: WorkflowDatasetShape,
    pub semantic_type: Option<String>,
    pub unit: Option<String>,
    pub encoding: Option<WorkflowDatasetEncoding>,
    pub schema_ref: Option<OperatorSchemaRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkflowDatasetContract {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub values: Vec<WorkflowDatasetValueInfo>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorDescriptor {
    pub id: String,
    pub version: String,
    pub domain: String,
    pub family: String,
    pub kind: OperatorKind,
    pub summary: String,
    pub capability_tags: Vec<String>,
    pub origin: OperatorOrigin,
    pub input_schema: OperatorSchemaRef,
    pub output_schema: OperatorSchemaRef,
    #[serde(default)]
    pub inputs: Vec<OperatorPortDescriptor>,
    #[serde(default)]
    pub outputs: Vec<OperatorPortDescriptor>,
    pub validation: OperatorValidationProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OperatorRunContext {
    pub orchestrated: bool,
    pub project_id: Option<String>,
    pub model_id: Option<String>,
    pub workflow_run_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorRunRequest {
    pub operator_id: String,
    pub input: Value,
    #[serde(default)]
    pub context: OperatorRunContext,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorArtifactRef {
    pub kind: String,
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorRunResult {
    pub operator_id: String,
    pub summary: Value,
    #[serde(default)]
    pub artifacts: Vec<OperatorArtifactRef>,
}
