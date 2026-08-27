"use client";

import type {
  WorkflowDatasetContract,
  WorkflowGraphDefinition,
  WorkflowGraphNode,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import {
  applyWorkflowNodeTemplateSync,
  getWorkflowNodeTemplateSyncImpact,
  listAutoReconnectEdgeIds,
} from "@/components/workbench/workflow/workbench-workflow-template-impact";
import {
  countWorkflowBridgeNormalizationAdjustments,
  readBridgeNormalizationEntries,
} from "@/components/workbench/workflow/workbench-workflow-bridge-normalization";
import { normalizeBridgeConfigWithSupport } from "@/lib/workbench/workflow-bridge-contract-support";

export { countWorkflowBridgeNormalizationAdjustments };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isOptionalString(value: unknown): value is string | undefined {
  return value === undefined || typeof value === "string";
}

function isStringList(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((entry) => typeof entry === "string");
}

function isWorkflowGraphPort(value: unknown) {
  return (
    isRecord(value) &&
    isNonEmptyString(value.id) &&
    isNonEmptyString(value.artifact_type) &&
    isOptionalString(value.description) &&
    isOptionalString(value.dataset_value)
  );
}

function isWorkflowGraphNode(value: unknown): value is WorkflowGraphNode {
  if (!isRecord(value) || !isNonEmptyString(value.id) || !isNonEmptyString(value.kind)) return false;
  if (!isOptionalString(value.operator_id)) return false;
  if (value.config !== undefined && !isRecord(value.config)) return false;
  if (value.placement_tags !== undefined && !isStringList(value.placement_tags)) return false;
  if (value.required_capabilities !== undefined && !isStringList(value.required_capabilities)) return false;
  if (value.inputs !== undefined && (!Array.isArray(value.inputs) || !value.inputs.every(isWorkflowGraphPort))) return false;
  if (value.outputs !== undefined && (!Array.isArray(value.outputs) || !value.outputs.every(isWorkflowGraphPort))) return false;
  return true;
}

function isWorkflowGraphEndpoint(value: unknown) {
  return isRecord(value) && isNonEmptyString(value.node) && isNonEmptyString(value.port);
}

function isWorkflowGraphEdge(value: unknown) {
  return (
    isRecord(value) &&
    isNonEmptyString(value.id) &&
    isWorkflowGraphEndpoint(value.from) &&
    isWorkflowGraphEndpoint(value.to) &&
    isNonEmptyString(value.artifact_type) &&
    isOptionalString(value.dataset_value)
  );
}

function isWorkflowCatalogArtifact(value: unknown) {
  return (
    isRecord(value) &&
    isNonEmptyString(value.node_id) &&
    isNonEmptyString(value.artifact_type) &&
    isOptionalString(value.description)
  );
}

function isWorkflowDatasetAxis(value: unknown) {
  return (
    isRecord(value) &&
    isNonEmptyString(value.id) &&
    isOptionalString(value.label) &&
    (value.size === undefined || (typeof value.size === "number" && Number.isInteger(value.size) && value.size >= 0)) &&
    isOptionalString(value.semantic)
  );
}

function isWorkflowDatasetValue(value: unknown) {
  if (!isRecord(value)) return false;
  if (!isNonEmptyString(value.id) || !isNonEmptyString(value.data_class) || !isNonEmptyString(value.element_type)) return false;
  if (!isRecord(value.shape)) return false;
  if (value.shape.axes !== undefined && (!Array.isArray(value.shape.axes) || !value.shape.axes.every(isWorkflowDatasetAxis))) return false;
  if (!isOptionalString(value.semantic_type) || !isOptionalString(value.unit) || !isOptionalString(value.encoding)) return false;
  if (value.schema_ref !== undefined) {
    if (!isRecord(value.schema_ref)) return false;
    if (!isNonEmptyString(value.schema_ref.schema) || !isNonEmptyString(value.schema_ref.version)) return false;
  }
  return true;
}

function isStringRecord(value: unknown) {
  return isRecord(value) && Object.values(value).every((entry) => typeof entry === "string");
}

type WorkflowImportNormalizationDiagnostic = {
  message: string;
  locate?: { kind: "node"; nodeId: string };
};

export async function readJsonFile(file: File): Promise<unknown> {
  const text = await file.text();
  return JSON.parse(text) as unknown;
}

export function asWorkflowGraphDefinition(value: unknown): WorkflowGraphDefinition | null {
  if (!isRecord(value)) return null;
  if (typeof value.schema_version !== "string" || !isNonEmptyString(value.id)) return null;
  if (!Array.isArray(value.nodes) || !value.nodes.every(isWorkflowGraphNode)) return null;
  if (value.edges !== undefined && (!Array.isArray(value.edges) || !value.edges.every(isWorkflowGraphEdge))) return null;
  if (value.dataset_contract !== undefined && !asWorkflowDatasetContract(value.dataset_contract)) return null;
  if (value.entry_inputs !== undefined && (!Array.isArray(value.entry_inputs) || !value.entry_inputs.every(isWorkflowCatalogArtifact))) return null;
  if (value.output_artifacts !== undefined && (!Array.isArray(value.output_artifacts) || !value.output_artifacts.every(isWorkflowCatalogArtifact))) return null;
  if (value.entry_nodes !== undefined && !isStringList(value.entry_nodes)) return null;
  if (value.output_nodes !== undefined && !isStringList(value.output_nodes)) return null;
  if (value.defaults !== undefined && !isRecord(value.defaults)) return null;
  if (!isOptionalString(value.dispatch_policy)) return null;
  if (value.operator_fetch_plan !== undefined && (!Array.isArray(value.operator_fetch_plan) || !value.operator_fetch_plan.every(isRecord))) return null;
  if (value.placement_tags !== undefined && !isStringList(value.placement_tags)) return null;
  if (value.required_capabilities !== undefined && !isStringList(value.required_capabilities)) return null;
  return value as WorkflowGraphDefinition;
}

export function asWorkflowDatasetContract(value: unknown): WorkflowDatasetContract | null {
  if (!isRecord(value)) return null;
  if (typeof value.schema_version !== "string" || typeof value.version !== "string") return null;
  if (!isNonEmptyString(value.id)) return null;
  if (!isOptionalString(value.name) || !isOptionalString(value.description)) return null;
  if (!Array.isArray(value.values) || !value.values.every(isWorkflowDatasetValue)) return null;
  if (value.metadata !== undefined && !isStringRecord(value.metadata)) return null;
  return value as WorkflowDatasetContract;
}

export function mergeDatasetContractIntoGraph(
  graph: WorkflowGraphDefinition | null,
  contract: WorkflowDatasetContract,
): WorkflowGraphDefinition | null {
  if (!graph) return null;
  return {
    ...graph,
    dataset_contract: structuredClone(contract),
  };
}

export function normalizeImportedWorkflowGraph(
  graph: WorkflowGraphDefinition | null,
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
) {
  if (!graph) return { graph, autoReconnectEdgeIds: [] as string[], diagnostics: [] as WorkflowImportNormalizationDiagnostic[] };
  const nextGraph = structuredClone(graph) as WorkflowGraphDefinition;
  const autoReconnectEdgeIds = new Set<string>();
  const diagnostics: WorkflowImportNormalizationDiagnostic[] = [];
  const descriptorMap = new Map(operatorDescriptors.map((descriptor) => [descriptor.id, descriptor] as const));

  for (const node of nextGraph.nodes) {
    const operatorId = node.operator_id?.trim();
    if (!operatorId) continue;
    const descriptor = descriptorMap.get(operatorId);
    const impact = getWorkflowNodeTemplateSyncImpact(
      nextGraph,
      node.id,
      {
        kind: node.kind,
        operatorId,
        config:
          node.config && typeof node.config === "object"
            ? { ...(node.config as Record<string, unknown>) }
            : undefined,
      },
      operatorDescriptors,
    );
    for (const edgeId of listAutoReconnectEdgeIds(impact)) autoReconnectEdgeIds.add(edgeId);
    applyWorkflowNodeTemplateSync(
      nextGraph,
      node.id,
      {
        kind: node.kind,
        operatorId,
        config:
          node.config && typeof node.config === "object"
            ? { ...(node.config as Record<string, unknown>) }
            : undefined,
      },
      operatorDescriptors,
    );
    const syncedNode = nextGraph.nodes.find((entry) => entry.id === node.id);
    if (!syncedNode || !syncedNode.operator_id?.startsWith("bridge.")) continue;
    syncedNode.config = normalizeBridgeConfigWithSupport(
      syncedNode.operator_id,
      syncedNode.config as Record<string, unknown> | null | undefined,
      descriptor,
    ) ?? undefined;
    for (const entry of readBridgeNormalizationEntries(syncedNode)) {
      diagnostics.push({
        message: `Bridge contract normalized at ${syncedNode.id}: ${entry.field} ${entry.previous} -> ${entry.next}`,
        locate: { kind: "node", nodeId: syncedNode.id },
      });
    }
  }

  return {
    graph: nextGraph,
    autoReconnectEdgeIds: [...autoReconnectEdgeIds],
    diagnostics,
  };
}
