"use client";

import type { WorkflowGraphNode } from "@/lib/api";

export const WORKFLOW_CONDITION_OPERATORS = [
  "truthy",
  "falsy",
  "eq",
  "neq",
  "gt",
  "gte",
  "lt",
  "lte",
  "contains",
] as const;

export type WorkflowConditionOperator =
  (typeof WORKFLOW_CONDITION_OPERATORS)[number];

export type WorkflowConditionPredicate = {
  path?: string;
  operator?: WorkflowConditionOperator;
  value?: unknown;
};

export type WorkflowConditionConfig = {
  predicate?: WorkflowConditionPredicate;
};

export function isWorkflowConditionNode(node?: Pick<WorkflowGraphNode, "kind"> | null) {
  return node?.kind === "condition";
}

export function createDefaultWorkflowConditionConfig(): WorkflowConditionConfig {
  return {
    predicate: {
      path: "max_displacement",
      operator: "gt",
      value: 0,
    },
  };
}

export function resolveWorkflowConditionConfig(
  config?: Record<string, unknown> | null,
): WorkflowConditionConfig {
  const predicate =
    config?.predicate && typeof config.predicate === "object"
      ? (config.predicate as Record<string, unknown>)
      : {};
  const operator = WORKFLOW_CONDITION_OPERATORS.includes(
    predicate.operator as WorkflowConditionOperator,
  )
    ? (predicate.operator as WorkflowConditionOperator)
    : "gt";

  return {
    predicate: {
      path: typeof predicate.path === "string" ? predicate.path : "max_displacement",
      operator,
      value: "value" in predicate ? predicate.value : 0,
    },
  };
}

export function conditionOperatorNeedsValue(operator?: string | null) {
  return operator !== "truthy" && operator !== "falsy";
}

export function parseWorkflowConditionValue(text: string): unknown {
  const trimmed = text.trim();
  if (!trimmed) return "";
  try {
    return JSON.parse(trimmed);
  } catch {
    return trimmed;
  }
}

export function formatWorkflowConditionValue(value: unknown) {
  if (typeof value === "string") return value;
  if (value === undefined) return "";
  return JSON.stringify(value);
}
