"use client";

import type { ProtocolAgentDescriptor, WorkflowGraphDefinition } from "@/lib/api";
import type { WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import type { WorkbenchAuditTimelineEntry } from "@/lib/workbench/workbench-audit-timeline";

export type WorkflowAuditNavigationTarget =
  | { kind: "node"; nodeId: string; label: string }
  | { kind: "dataset"; datasetValueId?: string; label: string }
  | { kind: "run"; jobId: string; label: string };

export type WorkflowAuditFocusHint = {
  artifactMode?: "entry" | "output" | null;
  artifactNodeId?: string | null;
  artifactType?: string | null;
  branchNodeId?: string | null;
  branchOutputId?: string | null;
  datasetValueId?: string | null;
  edgeId?: string | null;
  nodeId?: string | null;
};

export function parseWorkflowArtifactFocusKey(artifactKey: string | null | undefined) {
  if (!artifactKey) return null;
  const [artifactMode, artifactNodeId, ...rest] = artifactKey.split(":");
  if ((artifactMode !== "entry" && artifactMode !== "output") || !artifactNodeId || rest.length < 2) return null;
  return {
    artifactMode,
    artifactNodeId,
    artifactType: rest.slice(0, -1).join(":"),
  } as const;
}

function buildBranchContextFromEdge(
  edgeId: string,
  graph: WorkflowGraphDefinition | null | undefined,
) {
  const edge = graph?.edges?.find((entry) => entry.id === edgeId);
  if (!edge) return { edgeId };
  return {
    edgeId,
    branchNodeId: edge.from.port === "if_true" || edge.from.port === "if_false" ? edge.from.node : null,
    branchOutputId: edge.from.port === "if_true" || edge.from.port === "if_false" ? edge.from.port : null,
    edgeFromNodeId: edge.from.node,
    edgeFromPortId: edge.from.port,
    edgeToNodeId: edge.to.node,
    edgeToPortId: edge.to.port,
  };
}

export function buildWorkflowAuditContextFromValidationIssue(
  issue: WorkflowGraphValidationIssue,
  graph?: WorkflowGraphDefinition | null,
) {
  if (issue.locate?.kind === "node") return { nodeId: issue.locate.nodeId };
  if (issue.locate?.kind === "edge") return buildBranchContextFromEdge(issue.locate.edgeId, graph);
  if (issue.locate?.kind === "dataset") return { datasetValueId: issue.locate.datasetValueId ?? null };
  if (issue.locate?.kind === "artifact") {
    return {
      artifactNodeId: issue.locate.nodeId,
      artifactMode: issue.locate.mode,
      artifactType: issue.locate.artifactType,
    };
  }
  return undefined;
}

export function buildWorkflowAuditContextFromImportDiagnostic(diagnostic: WorkflowPackageImportDiagnostic) {
  if (diagnostic.locate?.kind === "node") return { nodeId: diagnostic.locate.nodeId };
  if (diagnostic.locate?.kind === "dataset") return { datasetValueId: diagnostic.locate.datasetValueId ?? null };
  return undefined;
}

export function resolveWorkflowAuditNavigationTarget(entry: WorkbenchAuditTimelineEntry): WorkflowAuditNavigationTarget | null {
  const context = entry.context;
  if (!context) return null;
  if (typeof context.nodeId === "string" && context.nodeId.length > 0) {
    return { kind: "node", nodeId: context.nodeId, label: `Node ${context.nodeId}` };
  }
  if (typeof context.artifactNodeId === "string" && context.artifactNodeId.length > 0) {
    return { kind: "node", nodeId: context.artifactNodeId, label: `Artifact node ${context.artifactNodeId}` };
  }
  if (typeof context.datasetValueId === "string" && context.datasetValueId.length > 0) {
    return { kind: "dataset", datasetValueId: context.datasetValueId, label: `Dataset ${context.datasetValueId}` };
  }
  if (typeof context.jobId === "string" && context.jobId.length > 0) {
    return { kind: "run", jobId: context.jobId, label: `Run ${context.jobId}` };
  }
  return null;
}

export function resolveWorkflowAuditAgentMatches(
  entry: WorkbenchAuditTimelineEntry,
  protocolAgents: readonly ProtocolAgentDescriptor[],
) {
  const context = entry.context;
  if (!context) return [] as ProtocolAgentDescriptor[];
  return protocolAgents.filter((agent) => {
    if (typeof context.agentId === "string" && agent.id === context.agentId) return true;
    if (typeof context.jobId === "string" && agent.active_lease?.job_id === context.jobId) return true;
    if (typeof context.runtimeMode === "string" && agent.descriptor?.runtime?.runtime_mode === context.runtimeMode) return true;
    if (typeof context.frontendRuntimeMode === "string" && agent.descriptor?.runtime?.runtime_mode === context.frontendRuntimeMode) return true;
    if (typeof context.authorityMode === "string" && agent.descriptor?.authority?.authority_mode === context.authorityMode) return true;
    return false;
  });
}

export function matchesWorkflowAuditFocusHint(
  entry: WorkbenchAuditTimelineEntry,
  focusHint: WorkflowAuditFocusHint | null | undefined,
) {
  if (!focusHint?.nodeId && !focusHint?.datasetValueId && !focusHint?.edgeId && !focusHint?.artifactNodeId && !focusHint?.branchNodeId) return false;
  const context = entry.context;
  if (!context) return false;
  if (focusHint.nodeId && (context.nodeId === focusHint.nodeId || context.artifactNodeId === focusHint.nodeId)) {
    return true;
  }
  if (
    focusHint.branchNodeId &&
    context.branchNodeId === focusHint.branchNodeId &&
    (!focusHint.branchOutputId || context.branchOutputId === focusHint.branchOutputId)
  ) {
    return true;
  }
  if (focusHint.edgeId && context.edgeId === focusHint.edgeId) return true;
  if (
    focusHint.artifactNodeId &&
    context.artifactNodeId === focusHint.artifactNodeId &&
    (!focusHint.artifactMode || context.artifactMode === focusHint.artifactMode) &&
    (!focusHint.artifactType || context.artifactType === focusHint.artifactType)
  ) {
    return true;
  }
  return Boolean(focusHint.datasetValueId && context.datasetValueId === focusHint.datasetValueId);
}
