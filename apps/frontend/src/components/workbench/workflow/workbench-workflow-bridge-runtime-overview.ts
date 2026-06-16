"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import { summarizeWorkflowBridgeRuntimeStatuses } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-validation";
import type { WorkflowRunRecord } from "@/components/workbench/workflow/workbench-workflow-types";

export type WorkflowBridgeRuntimeFilterState =
  | "bridge_aligned"
  | "bridge_drift"
  | "bridge_missing_runtime";

export function resolveBridgeRuntimeOverview(
  workflow: WorkflowCatalogEntry | null | undefined,
  run?: WorkflowRunRecord | null,
): { summary: string; tone: "good" | "watch" | "risk" } | null {
  if (!workflow?.graph || !run?.result) return null;
  const counts = summarizeWorkflowBridgeRuntimeStatuses(workflow.graph, run.result);
  const total = counts.aligned + counts.drift + counts["missing-runtime"];
  if (total === 0) return null;
  return { summary: `${counts.aligned}/${counts.drift}/${counts["missing-runtime"]}`, tone: counts.drift > 0 ? "watch" : counts["missing-runtime"] > 0 ? "risk" : "good" };
}

export function resolveBridgeRuntimeFilterState(
  workflow: WorkflowCatalogEntry | null | undefined,
  run?: WorkflowRunRecord | null,
): WorkflowBridgeRuntimeFilterState | null {
  if (!workflow?.graph || !run?.result) return null;
  const counts = summarizeWorkflowBridgeRuntimeStatuses(workflow.graph, run.result);
  const total = counts.aligned + counts.drift + counts["missing-runtime"];
  if (total === 0) return null;
  if (counts.drift > 0) return "bridge_drift";
  if (counts["missing-runtime"] > 0) return "bridge_missing_runtime";
  return "bridge_aligned";
}

export function rankBridgeRuntimeState(state: WorkflowBridgeRuntimeFilterState | null) {
  return state === "bridge_drift" ? 2 : state === "bridge_missing_runtime" ? 1 : 0;
}

export function summarizeBridgeRuntimeStates(
  workflows: WorkflowCatalogEntry[],
  latestRunByWorkflowId: Map<string, WorkflowRunRecord>,
) {
  return workflows.reduce((summary, workflow) => {
    const state = resolveBridgeRuntimeFilterState(workflow, latestRunByWorkflowId.get(workflow.id));
    if (state === "bridge_aligned") summary.aligned += 1;
    else if (state === "bridge_drift") summary.drift += 1;
    else if (state === "bridge_missing_runtime") summary.missing += 1;
    return summary;
  }, { aligned: 0, drift: 0, missing: 0 });
}
