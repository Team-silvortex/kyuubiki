export type WorkflowCatalogEntryArtifact = {
  node_id: string;
  artifact_type: string;
  description: string;
};

export type WorkflowOperatorSchemaRef = {
  schema: string;
  version: string;
};

export type WorkflowOperatorPortDescriptor = {
  id: string;
  artifact_type: string;
  description: string;
  dataset_value?: string | null;
  schema_ref?: WorkflowOperatorSchemaRef | null;
};

export type WorkflowOperatorValidationProfile = {
  baseline_status: "verified" | "partial" | "unverified";
  baseline_cases: string[];
  smoke_paths: string[];
};

export type WorkflowOperatorDescriptor = {
  id: string;
  version: string;
  domain: string;
  family: string;
  kind: "solver" | "transform" | "extract" | "export" | "workflow_bridge";
  summary: string;
  capability_tags: string[];
  origin: "built_in" | "external_local" | "external_remote";
  input_schema: WorkflowOperatorSchemaRef;
  output_schema: WorkflowOperatorSchemaRef;
  config_schema?: WorkflowOperatorSchemaRef;
  config_example?: Record<string, unknown> | null;
  inputs: WorkflowOperatorPortDescriptor[];
  outputs: WorkflowOperatorPortDescriptor[];
  validation: WorkflowOperatorValidationProfile;
};

export type WorkflowOperatorCatalogPayload = {
  operators: WorkflowOperatorDescriptor[];
};

export type WorkflowGraphPort = {
  id: string;
  artifact_type: string;
  description?: string;
  dataset_value?: string;
};

export type WorkflowGraphNode = {
  id: string;
  kind: string;
  operator_id?: string;
  config?: Record<string, unknown>;
  inputs?: WorkflowGraphPort[];
  outputs?: WorkflowGraphPort[];
};

export type WorkflowGraphEdge = {
  id: string;
  from: { node: string; port: string };
  to: { node: string; port: string };
  artifact_type: string;
  dataset_value?: string;
};

export type WorkflowDatasetAxis = {
  id: string;
  label?: string;
  size?: number;
  semantic?: string;
};

export type WorkflowDatasetShape = {
  axes?: WorkflowDatasetAxis[];
};

export type WorkflowDatasetSchemaRef = {
  schema: string;
  version: string;
};

export type WorkflowDatasetValueInfo = {
  id: string;
  data_class: string;
  element_type: string;
  shape: WorkflowDatasetShape;
  semantic_type?: string;
  unit?: string;
  encoding?: string;
  schema_ref?: WorkflowDatasetSchemaRef;
};

export type WorkflowDatasetContract = {
  schema_version: string;
  id: string;
  version: string;
  name?: string;
  description?: string;
  values: WorkflowDatasetValueInfo[];
  metadata?: Record<string, string>;
};

export type WorkflowGraphDefinition = {
  schema_version: string;
  id: string;
  name?: string;
  version?: string;
  dataset_contract?: WorkflowDatasetContract;
  entry_inputs?: WorkflowCatalogEntryArtifact[];
  output_artifacts?: WorkflowCatalogEntryArtifact[];
  entry_nodes?: string[];
  output_nodes?: string[];
  defaults?: Record<string, unknown>;
  nodes: WorkflowGraphNode[];
  edges?: WorkflowGraphEdge[];
};

export type WorkflowCatalogEntry = {
  id: string;
  name: string;
  version: string;
  summary: string;
  graph?: WorkflowGraphDefinition;
  entry_inputs: WorkflowCatalogEntryArtifact[];
  output_artifacts: WorkflowCatalogEntryArtifact[];
  local?: {
    storage_id: string;
    source_workflow_id?: string;
    source_workflow_name?: string;
    input_artifact_texts?: Record<string, string>;
    promoted_at?: string;
    variant_of_workflow_id?: string;
    variant_of_workflow_name?: string;
    notes?: string;
  };
};

export type WorkflowCatalogPayload = {
  workflows: WorkflowCatalogEntry[];
};

export type WorkflowGraphJobResult = {
  workflow_id: string;
  current_node?: string | null;
  progress_events?: Array<Record<string, unknown>>;
  completed_nodes: string[];
  artifacts: Record<string, unknown>;
};
