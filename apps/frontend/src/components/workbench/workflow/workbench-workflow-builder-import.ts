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
  return typeof value === "object" && value !== null;
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
  if (typeof value.id !== "string") return null;
  if (!Array.isArray(value.nodes)) return null;
  return value as WorkflowGraphDefinition;
}

export function asWorkflowDatasetContract(value: unknown): WorkflowDatasetContract | null {
  if (!isRecord(value)) return null;
  if (typeof value.id !== "string") return null;
  if (!Array.isArray(value.values)) return null;
  return value as WorkflowDatasetContract;
}

export function mergeDatasetContractIntoGraph(
  graph: WorkflowGraphDefinition | null,
  contract: WorkflowDatasetContract,
): WorkflowGraphDefinition | null {
  if (!graph) return null;
  return {
    ...graph,
    dataset_contract: {
      ...contract,
      values: [...contract.values],
      metadata: contract.metadata ? { ...contract.metadata } : {},
    },
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
