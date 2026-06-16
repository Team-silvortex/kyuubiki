"use client";

import type { WorkflowGraphNode } from "@/lib/api";

export function pickConnectedPorts(
  sourceNode: WorkflowGraphNode,
  nextNode: WorkflowGraphNode,
) {
  const sourceOutputs = sourceNode.outputs ?? [];
  const targetInputs = nextNode.inputs ?? [];

  for (const sourcePort of sourceOutputs) {
    const datasetMatch = targetInputs.find(
      (port) =>
        sourcePort.dataset_value &&
        port.dataset_value &&
        port.dataset_value === sourcePort.dataset_value,
    );
    if (datasetMatch) {
      return { sourcePort, targetPort: datasetMatch };
    }
  }

  for (const sourcePort of sourceOutputs) {
    const artifactMatch = targetInputs.find(
      (port) => port.artifact_type === sourcePort.artifact_type,
    );
    if (artifactMatch) {
      return { sourcePort, targetPort: artifactMatch };
    }
  }

  return {
    sourcePort: sourceOutputs[0],
    targetPort: targetInputs[0],
  };
}
