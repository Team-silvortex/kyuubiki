"use client";

import type { WorkbenchAuditTimelineEntry } from "@/lib/workbench/workbench-audit-timeline";

export function buildWorkflowAuditReplayPlan(entry: WorkbenchAuditTimelineEntry) {
  const context = entry.context;
  const nodeIds = [
    typeof context?.branchNodeId === "string" ? context.branchNodeId : null,
    typeof context?.nodeId === "string" ? context.nodeId : null,
    typeof context?.artifactNodeId === "string" ? context.artifactNodeId : null,
    typeof context?.edgeFromNodeId === "string" ? context.edgeFromNodeId : null,
    typeof context?.edgeToNodeId === "string" ? context.edgeToNodeId : null,
  ].filter((value): value is string => Boolean(value));
  const edgeIds = [
    typeof context?.edgeId === "string" ? context.edgeId : null,
  ].filter((value): value is string => Boolean(value));
  return {
    edgeIds: [...new Set(edgeIds)],
    nodeIds: [...new Set(nodeIds)],
    nodeId: typeof context?.branchNodeId === "string" ? context.branchNodeId : typeof context?.nodeId === "string" ? context.nodeId : null,
  };
}
