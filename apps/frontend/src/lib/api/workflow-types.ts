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

export type WorkflowOperatorExecutionDescriptor = {
  authority_mode: "central_operator_library";
  execution_mode: "orchestra_fetch" | "orchestra_only";
  source_ref: string;
  package_ref?: string | null;
  package_version?: string | null;
  integrity?: string | null;
  placement_tags: string[];
  required_capabilities: string[];
  cache_scope: "ephemeral" | "job" | "session";
  agent_fetchable?: boolean;
};

export type WorkflowBridgeContractSupport = {
  source: {
    fields: string[];
    distributions: Record<string, string[]>;
    node_index_fields: string[];
  };
  transform: {
    reductions: string[];
    default_reduction_by_distribution?: Record<string, string>;
    default_scale?: number;
    default_value?: number;
  };
  target: {
    fields: string[];
    default_field?: string;
  };
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
  contract_support?: WorkflowBridgeContractSupport;
  inputs: WorkflowOperatorPortDescriptor[];
  outputs: WorkflowOperatorPortDescriptor[];
  validation: WorkflowOperatorValidationProfile;
  execution?: WorkflowOperatorExecutionDescriptor;
};

export type WorkflowOperatorCatalogPayload = {
  operators: WorkflowOperatorDescriptor[];
};

export type WorkflowCatalogQuery = Partial<{
  q: string;
  domain: string;
  capability: string;
  operator_id: string;
  entry_artifact: string;
  output_artifact: string;
}>;

export type WorkflowOperatorCatalogQuery = Partial<{
  q: string;
  domain: string;
  kind: WorkflowOperatorDescriptor["kind"];
  validation: WorkflowOperatorValidationProfile["baseline_status"];
  capability: string;
}>;

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
  placement_tags?: string[];
  required_capabilities?: string[];
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
  dispatch_policy?: string;
  operator_fetch_plan?: Array<Record<string, unknown>>;
  placement_tags?: string[];
  required_capabilities?: string[];
  nodes: WorkflowGraphNode[];
  edges?: WorkflowGraphEdge[];
};

export type WorkflowCatalogRuntimeManifest = {
  required_operator_ids: string[];
  sample_input_node_ids: string[];
  included_input_text_node_ids: string[];
  bridge_seed_summaries: Array<Record<string, unknown>>;
  dispatch_policy: Record<string, unknown>;
  operator_fetch_plan: Array<Record<string, unknown>>;
};

export type WorkflowCatalogEntry = {
  id: string;
  name: string;
  version: string;
  summary: string;
  domains?: string[];
  capability_tags?: string[];
  graph?: WorkflowGraphDefinition;
  runtime_manifest?: WorkflowCatalogRuntimeManifest;
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
    tags?: string[];
    imported_from_package_id?: string;
    imported_from_package_version?: string;
  };
};

export type WorkflowCatalogPayload = {
  workflows: WorkflowCatalogEntry[];
};

export type WorkflowProgressEvent = {
  stage:
    | "queued"
    | "preprocessing"
    | "partitioning"
    | "solving"
    | "postprocessing"
    | "completed"
    | "failed"
    | "cancelled";
  progress: number;
  message?: string | null;
  node_id?: string | null;
  kind?: string | null;
  emitted_at?: string | null;
};

export type WorkflowSummaryArtifactFieldValue =
  | string
  | number
  | boolean
  | null;

export type WorkflowSummaryArtifactPayload = {
  contract_version: string;
  summary_kind?: string;
  source_operator_id?: string;
  source_artifact_type?: string;
  field_namespace?: string;
  fields: Record<string, WorkflowSummaryArtifactFieldValue>;
  metadata?: Record<string, string | number | boolean | null>;
};

export type WorkflowGraphArtifactEnvelope = {
  artifact_key?: string;
  artifact_type?: string;
  dataset_value?: string;
  node_id?: string;
  port_id?: string;
  encoding?: string;
  schema_ref?: WorkflowOperatorSchemaRef | null;
  contract_version?: string;
  payload?: unknown;
  content?: unknown;
};

export type WorkflowGraphArtifactValue = WorkflowGraphArtifactEnvelope | unknown;

export type WorkflowGraphJobResult = {
  workflow_id: string;
  current_node?: string | null;
  progress_events?: WorkflowProgressEvent[];
  completed_nodes: string[];
  skipped_nodes?: string[];
  branch_decisions?: Array<{
    node_id: string;
    chosen_output: string;
    predicate_result: boolean;
  }>;
  node_runs?: Array<{
    node_id: string;
    kind: string;
    operator_id?: string | null;
    status: "completed" | "skipped";
    consumed_artifacts?: string[];
    produced_artifacts?: string[];
  }>;
  artifact_lineage?: Array<{
    artifact_key: string;
    node_id: string;
    port_id: string;
    source_artifacts?: string[];
  }>;
  artifacts: Record<string, WorkflowGraphArtifactValue>;
};
