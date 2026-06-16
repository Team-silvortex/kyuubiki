"use client";

type WorkflowBridgeNormalizationEntry = {
  field: string;
  previous: string;
  next: string;
};

type WorkflowBridgeNormalizationNode = {
  config?: {
    contract_normalization?: unknown;
  } | null;
};

type WorkflowBridgeNormalizationGraph = {
  nodes: WorkflowBridgeNormalizationNode[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function readBridgeNormalizationEntries(node: WorkflowBridgeNormalizationNode) {
  const value = node.config?.contract_normalization;
  if (!Array.isArray(value)) return [] as WorkflowBridgeNormalizationEntry[];
  return value.filter((entry): entry is WorkflowBridgeNormalizationEntry => (
    isRecord(entry) &&
    typeof entry.field === "string" &&
    typeof entry.previous === "string" &&
    typeof entry.next === "string"
  ));
}

export function countWorkflowBridgeNormalizationAdjustments(
  graph: WorkflowBridgeNormalizationGraph | null,
) {
  if (!graph) return 0;
  return graph.nodes.reduce(
    (count, node) => count + readBridgeNormalizationEntries(node).length,
    0,
  );
}
