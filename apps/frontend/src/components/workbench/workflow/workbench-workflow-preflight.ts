"use client";

import type {
  WorkflowCatalogEntry,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import type { HeadlessActionContract } from "@/components/workbench/workbench-headless-workflow-contract";
import {
  buildHeadlessWorkflowExecutionBatch,
  type HeadlessWorkflowExecutionBatch,
} from "@/components/workbench/workbench-headless-workflow-export";
import { validateWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-validation";

export type WorkbenchWorkflowPreflightReport = {
  schema_version: "kyuubiki.workbench-workflow-preflight/v1";
  ok: boolean;
  status: "ready" | "blocked";
  workflow_id: string;
  validation_issue_count: number;
  blocking_issue_count: number;
  headless_ready: boolean;
  headless_step_count: number;
  runtime_dispatch_policy: string;
  issues: Array<{
    id: string;
    level: "info" | "warning" | "error";
    message: string;
    source: "workflow_validation" | "headless_export" | "runtime_policy";
  }>;
  headless_batch?: HeadlessWorkflowExecutionBatch;
};

export function buildWorkbenchWorkflowPreflightReport({
  actionMap = new Map(),
  operatorDescriptors = [],
  workflow,
}: {
  actionMap?: Map<string, HeadlessActionContract>;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  workflow: WorkflowCatalogEntry;
}): WorkbenchWorkflowPreflightReport {
  const validationIssues = validateWorkflowGraphDefinition(
    workflow.graph ?? null,
    workflow.entry_inputs,
    workflow.output_artifacts,
    operatorDescriptors,
  );
  const issues: WorkbenchWorkflowPreflightReport["issues"] = validationIssues.map((issue) => ({
    id: issue.id,
    level: issue.level,
    message: issue.message,
    source: "workflow_validation",
  }));
  if (!workflow.graph) {
    issues.push({
      id: "workflow:graph:missing",
      level: "error",
      message: "Workflow is missing a graph definition.",
      source: "workflow_validation",
    });
  }

  const headlessBatch = workflow.graph
    ? buildHeadlessBatchFromWorkflow(workflow, actionMap)
    : undefined;
  if (workflow.graph && !headlessBatch) {
    issues.push({
      id: "headless:export:no-actions",
      level: "warning",
      message: "Workflow has no mapped headless action contract for SDK export.",
      source: "headless_export",
    });
  }

  const runtimeDispatchPolicy = workflow.graph?.dispatch_policy ?? "inline_graph";
  if (runtimeDispatchPolicy === "ui_only") {
    issues.push({
      id: "runtime:dispatch:ui-only",
      level: "error",
      message: "Workflow dispatch policy is UI-only and cannot be used by headless clients.",
      source: "runtime_policy",
    });
  }

  const blockingIssueCount = issues.filter((issue) => issue.level === "error").length;
  const headlessReady = Boolean(headlessBatch && blockingIssueCount === 0);
  const ok = blockingIssueCount === 0;

  return {
    schema_version: "kyuubiki.workbench-workflow-preflight/v1",
    ok,
    status: ok ? "ready" : "blocked",
    workflow_id: workflow.id,
    validation_issue_count: validationIssues.length,
    blocking_issue_count: blockingIssueCount,
    headless_ready: headlessReady,
    headless_step_count: headlessBatch?.steps.length ?? 0,
    runtime_dispatch_policy: runtimeDispatchPolicy,
    issues,
    headless_batch: headlessBatch,
  };
}

function buildHeadlessBatchFromWorkflow(
  workflow: WorkflowCatalogEntry,
  actionMap: Map<string, HeadlessActionContract>,
) {
  const steps = workflow.graph?.nodes
    .filter((node) => node.operator_id && actionMap.has(node.operator_id))
    .map((node) => ({
      action: node.operator_id ?? "",
      payload: {
        node_id: node.id,
        config: node.config ?? {},
      },
    })) ?? [];
  if (steps.length === 0) return undefined;
  return buildHeadlessWorkflowExecutionBatch({
    actionMap,
    draft: {
      id: workflow.id,
      steps,
    },
    language: "en",
  });
}
