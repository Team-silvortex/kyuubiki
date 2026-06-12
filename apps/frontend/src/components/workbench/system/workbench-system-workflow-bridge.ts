"use client";

import { requestWorkflowBuilderFocus, type WorkflowBuilderFocusTarget } from "@/components/workbench/workflow/workbench-workflow-builder-focus";
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
