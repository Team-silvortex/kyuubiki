"use client";

import type { WorkflowBuilderFocusTarget } from "@/components/workbench/workflow/workbench-workflow-builder-focus";
import type { WorkflowPolicyAction, WorkflowPolicyActionStatus } from "@/components/workbench/workflow/workbench-workflow-policy-actions";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowFocusStripProps = {
  labels: WorkflowSidebarLabels;
  activeTarget: WorkflowBuilderFocusTarget | null;
  feedback: { action: WorkflowPolicyAction; status: WorkflowPolicyActionStatus; detail: string } | null;
};

function resolveWorkflowFocusTargetLabel(labels: WorkflowSidebarLabels, activeTarget: WorkflowBuilderFocusTarget) {
  switch (activeTarget) {
    case "validation":
      return labels.validationTitle;
    case "package-policy":
      return labels.packageInstallRulesTitle;
    case "snapshots":
      return labels.validationSnapshotsTitle;
  }
}

export function WorkbenchWorkflowFocusStrip({
  labels,
  activeTarget,
  feedback,
}: WorkbenchWorkflowFocusStripProps) {
  if (!activeTarget && !feedback) return null;
  const targetLabel = activeTarget ? resolveWorkflowFocusTargetLabel(labels, activeTarget) : labels.packageInstallRulesTitle;
  const feedbackTone = feedback?.status === "ready" ? "good" : "watch";

  return (
    <section className="workflow-focus-strip" data-workflow-focus-strip="active">
      <div className="card-head">
        <h2>{labels.focusStripTitle}</h2>
        <span className={`status-pill status-pill--${feedback ? feedbackTone : "watch"}`}>{targetLabel}</span>
      </div>
      <div className="button-row">
        <span className="ghost-button ghost-button--compact">{labels.focusStripFromPolicyLabel}</span>
        <span className="ghost-button ghost-button--compact">{labels.builderPageLabel}</span>
        <span className="ghost-button ghost-button--compact ghost-button--active">
          {targetLabel}
        </span>
        {feedback ? <span className="ghost-button ghost-button--compact">{feedback.detail}</span> : null}
      </div>
    </section>
  );
}
