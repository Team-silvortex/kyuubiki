"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";
import {
  conditionOperatorNeedsValue,
  isWorkflowConditionNode,
  resolveWorkflowConditionConfig,
  WORKFLOW_CONDITION_OPERATORS,
} from "@/components/workbench/workflow/workbench-workflow-condition";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-validation-types";

export function validateConditionNodes(graph: WorkflowGraphDefinition): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];

  for (const node of graph.nodes) {
    if (!isWorkflowConditionNode(node)) continue;
    const config = resolveWorkflowConditionConfig(
      node.config as Record<string, unknown> | null | undefined,
    );
    const predicate = config.predicate ?? {};
    const operator = predicate.operator ?? "gt";

    if (!WORKFLOW_CONDITION_OPERATORS.includes(operator)) {
      issues.push({
        id: `condition:operator:${node.id}`,
        level: "warning",
        message: `Condition node "${node.id}" uses unsupported operator "${String(predicate.operator ?? "")}".`,
        locate: { kind: "node", nodeId: node.id },
      });
    }
    if (conditionOperatorNeedsValue(operator) && predicate.value === undefined) {
      issues.push({
        id: `condition:value:${node.id}`,
        level: "warning",
        message: `Condition node "${node.id}" requires a comparison value for operator "${operator}".`,
        locate: { kind: "node", nodeId: node.id },
      });
    }
    if ((node.inputs ?? []).length === 0) {
      issues.push({
        id: `condition:inputs:${node.id}`,
        level: "warning",
        message: `Condition node "${node.id}" should expose one input port to evaluate.`,
        locate: { kind: "node", nodeId: node.id },
      });
    }
    if ((node.outputs ?? []).length < 2) {
      issues.push({
        id: `condition:outputs:${node.id}`,
        level: "warning",
        message: `Condition node "${node.id}" should expose true/false output ports.`,
        locate: { kind: "node", nodeId: node.id },
      });
    }
  }

  return issues;
}
