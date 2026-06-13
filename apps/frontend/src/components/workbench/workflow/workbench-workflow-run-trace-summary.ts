"use client";

import type { WorkflowGraphJobResult, WorkflowProgressEvent } from "@/lib/api";

export type WorkflowRunTraceProgressItem = {
  stage: WorkflowProgressEvent["stage"];
  progress: number;
  label: string | null;
  nodeId?: string | null;
  kind?: string | null;
  emittedAt?: string | null;
};

export type WorkflowRunTraceSummary = {
  branchDecisionCount: number;
  completedNodeRunCount: number;
  skippedNodeRunCount: number;
  rootArtifactCount: number;
  derivedArtifactCount: number;
  progressEventCount: number;
  latestProgressLabel?: string | null;
  recentProgressEvents: WorkflowRunTraceProgressItem[];
  latestBranchPredicate?: boolean | null;
};

function formatProgressEventLabel(event: WorkflowProgressEvent) {
  const candidates = [
    event.stage,
    event.kind,
    event.message,
    event.node_id,
  ];
  const value = candidates.find((entry) => typeof entry === "string" && entry.trim().length > 0);
  return typeof value === "string" ? value : null;
}

function toProgressItem(event: WorkflowProgressEvent): WorkflowRunTraceProgressItem {
  return {
    stage: event.stage,
    progress: event.progress,
    label: formatProgressEventLabel(event),
    nodeId: event.node_id,
    kind: event.kind,
    emittedAt: event.emitted_at,
  };
}

export function summarizeWorkflowRunTrace(
  result?: WorkflowGraphJobResult | null,
): WorkflowRunTraceSummary | undefined {
  if (!result) return undefined;
  const nodeRuns = result.node_runs ?? [];
  const artifactLineage = result.artifact_lineage ?? [];
  const branchDecisions = result.branch_decisions ?? [];
  const progressEvents = result.progress_events ?? [];
  let completedNodeRunCount = 0;
  let skippedNodeRunCount = 0;
  let rootArtifactCount = 0;
  let derivedArtifactCount = 0;
  for (const entry of nodeRuns) {
    if (entry.status === "completed") completedNodeRunCount += 1;
    if (entry.status === "skipped") skippedNodeRunCount += 1;
  }
  for (const entry of artifactLineage) {
    if ((entry.source_artifacts?.length ?? 0) === 0) rootArtifactCount += 1;
    else derivedArtifactCount += 1;
  }
  return {
    branchDecisionCount: branchDecisions.length,
    completedNodeRunCount,
    skippedNodeRunCount,
    rootArtifactCount,
    derivedArtifactCount,
    progressEventCount: progressEvents.length,
    latestProgressLabel:
      progressEvents.length > 0
        ? formatProgressEventLabel(progressEvents[progressEvents.length - 1] ?? {})
        : null,
    recentProgressEvents: progressEvents.slice(-6).reverse().map(toProgressItem),
    latestBranchPredicate: branchDecisions[branchDecisions.length - 1]?.predicate_result ?? null,
  };
}
