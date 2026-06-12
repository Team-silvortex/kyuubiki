"use client";

import { requestWorkflowBuilderFocus, type WorkflowBuilderFocusTarget } from "@/components/workbench/workflow/workbench-workflow-builder-focus";
import { requestWorkflowPolicyAction, type WorkflowPolicyAction } from "@/components/workbench/workflow/workbench-workflow-policy-actions";
import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import type { WorkflowSurfaceTab } from "@/components/workbench/workflow/workbench-workflow-types";

export function buildSystemWorkflowPolicyAction(
  t: WorkbenchCopy,
  setSidebarSection: (section: "study" | "model" | "workflow" | "library" | "system") => void,
  handleWorkflowPanelTabChange: (tab: WorkflowSurfaceTab) => void,
  target: WorkflowBuilderFocusTarget,
  tone: "good" | "watch",
) {
  return {
    label: t.workflowValidationLocateLabel,
    onClick: () => {
      setSidebarSection("workflow");
      handleWorkflowPanelTabChange("builder");
      window.setTimeout(() => requestWorkflowBuilderFocus(target), 0);
    },
    target: `${t.sections.workflow} / ${t.workflowBuilderPage}`,
    tone,
  };
}

export function buildSystemWorkflowPolicyCommandAction(
  t: WorkbenchCopy,
  setSidebarSection: (section: "study" | "model" | "workflow" | "library" | "system") => void,
  handleWorkflowPanelTabChange: (tab: WorkflowSurfaceTab) => void,
  focusTarget: WorkflowBuilderFocusTarget,
  action: WorkflowPolicyAction,
  label: string,
) {
  return {
    label,
    onClick: () => {
      setSidebarSection("workflow");
      handleWorkflowPanelTabChange("builder");
      window.setTimeout(() => {
        requestWorkflowBuilderFocus(focusTarget);
        requestWorkflowPolicyAction(action);
      }, 0);
    },
  };
}
