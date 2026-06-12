"use client";

import type { WorkflowBuilderFocusTarget } from "@/components/workbench/workflow/workbench-workflow-builder-focus";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowFocusStripProps = {
  labels: WorkflowSidebarLabels;
  activeTarget: WorkflowBuilderFocusTarget | null;
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
}: WorkbenchWorkflowFocusStripProps) {
  if (!activeTarget) return null;

  return (
    <section className="workflow-focus-strip" data-workflow-focus-strip="active">
      <div className="card-head">
        <h2>{labels.focusStripTitle}</h2>
        <span className="status-pill status-pill--watch">{resolveWorkflowFocusTargetLabel(labels, activeTarget)}</span>
      </div>
      <div className="button-row">
        <span className="ghost-button ghost-button--compact">{labels.focusStripFromPolicyLabel}</span>
        <span className="ghost-button ghost-button--compact">{labels.builderPageLabel}</span>
        <span className="ghost-button ghost-button--compact ghost-button--active">
          {resolveWorkflowFocusTargetLabel(labels, activeTarget)}
        </span>
      </div>
    </section>
  );
}
