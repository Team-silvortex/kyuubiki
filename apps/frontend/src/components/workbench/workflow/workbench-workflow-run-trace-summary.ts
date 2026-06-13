"use client";

import type { WorkflowGraphJobResult } from "@/lib/api";

export type WorkflowRunTraceSummary = {
  branchDecisionCount: number;
  completedNodeRunCount: number;
  skippedNodeRunCount: number;
  rootArtifactCount: number;
  derivedArtifactCount: number;
  progressEventCount: number;
  latestProgressLabel?: string | null;
  latestBranchPredicate?: boolean | null;
};

function formatProgressEventLabel(event: Record<string, unknown>) {
  const candidates = [
    event.phase,
    event.stage,
    event.kind,
    event.type,
    event.message,
    event.node_id,
  ];
  const value = candidates.find((entry) => typeof entry === "string" && entry.trim().length > 0);
  return typeof value === "string" ? value : null;
}

export function summarizeWorkflowRunTrace(
  result?: WorkflowGraphJobResult | null,
): WorkflowRunTraceSummary | undefined {
  if (!result) return undefined;
  const nodeRuns = result.node_runs ?? [];
  const artifactLineage = result.artifact_lineage ?? [];
  const branchDecisions = result.branch_decisions ?? [];
  const progressEvents = result.progress_events ?? [];
  return {
    branchDecisionCount: branchDecisions.length,
    completedNodeRunCount: nodeRuns.filter((entry) => entry.status === "completed").length,
    skippedNodeRunCount: nodeRuns.filter((entry) => entry.status === "skipped").length,
    rootArtifactCount: artifactLineage.filter((entry) => (entry.source_artifacts?.length ?? 0) === 0).length,
    derivedArtifactCount: artifactLineage.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length,
    progressEventCount: progressEvents.length,
    latestProgressLabel:
      progressEvents.length > 0
        ? formatProgressEventLabel(progressEvents[progressEvents.length - 1] ?? {})
        : null,
    latestBranchPredicate: branchDecisions[branchDecisions.length - 1]?.predicate_result ?? null,
  };
}
