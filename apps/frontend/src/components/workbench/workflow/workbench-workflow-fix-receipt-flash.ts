"use client";

import type { RefObject } from "react";
import { buildWorkflowFixReceiptHighlightPlan } from "@/components/workbench/workflow/workbench-workflow-fix-receipt-highlight";
import type { WorkflowValidationFixSummaryEntry } from "@/components/workbench/workflow/workbench-workflow-validation-summary";

type FlashWorkflowFixReceiptHighlightsArgs = {
  builderRootRef: RefObject<HTMLElement | null>;
  summary: WorkflowValidationFixSummaryEntry[];
  setFocusedNodeId: (value: string | null) => void;
  setFocusedEdgeId: (value: string | null) => void;
  setFocusedArtifactKey: (value: string | null) => void;
  setHighlightedNodeIds: (value: string[]) => void;
  setHighlightedEdgeIds: (value: string[]) => void;
  setHighlightedPortKeys: (value: string[]) => void;
  setHighlightedArtifactKeys: (value: string[]) => void;
};

export function flashWorkflowFixReceiptHighlights(args: FlashWorkflowFixReceiptHighlightsArgs) {
  const plan = buildWorkflowFixReceiptHighlightPlan(args.summary);
  if (!plan.firstNodeId && !plan.firstEdgeId && !plan.firstArtifactKey && plan.portKeys.length === 0) return;
  if (plan.firstNodeId) args.setFocusedNodeId(plan.firstNodeId);
  if (plan.firstEdgeId) args.setFocusedEdgeId(plan.firstEdgeId);
  if (plan.firstArtifactKey) args.setFocusedArtifactKey(plan.firstArtifactKey);
  args.setHighlightedNodeIds(plan.nodeIds);
  args.setHighlightedEdgeIds(plan.edgeIds);
  args.setHighlightedPortKeys(plan.portKeys);
  args.setHighlightedArtifactKeys(plan.artifactKeys);
  queueMicrotask(() => {
    const root = args.builderRootRef.current;
    const target = plan.firstNodeId ? root?.querySelector<HTMLElement>(`[data-workflow-node-id="${plan.firstNodeId}"]`) : plan.firstEdgeId ? root?.querySelector<HTMLElement>(`[data-workflow-edge-id="${plan.firstEdgeId}"]`) : plan.firstArtifactKey ? root?.querySelector<HTMLElement>(`[data-workflow-artifact-key="${plan.firstArtifactKey}"]`) : plan.portKeys[0] ? root?.querySelector<HTMLElement>(`[data-workflow-port-field="${plan.portKeys[0]}:artifact_type"]`) : null;
    target?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });
  window.setTimeout(() => {
    args.setHighlightedNodeIds([]);
    args.setHighlightedEdgeIds([]);
    args.setHighlightedPortKeys([]);
    args.setHighlightedArtifactKeys([]);
  }, 2200);
}
