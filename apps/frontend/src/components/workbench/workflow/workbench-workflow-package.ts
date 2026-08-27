"use client";

import type { WorkflowCatalogEntry, WorkflowCatalogEntryArtifact, WorkflowGraphDefinition } from "@/lib/api";
import { asWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-import";
import type { WorkflowTemplateChainPreferenceSnapshot } from "@/components/workbench/workflow/workbench-workflow-template-chain-storage";

export type WorkflowPackageSearchIndex = {
  domains: string[];
  capability_tags: string[];
  operator_ids: string[];
  entry_artifacts: string[];
  output_artifacts: string[];
};

export type WorkflowPackageContractEntry = {
  node_id: string;
  artifact_type: string;
  description?: string;
  dataset_value?: string;
  semantic_type?: string;
  schema_ref?: string;
};

export type WorkflowPackageContractManifest = {
  dataset_schema?: string;
  dataset_contract_id?: string;
  dataset_contract_version?: string;
  dataset_value_ids: string[];
  entry_contracts: WorkflowPackageContractEntry[];
  output_contracts: WorkflowPackageContractEntry[];
};

export type WorkflowPackageBridgeSeedSummary = {
  operator_id: string;
  node_count: number;
  element_count: number;
  contract_version?: string;
};

export type WorkflowPackageOperatorFetchEntry = {
  operator_id: string;
  execution_mode: "orchestra_fetch" | "orchestra_only";
  source_ref: string;
  package_ref?: string;
  package_version?: string;
  integrity?: string;
  placement_tags: string[];
  required_capabilities: string[];
  cache_scope: "ephemeral" | "job" | "session";
};

export type WorkflowPackageDispatchPolicy = {
  authority_mode: "central_operator_library";
  agent_cache_policy: "ephemeral_fetch";
  missing_operator_behavior: "fetch_from_orchestra";
  agent_library_replication: "forbidden";
};

export type WorkflowPackageRuntimeManifest = {
  required_operator_ids: string[];
  sample_input_node_ids: string[];
  included_input_text_node_ids: string[];
  bridge_seed_summaries: WorkflowPackageBridgeSeedSummary[];
  dispatch_policy: WorkflowPackageDispatchPolicy;
  operator_fetch_plan: WorkflowPackageOperatorFetchEntry[];
};

export type WorkflowPackage = {
  format: "kyuubiki.workflow-package";
  version: 1;
  package_id: string;
  name: string;
  summary?: string;
  tags?: string[];
  package_version?: string;
  exported_at: string;
  search_index: WorkflowPackageSearchIndex;
  contract_manifest: WorkflowPackageContractManifest;
  runtime_manifest: WorkflowPackageRuntimeManifest;
  workflow: {
    id: string;
    source_workflow_id?: string;
    source_workflow_name?: string;
    variant_of_workflow_id?: string;
    variant_of_workflow_name?: string;
    notes?: string;
    graph: WorkflowGraphDefinition;
    input_artifact_texts?: Record<string, string>;
    input_artifact_semantic_types?: Record<string, string>;
    input_artifact_contract_warnings?: Record<string, string[]>;
    template_chain_preferences?: WorkflowTemplateChainPreferenceSnapshot;
  };
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}
function isOptionalString(value: unknown): value is string | undefined {
  return value === undefined || typeof value === "string";
}
function asStringArray(value: unknown): string[] | null {
  if (!Array.isArray(value) || !value.every((entry) => typeof entry === "string")) return null;
  return [...value];
}
function asStringRecord(value: unknown): Record<string, string> | null {
  if (!isRecord(value) || !Object.values(value).every((entry) => typeof entry === "string")) return null;
  return { ...value } as Record<string, string>;
}
function asStringArrayRecord(value: unknown): Record<string, string[]> | null {
  if (!isRecord(value)) return null;
  const entries: Array<[string, string[]]> = [];
  for (const [key, entryValue] of Object.entries(value)) {
    const values = asStringArray(entryValue);
    if (!values) return null;
    entries.push([key, values]);
  }
  return Object.fromEntries(entries);
}

function asTemplateChainPreferences(
  value: unknown,
): WorkflowTemplateChainPreferenceSnapshot | null {
  if (!isRecord(value)) return null;
  const favoriteChainIds = asStringArray(value.favoriteChainIds);
  const favoriteChainAliases = asStringRecord(value.favoriteChainAliases);
  if (!favoriteChainIds || !favoriteChainAliases) return null;
  return { favoriteChainIds, favoriteChainAliases };
}

function uniqueSorted(values: Array<string | undefined | null>) {
  return [...new Set(values.filter((value): value is string => typeof value === "string" && value.trim().length > 0))].sort();
}

function deriveDomainsFromGraph(graph: WorkflowGraphDefinition) {
  const text = JSON.stringify(graph).toLowerCase();
  return uniqueSorted([
    text.includes("electrostatic") ? "electromagnetic" : undefined,
    text.includes("thermal_plane") || text.includes("thermo") ? "thermo_mechanical" : undefined,
    text.includes("heat_plane") || text.includes("temperature") ? "thermal" : undefined,
    text.includes("frame") || text.includes("truss") || text.includes("beam") ? "mechanical" : undefined,
  ]);
}

function deriveCapabilityTagsFromGraph(graph: WorkflowGraphDefinition) {
  const nodeKinds = graph.nodes.map((node) => node.kind);
  const operatorIds = graph.nodes
    .map((node) => node.operator_id)
    .filter((value): value is string => typeof value === "string");
  const text = JSON.stringify(graph).toLowerCase();

  return uniqueSorted([
    ...nodeKinds,
    ...operatorIds.flatMap((value) => value.split(/[^a-z0-9]+/i).filter(Boolean)),
    text.includes("quad") ? "quad" : undefined,
    text.includes("triangle") ? "triangle" : undefined,
    text.includes("workflow_bridge") ? "workflow_bridge" : undefined,
    text.includes("condition") ? "condition" : undefined,
  ]);
}

function formatSchemaRef(value?: { schema: string; version: string } | null) {
  if (!value?.schema || !value?.version) return undefined;
  return `${value.schema}@${value.version}`;
}

function buildArtifactContractEntries(params: {
  artifacts: WorkflowCatalogEntryArtifact[];
  graph: WorkflowGraphDefinition;
  portDirection: "input" | "output";
}) {
  const values = params.graph.dataset_contract?.values ?? [];
  const valueMap = new Map(values.map((value) => [value.id, value] as const));

  return params.artifacts.map((artifact) => {
    const node = params.graph.nodes.find((entry) => entry.id === artifact.node_id);
    const ports = params.portDirection === "input" ? node?.inputs ?? [] : node?.outputs ?? [];
    const matchedPort = ports.find((port) => port.artifact_type === artifact.artifact_type);
    const matchedValue =
      (matchedPort?.dataset_value ? valueMap.get(matchedPort.dataset_value) : null) ??
      values.find((value) => value.semantic_type === artifact.artifact_type) ??
      null;

    return {
      node_id: artifact.node_id,
      artifact_type: artifact.artifact_type,
      description: artifact.description,
      dataset_value: matchedPort?.dataset_value ?? matchedValue?.id,
      semantic_type: matchedValue?.semantic_type,
      schema_ref: formatSchemaRef(matchedValue?.schema_ref),
    };
  });
}

export function buildWorkflowPackageContractManifest(
  graph: WorkflowGraphDefinition,
): WorkflowPackageContractManifest {
  const datasetValues = graph.dataset_contract?.values ?? [];

  return {
    dataset_schema: graph.dataset_contract?.schema_version,
    dataset_contract_id: graph.dataset_contract?.id,
    dataset_contract_version: graph.dataset_contract?.version,
    dataset_value_ids: datasetValues.map((value) => value.id),
    entry_contracts: buildArtifactContractEntries({
      artifacts: graph.entry_inputs ?? [],
      graph,
      portDirection: "input",
    }),
    output_contracts: buildArtifactContractEntries({
      artifacts: graph.output_artifacts ?? [],
      graph,
      portDirection: "output",
    }),
  };
}

function asNodeRecord(value: unknown) {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : null;
}

export function buildWorkflowPackageRuntimeManifest(params: {
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
}): WorkflowPackageRuntimeManifest {
  const requiredOperatorIds = uniqueSorted(
    params.graph.nodes.map((node) => node.operator_id).filter((value): value is string => typeof value === "string"),
  );
  const sampleInputNodeIds = uniqueSorted(
    (params.graph.entry_inputs ?? []).map((artifact) => artifact.node_id),
  );
  const includedInputTextNodeIds = uniqueSorted(
    Object.keys(params.inputArtifactTexts ?? {}),
  );
  const bridgeSeedSummaries = params.graph.nodes.flatMap((node) => {
    if (!node.operator_id?.startsWith("bridge.")) return [];
    const config = asNodeRecord(node.config);
    const seedModel = config?.seed_model
      ? asNodeRecord(config.seed_model)
      : config;
    const nodes = Array.isArray(seedModel?.nodes) ? seedModel.nodes : [];
    const elements = Array.isArray(seedModel?.elements) ? seedModel.elements : [];
    const contract = asNodeRecord(config?.contract);
    return [
      {
        operator_id: node.operator_id,
        node_count: nodes.length,
        element_count: elements.length,
        contract_version:
          typeof contract?.version === "string" ? contract.version : undefined,
      },
    ];
  });

  const operatorFetchPlan = requiredOperatorIds.map((operatorId) => ({
    operator_id: operatorId,
    execution_mode: "orchestra_fetch" as const,
    source_ref: `orchestra://operator/${operatorId}`,
    package_ref: `orchestra://operator-package/${operatorId}`,
    package_version: "library-managed",
    integrity: undefined,
    placement_tags: deriveOperatorPlacementTags(operatorId),
    required_capabilities: deriveOperatorRequiredCapabilities(operatorId),
    cache_scope: "job" as const,
  }));

  return {
    required_operator_ids: requiredOperatorIds,
    sample_input_node_ids: sampleInputNodeIds,
    included_input_text_node_ids: includedInputTextNodeIds,
    bridge_seed_summaries: bridgeSeedSummaries,
    dispatch_policy: {
      authority_mode: "central_operator_library",
      agent_cache_policy: "ephemeral_fetch",
      missing_operator_behavior: "fetch_from_orchestra",
      agent_library_replication: "forbidden",
    },
    operator_fetch_plan: operatorFetchPlan,
  };
}

export function buildWorkflowPackageSearchIndex(params: {
  workflow?: Pick<WorkflowCatalogEntry, "domains" | "capability_tags"> | null;
  graph: WorkflowGraphDefinition;
  tags?: string[];
}): WorkflowPackageSearchIndex {
  const graph = params.graph;
  const operatorIds = uniqueSorted(
    graph.nodes.map((node) => node.operator_id).filter((value): value is string => typeof value === "string"),
  );

  return {
    domains: uniqueSorted([...(params.workflow?.domains ?? []), ...deriveDomainsFromGraph(graph)]),
    capability_tags: uniqueSorted([
      ...(params.workflow?.capability_tags ?? []),
      ...(params.tags ?? []),
      ...deriveCapabilityTagsFromGraph(graph),
    ]),
    operator_ids: operatorIds,
    entry_artifacts: uniqueSorted((graph.entry_inputs ?? []).map((entry) => entry.artifact_type)),
    output_artifacts: uniqueSorted((graph.output_artifacts ?? []).map((entry) => entry.artifact_type)),
  };
}

export function buildWorkflowPackage(params: {
  workflow: WorkflowCatalogEntry;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  inputArtifactSemanticTypes?: Record<string, string>;
  inputArtifactContractWarnings?: Record<string, string[]>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
}): WorkflowPackage {
  const tags = params.workflow.local?.tags ?? params.workflow.capability_tags ?? [];

  const packageValue: WorkflowPackage = {
    format: "kyuubiki.workflow-package",
    version: 1,
    package_id: params.workflow.local?.imported_from_package_id ?? params.workflow.id,
    name: params.workflow.name,
    summary: params.workflow.summary,
    tags,
    package_version:
      params.workflow.local?.imported_from_package_version ?? params.workflow.version,
    exported_at: new Date().toISOString(),
    search_index: buildWorkflowPackageSearchIndex({
      workflow: params.workflow,
      graph: params.graph,
      tags,
    }),
    contract_manifest: buildWorkflowPackageContractManifest(params.graph),
    runtime_manifest: buildWorkflowPackageRuntimeManifest({
      graph: params.graph,
      inputArtifactTexts: params.inputArtifactTexts,
    }),
    workflow: {
      id: params.graph.id,
      source_workflow_id:
        params.workflow.local?.source_workflow_id ?? params.workflow.id,
      source_workflow_name:
        params.workflow.local?.source_workflow_name ?? params.workflow.name,
      variant_of_workflow_id: params.workflow.local?.variant_of_workflow_id,
      variant_of_workflow_name: params.workflow.local?.variant_of_workflow_name,
      notes: params.workflow.local?.notes,
      graph: params.graph,
      input_artifact_texts: params.inputArtifactTexts,
      input_artifact_semantic_types: params.inputArtifactSemanticTypes,
      input_artifact_contract_warnings: params.inputArtifactContractWarnings,
      template_chain_preferences: params.templateChainPreferences,
    },
  };
  return structuredClone(packageValue);
}

function asParsedArray<T>(value: unknown, parser: (entry: unknown) => T | null): T[] | null {
  if (!Array.isArray(value)) return null;
  const entries: T[] = [];
  for (const entry of value) {
    const parsed = parser(entry);
    if (!parsed) return null;
    entries.push(parsed);
  }
  return entries;
}

function asPackageContractEntry(value: unknown): WorkflowPackageContractEntry | null {
  if (!isRecord(value) || !isNonEmptyString(value.node_id) || !isNonEmptyString(value.artifact_type)) return null;
  if (!isOptionalString(value.description) || !isOptionalString(value.dataset_value) || !isOptionalString(value.semantic_type) || !isOptionalString(value.schema_ref)) return null;
  return {
    node_id: value.node_id,
    artifact_type: value.artifact_type,
    description: value.description,
    dataset_value: value.dataset_value,
    semantic_type: value.semantic_type,
    schema_ref: value.schema_ref,
  };
}

function asPackageContractManifest(value: unknown): WorkflowPackageContractManifest | null {
  if (!isRecord(value)) return null;
  if (!isOptionalString(value.dataset_schema) || !isOptionalString(value.dataset_contract_id) || !isOptionalString(value.dataset_contract_version)) return null;
  const datasetValueIds = asStringArray(value.dataset_value_ids);
  const entryContracts = asParsedArray(value.entry_contracts, asPackageContractEntry);
  const outputContracts = asParsedArray(value.output_contracts, asPackageContractEntry);
  if (!datasetValueIds || !entryContracts || !outputContracts) return null;
  return {
    dataset_schema: value.dataset_schema,
    dataset_contract_id: value.dataset_contract_id,
    dataset_contract_version: value.dataset_contract_version,
    dataset_value_ids: datasetValueIds,
    entry_contracts: entryContracts,
    output_contracts: outputContracts,
  };
}

function asBridgeSeedSummary(value: unknown): WorkflowPackageBridgeSeedSummary | null {
  if (!isRecord(value) || !isNonEmptyString(value.operator_id)) return null;
  if (!Number.isInteger(value.node_count) || (value.node_count as number) < 0) return null;
  if (!Number.isInteger(value.element_count) || (value.element_count as number) < 0) return null;
  if (!isOptionalString(value.contract_version)) return null;
  return {
    operator_id: value.operator_id,
    node_count: value.node_count as number,
    element_count: value.element_count as number,
    contract_version: value.contract_version,
  };
}

function asOperatorFetchEntry(value: unknown): WorkflowPackageOperatorFetchEntry | null {
  if (!isRecord(value) || !isNonEmptyString(value.operator_id) || !isNonEmptyString(value.source_ref)) return null;
  if (value.execution_mode !== "orchestra_fetch" && value.execution_mode !== "orchestra_only") return null;
  if (value.cache_scope !== "ephemeral" && value.cache_scope !== "job" && value.cache_scope !== "session") return null;
  if (!isOptionalString(value.package_ref) || !isOptionalString(value.package_version) || !isOptionalString(value.integrity)) return null;
  const placementTags = asStringArray(value.placement_tags);
  const requiredCapabilities = asStringArray(value.required_capabilities);
  if (!placementTags || !requiredCapabilities) return null;
  return {
    operator_id: value.operator_id,
    execution_mode: value.execution_mode,
    source_ref: value.source_ref,
    package_ref: value.package_ref,
    package_version: value.package_version,
    integrity: value.integrity,
    placement_tags: placementTags,
    required_capabilities: requiredCapabilities,
    cache_scope: value.cache_scope,
  };
}

function asRuntimeManifest(value: unknown): WorkflowPackageRuntimeManifest | null {
  if (!isRecord(value) || !isRecord(value.dispatch_policy)) return null;
  const policy = value.dispatch_policy;
  if (policy.authority_mode !== "central_operator_library" || policy.agent_cache_policy !== "ephemeral_fetch" || policy.missing_operator_behavior !== "fetch_from_orchestra" || policy.agent_library_replication !== "forbidden") return null;
  const requiredOperatorIds = asStringArray(value.required_operator_ids);
  const sampleInputNodeIds = asStringArray(value.sample_input_node_ids);
  const includedInputTextNodeIds = asStringArray(value.included_input_text_node_ids);
  const bridgeSeedSummaries = asParsedArray(value.bridge_seed_summaries, asBridgeSeedSummary);
  const operatorFetchPlan = asParsedArray(value.operator_fetch_plan, asOperatorFetchEntry);
  if (!requiredOperatorIds || !sampleInputNodeIds || !includedInputTextNodeIds || !bridgeSeedSummaries || !operatorFetchPlan) return null;
  return {
    required_operator_ids: requiredOperatorIds,
    sample_input_node_ids: sampleInputNodeIds,
    included_input_text_node_ids: includedInputTextNodeIds,
    bridge_seed_summaries: bridgeSeedSummaries,
    dispatch_policy: {
      authority_mode: "central_operator_library",
      agent_cache_policy: "ephemeral_fetch",
      missing_operator_behavior: "fetch_from_orchestra",
      agent_library_replication: "forbidden",
    },
    operator_fetch_plan: operatorFetchPlan,
  };
}

function asPackageSearchIndex(value: unknown): WorkflowPackageSearchIndex | null {
  if (!isRecord(value)) return null;
  const domains = asStringArray(value.domains);
  const capabilityTags = asStringArray(value.capability_tags);
  const operatorIds = asStringArray(value.operator_ids);
  const entryArtifacts = asStringArray(value.entry_artifacts);
  const outputArtifacts = asStringArray(value.output_artifacts);
  if (!domains || !capabilityTags || !operatorIds || !entryArtifacts || !outputArtifacts) return null;
  return {
    domains,
    capability_tags: capabilityTags,
    operator_ids: operatorIds,
    entry_artifacts: entryArtifacts,
    output_artifacts: outputArtifacts,
  };
}

export function asWorkflowPackage(value: unknown): WorkflowPackage | null {
  if (!isRecord(value) || value.format !== "kyuubiki.workflow-package" || value.version !== 1) return null;
  if (!isNonEmptyString(value.package_id) || !isNonEmptyString(value.name) || !isRecord(value.workflow)) return null;
  if (!isOptionalString(value.summary) || !isOptionalString(value.package_version)) return null;
  if (typeof value.exported_at !== "string" || !Number.isFinite(Date.parse(value.exported_at))) return null;
  const tags = value.tags === undefined ? undefined : asStringArray(value.tags);
  if (tags === null) return null;
  const searchIndex = asPackageSearchIndex(value.search_index);
  const contractManifest = asPackageContractManifest(value.contract_manifest);
  const runtimeManifest = asRuntimeManifest(value.runtime_manifest);
  const graph = asWorkflowGraphDefinition(value.workflow.graph);
  if (!searchIndex || !contractManifest || !runtimeManifest || !graph || !isNonEmptyString(value.workflow.id)) return null;
  if (!isOptionalString(value.workflow.source_workflow_id) || !isOptionalString(value.workflow.source_workflow_name) || !isOptionalString(value.workflow.variant_of_workflow_id) || !isOptionalString(value.workflow.variant_of_workflow_name) || !isOptionalString(value.workflow.notes)) return null;

  const inputArtifactTexts = value.workflow.input_artifact_texts === undefined ? undefined : asStringRecord(value.workflow.input_artifact_texts);
  const inputArtifactSemanticTypes = value.workflow.input_artifact_semantic_types === undefined ? undefined : asStringRecord(value.workflow.input_artifact_semantic_types);
  const inputArtifactContractWarnings = value.workflow.input_artifact_contract_warnings === undefined ? undefined : asStringArrayRecord(value.workflow.input_artifact_contract_warnings);
  const templateChainPreferences = value.workflow.template_chain_preferences === undefined ? undefined : asTemplateChainPreferences(value.workflow.template_chain_preferences);
  if (inputArtifactTexts === null || inputArtifactSemanticTypes === null || inputArtifactContractWarnings === null || templateChainPreferences === null) return null;

  const packageValue: WorkflowPackage = {
    format: "kyuubiki.workflow-package",
    version: 1,
    package_id: value.package_id,
    name: value.name,
    summary: value.summary,
    tags,
    package_version: value.package_version,
    exported_at: value.exported_at,
    search_index: searchIndex,
    contract_manifest: contractManifest,
    runtime_manifest: runtimeManifest,
    workflow: {
      id: value.workflow.id,
      source_workflow_id: value.workflow.source_workflow_id,
      source_workflow_name: value.workflow.source_workflow_name,
      variant_of_workflow_id: value.workflow.variant_of_workflow_id,
      variant_of_workflow_name: value.workflow.variant_of_workflow_name,
      notes: value.workflow.notes,
      graph,
      input_artifact_texts: inputArtifactTexts,
      input_artifact_semantic_types: inputArtifactSemanticTypes,
      input_artifact_contract_warnings: inputArtifactContractWarnings,
      template_chain_preferences: templateChainPreferences,
    },
  };
  return structuredClone(packageValue);
}

function deriveOperatorPlacementTags(operatorId: string) {
  const normalized = operatorId.toLowerCase();
  return uniqueSorted([
    normalized.includes("electrostatic") ? "electromagnetic" : undefined,
    normalized.includes("thermal") ? "thermo_mechanical" : undefined,
    normalized.includes("heat") ? "thermal" : undefined,
    normalized.includes("frame") ? "frame" : undefined,
    normalized.includes("truss") ? "truss" : undefined,
    normalized.includes("plane") ? "mesh" : undefined,
    normalized.includes("bridge.") ? "bridge" : undefined,
    normalized.includes("extract.") ? "postprocess" : undefined,
    normalized.includes("transform.") ? "transform" : undefined,
    normalized.includes("export.") ? "export" : undefined,
  ]);
}

function deriveOperatorRequiredCapabilities(operatorId: string) {
  const normalized = operatorId.toLowerCase();
  return uniqueSorted([
    normalized.startsWith("solve.") ? "solver_rpc" : undefined,
    normalized.startsWith("bridge.") ? "workflow_bridge_runtime" : undefined,
    normalized.startsWith("transform.") ? "workflow_transform_runtime" : undefined,
    normalized.startsWith("extract.") ? "workflow_extract_runtime" : undefined,
    normalized.startsWith("export.") ? "workflow_export_runtime" : undefined,
  ]);
}
