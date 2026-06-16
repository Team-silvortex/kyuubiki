"use client";

import type {
  WorkflowCatalogEntryArtifact,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import { validateBridgeNodes } from "@/components/workbench/workflow/workbench-workflow-bridge-validation";
import { buildPortsForWorkflowNodeTemplate } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { applyWorkflowNodeTemplateSync } from "@/components/workbench/workflow/workbench-workflow-template-impact";
import { validateCatalogArtifacts } from "@/components/workbench/workflow/workbench-workflow-validation-catalog";
import { validateConditionNodes } from "@/components/workbench/workflow/workbench-workflow-validation-condition";
import { buildNodeMap } from "@/components/workbench/workflow/workbench-workflow-validation-graph";
import { validateEdgeAndDatasetReferences } from "@/components/workbench/workflow/workbench-workflow-validation-edge";
import { applyWorkflowGraphFix } from "@/components/workbench/workflow/workbench-workflow-validation-fix";
import { validateOperatorDescriptorContracts } from "@/components/workbench/workflow/workbench-workflow-validation-operator-descriptor";
import { validateRuntimeSupport } from "@/components/workbench/workflow/workbench-workflow-validation-runtime";
import type {
  WorkflowGraphValidationIssue,
  WorkflowValidationFixBatchResult,
} from "@/components/workbench/workflow/workbench-workflow-validation-types";
import { normalizeBridgeConfigWithSupport } from "@/lib/workbench/workflow-bridge-contract-support";

export type {
  WorkflowGraphValidationIssue,
  WorkflowValidationFixBatchResult,
} from "@/components/workbench/workflow/workbench-workflow-validation-types";

export function validateWorkflowGraphDefinition(
  graph: WorkflowGraphDefinition | null,
  entryInputs: WorkflowCatalogEntryArtifact[],
  outputArtifacts: WorkflowCatalogEntryArtifact[],
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
): WorkflowGraphValidationIssue[] {
  if (!graph) return [];
  const issues: WorkflowGraphValidationIssue[] = [];
  const nodeMap = buildNodeMap(graph);

  for (const nodeId of graph.entry_nodes ?? []) {
    if (!nodeMap.has(nodeId)) {
      issues.push({
        id: `entry-node:${nodeId}`,
        level: "warning",
        message: `Entry node "${nodeId}" is not present in the graph.`,
        locate: { kind: "node", nodeId },
      });
    }
  }

  for (const nodeId of graph.output_nodes ?? []) {
    if (!nodeMap.has(nodeId)) {
      issues.push({
        id: `output-node:${nodeId}`,
        level: "warning",
        message: `Output node "${nodeId}" is not present in the graph.`,
        locate: { kind: "node", nodeId },
      });
    }
  }

  issues.push(...validateEdgeAndDatasetReferences(graph));
  issues.push(...validateRuntimeSupport(graph));
  const bridgeIssues = validateBridgeNodes(graph, operatorDescriptors);
  issues.push(
    ...bridgeIssues
      .filter((issue) => !issue.id.startsWith("bridge:contract:"))
      .map((issue) => {
        if (issue.id !== `bridge:seed-model:${issue.locate.nodeId}`) return issue;
        const bridgeNode = nodeMap.get(issue.locate.nodeId);
        return {
          ...issue,
          fix: bridgeNode?.operator_id
            ? {
                kind: "sync_node_template_from_operator" as const,
                nodeId: issue.locate.nodeId,
                operatorId: bridgeNode.operator_id,
                templateKind: bridgeNode.kind,
              }
            : undefined,
        };
      }),
  );
  issues.push(
    ...bridgeIssues
      .filter((issue) => issue.id.startsWith("bridge:contract:"))
      .map((issue) => {
        const bridgeNode = nodeMap.get(issue.locate.nodeId);
        return {
          ...issue,
          fix: bridgeNode?.operator_id
            ? {
                kind: "normalize_bridge_contract_from_support" as const,
                nodeId: issue.locate.nodeId,
                operatorId: bridgeNode.operator_id,
              }
            : undefined,
        };
      }),
  );
  issues.push(...validateCatalogArtifacts(graph, entryInputs, "entry"));
  issues.push(...validateCatalogArtifacts(graph, outputArtifacts, "output"));
  issues.push(...validateOperatorDescriptorContracts(graph, operatorDescriptors));
  issues.push(...validateConditionNodes(graph));

  return issues;
}

export function applyWorkflowValidationFix(
  graph: WorkflowGraphDefinition | null,
  issue: WorkflowGraphValidationIssue | undefined,
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
): WorkflowGraphDefinition | null {
  if (!graph || !issue?.fix) return graph;
  const next = structuredClone(graph) as WorkflowGraphDefinition;
  const fix = issue.fix;

  switch (fix.kind) {
    case "sync_node_template_from_operator": {
      const currentNode = next.nodes.find((entry) => entry.id === fix.nodeId);
      applyWorkflowNodeTemplateSync(
        next,
        fix.nodeId,
        {
          kind: fix.templateKind,
          operatorId: fix.operatorId,
          config:
            currentNode?.config && typeof currentNode.config === "object"
              ? { ...(currentNode.config as Record<string, unknown>) }
              : undefined,
        },
        operatorDescriptors,
      );
      break;
    }
    case "normalize_bridge_contract_from_support": {
      const node = next.nodes.find((entry) => entry.id === fix.nodeId);
      const descriptor = operatorDescriptors.find((entry) => entry.id === fix.operatorId);
      if (!node || !descriptor?.contract_support) break;
      node.config = normalizeBridgeConfigWithSupport(
        fix.operatorId,
        node.config as Record<string, unknown> | null | undefined,
        descriptor,
      ) ?? undefined;
      break;
    }
    case "set_edge_artifact_type_from_source":
    case "set_edge_artifact_type_from_target": {
      return applyWorkflowGraphFix(next, fix as never) as WorkflowGraphDefinition;
    }
    case "set_catalog_artifact_type": {
      return applyWorkflowGraphFix(next, fix as never) as WorkflowGraphDefinition;
    }
    case "set_node_port_artifact_type_from_operator":
    case "set_node_port_dataset_value_from_operator":
    case "clear_port_dataset_value": {
      return applyWorkflowGraphFix(next, fix as never) as WorkflowGraphDefinition;
    }
    case "clear_edge_dataset_value": {
      return applyWorkflowGraphFix(next, fix as never) as WorkflowGraphDefinition;
    }
  }

  return next;
}

export function applyAllWorkflowValidationFixes(
  graph: WorkflowGraphDefinition | null,
  entryInputs: WorkflowCatalogEntryArtifact[],
  outputArtifacts: WorkflowCatalogEntryArtifact[],
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
) : WorkflowValidationFixBatchResult {
  let current = graph;
  let appliedCount = 0;
  const appliedIssues: WorkflowGraphValidationIssue[] = [];

  for (let iteration = 0; iteration < 24; iteration += 1) {
    const issues = validateWorkflowGraphDefinition(
      current,
      entryInputs,
      outputArtifacts,
      operatorDescriptors,
    );
    const nextIssue = issues.find((issue) => issue.fix);
    if (!nextIssue) break;

    const next = applyWorkflowValidationFix(current, nextIssue, operatorDescriptors);
    if (!next || JSON.stringify(next) === JSON.stringify(current)) break;
    current = next;
    appliedCount += 1;
    appliedIssues.push(nextIssue);
  }

  return { graph: current, appliedCount, appliedIssues };
}
