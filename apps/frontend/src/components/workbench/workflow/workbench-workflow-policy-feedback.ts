"use client";

import { useState } from "react";
import {
  useWorkflowPolicyActionFeedback,
  type WorkflowPolicyAction,
  type WorkflowPolicyActionStatus,
} from "@/components/workbench/workflow/workbench-workflow-policy-actions";

export function useWorkflowPolicyFeedback() {
  const [policyFeedback, setPolicyFeedback] = useState<{
    action: WorkflowPolicyAction;
    status: WorkflowPolicyActionStatus;
    detail: string;
  } | null>(null);

  useWorkflowPolicyActionFeedback((feedback) => {
    setPolicyFeedback(feedback);
    window.setTimeout(() => {
      setPolicyFeedback((current) =>
        current?.action === feedback.action && current?.detail === feedback.detail ? null : current,
      );
    }, 2600);
  });

  return { policyFeedback, setPolicyFeedback };
}
