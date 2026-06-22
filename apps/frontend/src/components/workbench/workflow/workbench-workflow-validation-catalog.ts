"use client";

import type {
  WorkflowCatalogEntryArtifact,
  WorkflowGraphDefinition,
} from "@/lib/api";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-validation-types";
import { buildWorkflowGraphKernelIndex } from "@/lib/workbench/frontend-kernel";

export function validateCatalogArtifacts(
  graph: WorkflowGraphDefinition,
  artifacts: WorkflowCatalogEntryArtifact[],
  mode: "entry" | "output",
): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];
  const { nodeById } = buildWorkflowGraphKernelIndex(graph);

  for (const artifact of artifacts) {
    const node = nodeById.get(artifact.node_id);
    if (!node) {
      issues.push({
        id: `${mode}:missing-node:${artifact.node_id}:${artifact.artifact_type}`,
        level: "warning",
        message: `${mode === "entry" ? "Entry input" : "Output artifact"} "${artifact.node_id}" is not present in the graph.`,
        locate: {
          kind: "artifact",
          mode,
          nodeId: artifact.node_id,
          artifactType: artifact.artifact_type,
        },
      });
      continue;
    }

    const ports = mode === "entry" ? node.inputs ?? [] : node.outputs ?? [];
    if (!ports.some((port) => port.artifact_type === artifact.artifact_type)) {
      issues.push({
        id: `${mode}:missing-artifact:${artifact.node_id}:${artifact.artifact_type}`,
        level: "warning",
        message: `${mode === "entry" ? "Entry input" : "Output artifact"} "${artifact.artifact_type}" is not exposed on node "${artifact.node_id}".`,
        locate: {
          kind: "artifact",
          mode,
          nodeId: artifact.node_id,
          artifactType: artifact.artifact_type,
        },
        fix: ports[0]
          ? {
              kind: "set_catalog_artifact_type",
              mode,
              nodeId: artifact.node_id,
              currentArtifactType: artifact.artifact_type,
              artifactType: ports[0].artifact_type,
            }
          : undefined,
      });
    }
  }

  return issues;
}
