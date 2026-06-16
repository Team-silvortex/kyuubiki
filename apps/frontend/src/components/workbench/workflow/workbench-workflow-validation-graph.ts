"use client";

import type {
  WorkflowDatasetContract,
  WorkflowGraphDefinition,
  WorkflowGraphNode,
  WorkflowGraphPort,
} from "@/lib/api";

export function buildNodeMap(graph: WorkflowGraphDefinition) {
  return new Map(graph.nodes.map((node) => [node.id, node] as const));
}

export function findPort(
  node: WorkflowGraphNode | undefined,
  portId: string,
  direction: "inputs" | "outputs",
): WorkflowGraphPort | undefined {
  return node?.[direction]?.find((port) => port.id === portId);
}

export function hasDatasetValue(
  contract: WorkflowDatasetContract | undefined,
  valueId: string | undefined,
) {
  if (!contract || !valueId) return true;
  return contract.values.some((value) => value.id === valueId);
}
