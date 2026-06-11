"use client";

import type { WorkflowGraphPort } from "@/lib/api";
import { createDefaultWorkflowConditionConfig } from "@/components/workbench/workflow/workbench-workflow-condition";

type WorkflowNodeTemplateControlPreset = {
  id: string;
  kind: string;
  label: string;
  operatorId?: string;
  config?: Record<string, unknown>;
  inputs: WorkflowGraphPort[];
  outputs: WorkflowGraphPort[];
};

export const CONTROL_NODE_TEMPLATE_PRESETS: WorkflowNodeTemplateControlPreset[] = [
  {
    id: "transform.first_available",
    kind: "transform",
    label: "Merge first available branch",
    operatorId: "transform.first_available",
    inputs: [
      { id: "left", artifact_type: "artifact/json", description: "Left branch payload", dataset_value: "json_export" },
      { id: "right", artifact_type: "artifact/json", description: "Right branch payload", dataset_value: "json_export" },
    ],
    outputs: [{ id: "merged", artifact_type: "artifact/json", description: "Merged branch payload", dataset_value: "json_export" }],
  },
  {
    id: "condition.if_else",
    kind: "condition",
    label: "Condition branch",
    config: createDefaultWorkflowConditionConfig(),
    inputs: [{ id: "value", artifact_type: "artifact/json", description: "Value to test", dataset_value: "result_summary" }],
    outputs: [
      { id: "if_true", artifact_type: "artifact/json", description: "Pass-through when predicate is true", dataset_value: "json_export" },
      { id: "if_false", artifact_type: "artifact/json", description: "Pass-through when predicate is false", dataset_value: "json_export" },
    ],
  },
];
