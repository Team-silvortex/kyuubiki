"use client";

import type { WorkflowValidationFixSummaryEntry } from "@/components/workbench/workflow/workbench-workflow-validation-summary";

export type WorkflowFixReceiptHighlightPlan = {
  nodeIds: string[];
  edgeIds: string[];
  portKeys: string[];
  artifactKeys: string[];
  firstNodeId: string | null;
  firstEdgeId: string | null;
  firstArtifactKey: string | null;
};

function pushAllUnique(target: string[], values: string[]) {
  for (const value of values) {
    if (!target.includes(value)) target.push(value);
  }
}

export function buildWorkflowFixReceiptHighlightPlan(
  entries: WorkflowValidationFixSummaryEntry[],
): WorkflowFixReceiptHighlightPlan {
  const nodeIds: string[] = [];
  const edgeIds: string[] = [];
  const portKeys: string[] = [];
  const artifactKeys: string[] = [];

  for (const entry of entries) {
    pushAllUnique(nodeIds, entry.nodeIds);
    pushAllUnique(edgeIds, entry.edgeIds);
    pushAllUnique(portKeys, entry.portKeys);
    pushAllUnique(artifactKeys, entry.artifactKeys);
  }

  return {
    nodeIds,
    edgeIds,
    portKeys,
    artifactKeys,
    firstNodeId: nodeIds[0] ?? null,
    firstEdgeId: edgeIds[0] ?? null,
    firstArtifactKey: artifactKeys[0] ?? null,
  };
}
