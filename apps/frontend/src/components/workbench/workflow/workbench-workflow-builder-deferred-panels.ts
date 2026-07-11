"use client";

import { useEffect, useState } from "react";
import {
  scheduleWorkflowDeferredRender,
  WORKFLOW_BUILDER_DEFERRED_PANEL_DELAY_MS,
} from "@/components/workbench/workflow/workbench-workflow-render-budget";

export function useWorkflowBuilderDeferredPanels(workflowId?: string | null) {
  const [showDeferredPanels, setShowDeferredPanels] = useState(false);

  useEffect(() => {
    setShowDeferredPanels(false);
    return scheduleWorkflowDeferredRender(
      () => setShowDeferredPanels(true),
      WORKFLOW_BUILDER_DEFERRED_PANEL_DELAY_MS,
    );
  }, [workflowId]);

  return showDeferredPanels;
}
