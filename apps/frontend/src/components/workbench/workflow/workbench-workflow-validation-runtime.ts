"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";
import { isWorkflowNodeSupportedInRuntime } from "@/components/workbench/workflow/workbench-workflow-runtime-support";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-validation-types";

export function validateRuntimeSupport(graph: WorkflowGraphDefinition): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];

  for (const node of graph.nodes) {
    if (isWorkflowNodeSupportedInRuntime(node)) continue;
    issues.push({
      id: `runtime:unsupported:${node.id}`,
      level: "warning",
      message: node.operator_id
        ? `Node "${node.id}" uses operator "${node.operator_id}" which is not supported by the current workflow executor.`
        : `Node "${node.id}" uses node kind "${node.kind}" which is not supported by the current workflow executor.`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  return issues;
}
