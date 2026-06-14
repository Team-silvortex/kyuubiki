"use client";

import type {
  WorkflowDatasetContract,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import {
  applyWorkflowNodeTemplateSync,
  getWorkflowNodeTemplateSyncImpact,
  listAutoReconnectEdgeIds,
} from "@/components/workbench/workflow/workbench-workflow-template-impact";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

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
  if (!graph) return { graph, autoReconnectEdgeIds: [] as string[] };
  const nextGraph = structuredClone(graph) as WorkflowGraphDefinition;
  const autoReconnectEdgeIds = new Set<string>();

  for (const node of nextGraph.nodes) {
    const operatorId = node.operator_id?.trim();
    if (!operatorId) continue;
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
  }

  return {
    graph: nextGraph,
    autoReconnectEdgeIds: [...autoReconnectEdgeIds],
  };
}
