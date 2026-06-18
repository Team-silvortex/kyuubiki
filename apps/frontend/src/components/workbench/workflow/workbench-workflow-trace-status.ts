"use client";

import { resolveWorkflowRunStatusTone } from "@/lib/api";

export type WorkflowTraceStatusTone = "good" | "watch" | "risk";
export type WorkflowTraceNodeRunStatus = "completed" | "skipped";

export function resolveWorkflowTraceContractHealthTone(label: string): WorkflowTraceStatusTone {
  if (label === "clean") return "good";
  if (label.includes("needs review")) return "risk";
  return "watch";
}

export function resolveWorkflowTraceContractWarningTone(count: number): WorkflowTraceStatusTone {
  if (count === 0) return "good";
  if (count <= 3) return "watch";
  return "risk";
}

export function resolveWorkflowTraceNodeRunTone(
  status: WorkflowTraceNodeRunStatus,
): WorkflowTraceStatusTone {
  return status === "completed" ? "good" : "watch";
}

export function resolveWorkflowTraceBranchPredicateTone(
  result: boolean,
): WorkflowTraceStatusTone {
  return result ? "good" : "risk";
}

export function resolveWorkflowTraceLineageSourceTone(
  sourceArtifacts?: string[],
): WorkflowTraceStatusTone {
  return (sourceArtifacts?.length ?? 0) > 0 ? "good" : "watch";
}

export function resolveWorkflowTraceLineageSourceLabel(sourceArtifacts?: string[]) {
  return (sourceArtifacts?.length ?? 0) > 0 ? "derived" : "root";
}

export function resolveWorkflowTraceHeaderHealthLabel(
  staticContractHealth: string,
  dynamicReviewState: string,
) {
  return dynamicReviewState.includes("needs review") ? "review" : staticContractHealth;
}

export function resolveWorkflowTraceProgressStageTone(stage: string): WorkflowTraceStatusTone {
  return resolveWorkflowRunStatusTone(stage);
}
