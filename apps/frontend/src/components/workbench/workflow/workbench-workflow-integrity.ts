"use client";

import { findStoredLocalWorkflow } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { listStoredWorkflowSnapshots } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";
import type {
  WorkflowCatalogEntry,
  WorkflowDatasetContract,
  WorkflowGraphDefinition,
  WorkflowGraphNode,
  WorkflowGraphPort,
  WorkflowOperatorDescriptor,
} from "@/lib/api";

export type WorkflowIntegrityIssue = {
  id: string;
  scope: "graph" | "dataset" | "operator" | "snapshot" | "local";
  severity: "warning";
  message: string;
  detail?: string;
  locate?:
    | { kind: "node"; nodeId: string }
    | { kind: "edge"; edgeId: string }
    | { kind: "dataset"; datasetValueId?: string }
    | { kind: "snapshot" }
    | { kind: "local" };
};

export type WorkflowIntegrityReport = {
  issues: WorkflowIntegrityIssue[];
  snapshotCount: number;
  summaryOnlySnapshotCount: number;
  localWorkflowFound: boolean;
};

function collectPortDatasetValues(node: WorkflowGraphNode) {
  return [...(node.inputs ?? []), ...(node.outputs ?? [])]
    .map((port) => port.dataset_value?.trim())
    .filter((value): value is string => Boolean(value));
}

function portExists(node: WorkflowGraphNode | undefined, portId: string, direction: "inputs" | "outputs") {
  return Boolean(node?.[direction]?.some((port) => port.id === portId));
}

function buildNodeMap(graph: WorkflowGraphDefinition) {
  return new Map(graph.nodes.map((node) => [node.id, node] as const));
}

function buildDatasetValueMap(contract?: WorkflowDatasetContract | null) {
  return new Map((contract?.values ?? []).map((value) => [value.id, value] as const));
}

function validateGraphStructure(graph: WorkflowGraphDefinition) {
  const issues: WorkflowIntegrityIssue[] = [];
  const nodeIds = new Set<string>();
  const edgeIds = new Set<string>();
  const nodeMap = buildNodeMap(graph);
  for (const node of graph.nodes) {
    if (nodeIds.has(node.id)) {
      issues.push({
        id: `graph:duplicate-node:${node.id}`,
        scope: "graph",
        severity: "warning",
        message: `Duplicate node id "${node.id}" detected.`,
        locate: { kind: "node", nodeId: node.id },
      });
    }
    nodeIds.add(node.id);
    if ((node.kind === "solve" || node.kind === "transform" || node.kind === "extract" || node.kind === "export") && !node.operator_id) {
      issues.push({
        id: `graph:missing-operator:${node.id}`,
        scope: "graph",
        severity: "warning",
        message: `Node "${node.id}" is missing operator_id.`,
        locate: { kind: "node", nodeId: node.id },
      });
    }
  }
  for (const edge of graph.edges ?? []) {
    if (edgeIds.has(edge.id)) {
      issues.push({
        id: `graph:duplicate-edge:${edge.id}`,
        scope: "graph",
        severity: "warning",
        message: `Duplicate edge id "${edge.id}" detected.`,
        locate: { kind: "edge", edgeId: edge.id },
      });
    }
    edgeIds.add(edge.id);
    const fromNode = nodeMap.get(edge.from.node);
    const toNode = nodeMap.get(edge.to.node);
    if (!fromNode || !toNode) {
      issues.push({
        id: `graph:missing-edge-node:${edge.id}`,
        scope: "graph",
        severity: "warning",
        message: `Edge "${edge.id}" points to a missing node endpoint.`,
        detail: `${edge.from.node}.${edge.from.port} -> ${edge.to.node}.${edge.to.port}`,
        locate: { kind: "edge", edgeId: edge.id },
      });
      continue;
    }
    if (!portExists(fromNode, edge.from.port, "outputs") || !portExists(toNode, edge.to.port, "inputs")) {
      issues.push({
        id: `graph:missing-edge-port:${edge.id}`,
        scope: "graph",
        severity: "warning",
        message: `Edge "${edge.id}" points to a missing port endpoint.`,
        detail: `${edge.from.node}.${edge.from.port} -> ${edge.to.node}.${edge.to.port}`,
        locate: { kind: "edge", edgeId: edge.id },
      });
    }
  }
  if (!graph.schema_version.trim()) {
    issues.push({
      id: "graph:missing-schema-version",
      scope: "graph",
      severity: "warning",
      message: "Workflow graph schema_version is empty.",
      locate: { kind: "local" },
    });
  }
  return issues;
}

function validateDatasetIntegrity(graph: WorkflowGraphDefinition) {
  const issues: WorkflowIntegrityIssue[] = [];
  const contract = graph.dataset_contract;
  if (!contract) {
    issues.push({
      id: "dataset:missing-contract",
      scope: "dataset",
      severity: "warning",
      message: "Workflow graph is missing a dataset contract.",
      locate: { kind: "dataset" },
    });
    return issues;
  }
  const datasetValueMap = buildDatasetValueMap(contract);
  const duplicateIds = new Set<string>();
  for (const value of contract.values) {
    if (duplicateIds.has(value.id)) {
      issues.push({
        id: `dataset:duplicate:${value.id}`,
        scope: "dataset",
        severity: "warning",
        message: `Dataset value "${value.id}" is duplicated.`,
        locate: { kind: "dataset", datasetValueId: value.id },
      });
    }
    duplicateIds.add(value.id);
  }
  const referencedValues = new Set<string>();
  for (const node of graph.nodes) {
    for (const valueId of collectPortDatasetValues(node)) referencedValues.add(valueId);
  }
  for (const edge of graph.edges ?? []) {
    if (edge.dataset_value?.trim()) referencedValues.add(edge.dataset_value.trim());
  }
  for (const valueId of referencedValues) {
    if (!datasetValueMap.has(valueId)) {
      issues.push({
        id: `dataset:missing-value:${valueId}`,
        scope: "dataset",
        severity: "warning",
        message: `Referenced dataset value "${valueId}" is not declared in the dataset contract.`,
        locate: { kind: "dataset", datasetValueId: valueId },
      });
    }
  }
  for (const value of contract.values) {
    if (!referencedValues.has(value.id)) {
      issues.push({
        id: `dataset:orphan:${value.id}`,
        scope: "dataset",
        severity: "warning",
        message: `Dataset value "${value.id}" is declared but unused by graph ports or edges.`,
        locate: { kind: "dataset", datasetValueId: value.id },
      });
    }
  }
  return issues;
}

function validateOperatorIntegrity(graph: WorkflowGraphDefinition, operatorDescriptors: WorkflowOperatorDescriptor[]) {
  const issues: WorkflowIntegrityIssue[] = [];
  const descriptorIds = new Set(operatorDescriptors.map((descriptor) => descriptor.id));
  for (const node of graph.nodes) {
    if (!node.operator_id) continue;
    if (!descriptorIds.has(node.operator_id)) {
      issues.push({
        id: `operator:missing-descriptor:${node.id}`,
        scope: "operator",
        severity: "warning",
        message: `Node "${node.id}" references operator "${node.operator_id}" that is not present in the operator catalog.`,
        locate: { kind: "node", nodeId: node.id },
      });
    }
  }
  return issues;
}

function validateLocalLifecycle(
  workflow: WorkflowCatalogEntry,
): { issues: WorkflowIntegrityIssue[]; localWorkflowFound: boolean } {
  if (!workflow.local?.storage_id) return { issues: [] as WorkflowIntegrityIssue[], localWorkflowFound: false };
  const localWorkflow = findStoredLocalWorkflow(workflow.local.storage_id);
  if (!localWorkflow) {
    const issues: WorkflowIntegrityIssue[] = [
      {
        id: `local:missing:${workflow.local.storage_id}`,
        scope: "local",
        severity: "warning",
        message: `Local workflow storage entry "${workflow.local.storage_id}" is missing.`,
        locate: { kind: "local" },
      },
    ];
    return {
      localWorkflowFound: false,
      issues,
    };
  }
  return { issues: [] as WorkflowIntegrityIssue[], localWorkflowFound: true };
}

function validateSnapshots(
  workflowId: string,
): { issues: WorkflowIntegrityIssue[]; snapshotCount: number; summaryOnlySnapshotCount: number } {
  const snapshots = listStoredWorkflowSnapshots(workflowId);
  const summaryOnlyCount = snapshots.filter((snapshot) => snapshot.payloadState === "summary_only").length;
  const issues: WorkflowIntegrityIssue[] = [];
  if (summaryOnlyCount > 0) {
    issues.push({
      id: `snapshot:summary-only:${workflowId}`,
      scope: "snapshot",
      severity: "warning",
      message: `${summaryOnlyCount} snapshot(s) were stored without full payloads due to size limits.`,
      locate: { kind: "snapshot" },
    });
  }
  return { issues, snapshotCount: snapshots.length, summaryOnlySnapshotCount: summaryOnlyCount };
}

export function buildWorkflowIntegrityReport(
  workflow?: WorkflowCatalogEntry | null,
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
): WorkflowIntegrityReport {
  if (!workflow?.graph) {
    const issues: WorkflowIntegrityIssue[] = [
      {
        id: "graph:missing",
        scope: "graph",
        severity: "warning",
        message: "Workflow entry does not include a graph payload.",
        locate: { kind: "local" },
      },
    ];
    return {
      issues,
      snapshotCount: 0,
      summaryOnlySnapshotCount: 0,
      localWorkflowFound: false,
    };
  }
  const graph = workflow.graph;
  const localLifecycle = validateLocalLifecycle(workflow);
  const snapshotState = validateSnapshots(workflow.id);
  return {
    issues: [
      ...validateGraphStructure(graph),
      ...validateDatasetIntegrity(graph),
      ...validateOperatorIntegrity(graph, operatorDescriptors),
      ...localLifecycle.issues,
      ...snapshotState.issues,
    ],
    snapshotCount: snapshotState.snapshotCount,
    summaryOnlySnapshotCount: snapshotState.summaryOnlySnapshotCount,
    localWorkflowFound: localLifecycle.localWorkflowFound,
  };
}
