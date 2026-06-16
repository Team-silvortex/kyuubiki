"use client";

import type { WorkflowFemInputSection } from "@/components/workbench/workflow/workbench-workflow-fem-input-profile";

export type WorkflowFemValidationIssue = {
  category: "physics" | "contract";
  field: string;
  message: string;
  severity: "warning";
  sectionKey: WorkflowFemInputSection["key"];
};

export function createWorkflowFemValidationIssue(
  category: WorkflowFemValidationIssue["category"],
  field: string,
  message: string,
  sectionKey: WorkflowFemInputSection["key"],
): WorkflowFemValidationIssue {
  return { category, field, message, severity: "warning", sectionKey };
}

export function dedupeWorkflowFemValidationIssues(
  issues: WorkflowFemValidationIssue[],
) {
  const seen = new Set<string>();
  return issues.filter((entry) => {
    const key = `${entry.category}:${entry.sectionKey}:${entry.field}:${entry.message}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}
