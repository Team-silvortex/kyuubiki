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
  return typeof value === "object" && value !== null;
}
function asStringArray(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) return undefined;
  const values = value.filter((entry): entry is string => typeof entry === "string");
  return values.length > 0 ? values : undefined;
}
function asStringRecord(value: unknown): Record<string, string> | undefined {
  if (!isRecord(value)) return undefined;
  return Object.fromEntries(
    Object.entries(value).filter(
      ([key, entryValue]) => typeof key === "string" && typeof entryValue === "string",
    ),
  ) as Record<string, string>;
}
function asStringArrayRecord(value: unknown): Record<string, string[]> | undefined {
  if (!isRecord(value)) return undefined;
  const entries = Object.entries(value).flatMap(([key, entryValue]) => {
    const values = asStringArray(entryValue);
    return typeof key === "string" && values ? [[key, values] as const] : [];
  });
  return entries.length > 0 ? Object.fromEntries(entries) : undefined;
}

function asTemplateChainPreferences(
  value: unknown,
): WorkflowTemplateChainPreferenceSnapshot | undefined {
  if (!isRecord(value)) return undefined;
  const favoriteChainIds = asStringArray(value.favoriteChainIds) ?? [];
  const favoriteChainAliases = asStringRecord(value.favoriteChainAliases) ?? {};
  if (favoriteChainIds.length === 0 && Object.keys(favoriteChainAliases).length === 0) {
    return undefined;
  }
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
}) {
  const values = params.graph.dataset_contract?.values ?? [];
  const valueMap = new Map(values.map((value) => [value.id, value] as const));

  return params.artifacts.map((artifact) => {
    const node = params.graph.nodes.find((entry) => entry.id === artifact.node_id);
    const ports = [
      ...(node?.inputs ?? []),
      ...(node?.outputs ?? []),
    ];
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
    }),
    output_contracts: buildArtifactContractEntries({
      artifacts: graph.output_artifacts ?? [],
      graph,
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

  return {
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
}

export function asWorkflowPackage(value: unknown): WorkflowPackage | null {
  if (!isRecord(value)) return null;
  if (
    value.format !== "kyuubiki.workflow-package" ||
    value.version !== 1 ||
    typeof value.package_id !== "string" ||
    typeof value.name !== "string" ||
    !isRecord(value.workflow)
  ) {
    return null;
  }

  const graph = asWorkflowGraphDefinition(value.workflow.graph);
  if (!graph) return null;

  return {
    format: "kyuubiki.workflow-package",
    version: 1,
    package_id: value.package_id,
    name: value.name,
    summary: typeof value.summary === "string" ? value.summary : undefined,
    tags: asStringArray(value.tags),
    package_version:
      typeof value.package_version === "string" ? value.package_version : undefined,
    exported_at:
      typeof value.exported_at === "string" ? value.exported_at : new Date().toISOString(),
    search_index: isRecord(value.search_index)
      ? {
          domains: asStringArray(value.search_index.domains) ?? [],
          capability_tags: asStringArray(value.search_index.capability_tags) ?? [],
          operator_ids: asStringArray(value.search_index.operator_ids) ?? [],
          entry_artifacts: asStringArray(value.search_index.entry_artifacts) ?? [],
          output_artifacts: asStringArray(value.search_index.output_artifacts) ?? [],
        }
      : buildWorkflowPackageSearchIndex({ graph, tags: asStringArray(value.tags) }),
    contract_manifest: isRecord(value.contract_manifest)
      ? {
          dataset_schema:
            typeof value.contract_manifest.dataset_schema === "string"
              ? value.contract_manifest.dataset_schema
              : undefined,
          dataset_contract_id:
            typeof value.contract_manifest.dataset_contract_id === "string"
              ? value.contract_manifest.dataset_contract_id
              : undefined,
          dataset_contract_version:
            typeof value.contract_manifest.dataset_contract_version === "string"
              ? value.contract_manifest.dataset_contract_version
              : undefined,
          dataset_value_ids: asStringArray(value.contract_manifest.dataset_value_ids) ?? [],
          entry_contracts: Array.isArray(value.contract_manifest.entry_contracts)
            ? value.contract_manifest.entry_contracts.flatMap((entry) =>
                isRecord(entry) && typeof entry.node_id === "string" && typeof entry.artifact_type === "string"
                  ? [
                      {
                        node_id: entry.node_id,
                        artifact_type: entry.artifact_type,
                        description:
                          typeof entry.description === "string" ? entry.description : undefined,
                        dataset_value:
                          typeof entry.dataset_value === "string" ? entry.dataset_value : undefined,
                        semantic_type:
                          typeof entry.semantic_type === "string" ? entry.semantic_type : undefined,
                        schema_ref:
                          typeof entry.schema_ref === "string" ? entry.schema_ref : undefined,
                      },
                    ]
                  : [],
              )
            : [],
          output_contracts: Array.isArray(value.contract_manifest.output_contracts)
            ? value.contract_manifest.output_contracts.flatMap((entry) =>
                isRecord(entry) && typeof entry.node_id === "string" && typeof entry.artifact_type === "string"
                  ? [
                      {
                        node_id: entry.node_id,
                        artifact_type: entry.artifact_type,
                        description:
                          typeof entry.description === "string" ? entry.description : undefined,
                        dataset_value:
                          typeof entry.dataset_value === "string" ? entry.dataset_value : undefined,
                        semantic_type:
                          typeof entry.semantic_type === "string" ? entry.semantic_type : undefined,
                        schema_ref:
                          typeof entry.schema_ref === "string" ? entry.schema_ref : undefined,
                      },
                    ]
                  : [],
              )
            : [],
        }
      : buildWorkflowPackageContractManifest(graph),
    runtime_manifest: isRecord(value.runtime_manifest)
      ? {
          required_operator_ids:
            asStringArray(value.runtime_manifest.required_operator_ids) ?? [],
          sample_input_node_ids:
            asStringArray(value.runtime_manifest.sample_input_node_ids) ?? [],
          included_input_text_node_ids:
            asStringArray(value.runtime_manifest.included_input_text_node_ids) ?? [],
          bridge_seed_summaries: Array.isArray(value.runtime_manifest.bridge_seed_summaries)
            ? value.runtime_manifest.bridge_seed_summaries.flatMap((entry) =>
                isRecord(entry) &&
                typeof entry.operator_id === "string" &&
                typeof entry.node_count === "number" &&
                typeof entry.element_count === "number"
                  ? [
                      {
                        operator_id: entry.operator_id,
                        node_count: entry.node_count,
                        element_count: entry.element_count,
                        contract_version:
                          typeof entry.contract_version === "string"
                            ? entry.contract_version
                            : undefined,
                      },
                    ]
                  : [],
              )
            : [],
          dispatch_policy:
            isRecord(value.runtime_manifest.dispatch_policy) &&
            value.runtime_manifest.dispatch_policy.authority_mode ===
              "central_operator_library" &&
            value.runtime_manifest.dispatch_policy.agent_cache_policy ===
              "ephemeral_fetch" &&
            value.runtime_manifest.dispatch_policy.missing_operator_behavior ===
              "fetch_from_orchestra" &&
            value.runtime_manifest.dispatch_policy.agent_library_replication ===
              "forbidden"
              ? {
                  authority_mode: "central_operator_library",
                  agent_cache_policy: "ephemeral_fetch",
                  missing_operator_behavior: "fetch_from_orchestra",
                  agent_library_replication: "forbidden",
                }
              : {
                  authority_mode: "central_operator_library",
                  agent_cache_policy: "ephemeral_fetch",
                  missing_operator_behavior: "fetch_from_orchestra",
                  agent_library_replication: "forbidden",
                },
          operator_fetch_plan: Array.isArray(value.runtime_manifest.operator_fetch_plan)
            ? value.runtime_manifest.operator_fetch_plan.flatMap((entry) =>
                isRecord(entry) &&
                typeof entry.operator_id === "string" &&
                (entry.execution_mode === "orchestra_fetch" ||
                  entry.execution_mode === "orchestra_only") &&
                typeof entry.source_ref === "string" &&
                (entry.cache_scope === "ephemeral" ||
                  entry.cache_scope === "job" ||
                  entry.cache_scope === "session")
                  ? [
                      {
                        operator_id: entry.operator_id,
                        execution_mode: entry.execution_mode,
                        source_ref: entry.source_ref,
                        package_ref:
                          typeof entry.package_ref === "string"
                            ? entry.package_ref
                            : undefined,
                        package_version:
                          typeof entry.package_version === "string"
                            ? entry.package_version
                            : undefined,
                        integrity:
                          typeof entry.integrity === "string"
                            ? entry.integrity
                            : undefined,
                        placement_tags: asStringArray(entry.placement_tags) ?? [],
                        required_capabilities:
                          asStringArray(entry.required_capabilities) ?? [],
                        cache_scope: entry.cache_scope,
                      },
                    ]
                  : [],
              )
            : buildWorkflowPackageRuntimeManifest({
                graph,
                inputArtifactTexts: asStringRecord(value.workflow.input_artifact_texts),
              }).operator_fetch_plan,
        }
      : buildWorkflowPackageRuntimeManifest({
          graph,
          inputArtifactTexts: asStringRecord(value.workflow.input_artifact_texts),
        }),
    workflow: {
      id: typeof value.workflow.id === "string" ? value.workflow.id : graph.id,
      source_workflow_id:
        typeof value.workflow.source_workflow_id === "string"
          ? value.workflow.source_workflow_id
          : undefined,
      source_workflow_name:
        typeof value.workflow.source_workflow_name === "string"
          ? value.workflow.source_workflow_name
          : undefined,
      variant_of_workflow_id:
        typeof value.workflow.variant_of_workflow_id === "string"
          ? value.workflow.variant_of_workflow_id
          : undefined,
      variant_of_workflow_name:
        typeof value.workflow.variant_of_workflow_name === "string"
          ? value.workflow.variant_of_workflow_name
          : undefined,
      notes: typeof value.workflow.notes === "string" ? value.workflow.notes : undefined,
      graph,
      input_artifact_texts: asStringRecord(value.workflow.input_artifact_texts),
      input_artifact_semantic_types: asStringRecord(
        value.workflow.input_artifact_semantic_types,
      ),
      input_artifact_contract_warnings: asStringArrayRecord(
        value.workflow.input_artifact_contract_warnings,
      ),
      template_chain_preferences: asTemplateChainPreferences(
        value.workflow.template_chain_preferences,
      ),
    },
  };
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
