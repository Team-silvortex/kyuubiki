"use client";

import type {
  WorkflowDatasetContract,
  WorkflowGraphDefinition,
} from "@/lib/api";

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
