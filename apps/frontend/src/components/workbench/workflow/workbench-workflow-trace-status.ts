"use client";

import { resolveWorkflowRunStatusTone } from "@/lib/api/job-status";

export type WorkflowTraceStatusTone = "good" | "watch" | "risk";
export type WorkflowTraceNodeRunStatus = "completed" | "skipped";

export function resolveWorkflowTraceContractHealthTone(label: string): WorkflowTraceStatusTone {
  if (label === "clean") return "good";
  if (label.includes("needs review")) return "risk";
  return "watch";
}

export function resolveWorkflowTraceContractWarningTone(count: number): WorkflowTraceStatusTone {
  if (!Number.isInteger(count) || count < 0) return "risk";
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
  return hasWorkflowTraceLineageSource(sourceArtifacts) ? "good" : "watch";
}

export function resolveWorkflowTraceLineageSourceLabel(sourceArtifacts?: string[]) {
  return hasWorkflowTraceLineageSource(sourceArtifacts) ? "derived" : "root";
}

function hasWorkflowTraceLineageSource(sourceArtifacts?: string[]) {
  return sourceArtifacts?.some(
    (artifact) => typeof artifact === "string" && artifact.trim().length > 0,
  ) ?? false;
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
