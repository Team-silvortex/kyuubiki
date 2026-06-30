"use client";

import type {
  WorkflowCatalogEntryArtifact,
  WorkflowDatasetContract,
  WorkflowDatasetValueInfo,
  WorkflowGraphDefinition,
  WorkflowGraphEdge,
  WorkflowGraphNode,
  WorkflowOperatorDescriptor,
  WorkflowGraphPort,
} from "@/lib/api";
import {
  buildPortsForWorkflowNodeTemplate,
  type WorkflowNodeTemplateSelection,
} from "@/components/workbench/workflow/workbench-workflow-node-templates";

export function cloneWorkflowGraph(graph: WorkflowGraphDefinition | null): WorkflowGraphDefinition | null {
  if (!graph) return null;
  return JSON.parse(JSON.stringify(graph)) as WorkflowGraphDefinition;
}

export function buildDraftArtifact(nextIndex: number): WorkflowCatalogEntryArtifact {
  return {
    node_id: `node_${nextIndex}`,
    artifact_type: "artifact/json",
    description: "",
  };
}

export function ensureDatasetContract(
  graph: WorkflowGraphDefinition | null,
): WorkflowDatasetContract | null {
  if (!graph) return null;
  if (!graph.dataset_contract) {
    graph.dataset_contract = {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: `${graph.id}.dataset`,
      version: graph.version ?? "1.13.0",
      name: `${graph.name ?? graph.id} dataset contract`,
      values: [],
      metadata: {},
    };
  }
  return graph.dataset_contract;
}

export function buildDraftDatasetValue(nextIndex: number): WorkflowDatasetValueInfo {
  return {
    id: `value_${nextIndex}`,
    data_class: "field",
    element_type: "scalar",
    shape: { axes: [] },
    semantic_type: "result/derived",
    encoding: "json",
    unit: "",
  };
}

export function buildDraftNode(
  nextIndex: number,
  template?: WorkflowNodeTemplateSelection,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
): WorkflowGraphNode {
  const resolved = buildPortsForWorkflowNodeTemplate(template, operatorDescriptors);
  return {
    id: `node_${nextIndex}`,
    kind: resolved.kind,
    operator_id: resolved.operatorId,
    config: resolved.config,
    inputs: resolved.inputs,
    outputs: resolved.outputs,
  };
}

export function buildDraftEdge(nextIndex: number, nodes: WorkflowGraphNode[]): WorkflowGraphEdge {
  const fromNode = nodes[0];
  const toNode = nodes[1] ?? nodes[0];
  return {
    id: `edge_${nextIndex}`,
    from: {
      node: fromNode?.id ?? "",
      port: fromNode?.outputs?.[0]?.id ?? "",
    },
    to: {
      node: toNode?.id ?? "",
      port: toNode?.inputs?.[0]?.id ?? "",
    },
    artifact_type: fromNode?.outputs?.[0]?.artifact_type ?? "artifact/json",
  };
}

export function buildDraftPort(direction: "in" | "out", nextIndex: number): WorkflowGraphPort {
  return {
    id: `${direction}_${nextIndex}`,
    artifact_type: "artifact/json",
    description: "",
  };
}

export function slugifyWorkflowAssetName(value: string): string {
  return (
    value
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "workflow"
  );
}

export function downloadJsonArtifact(filename: string, payload: unknown) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], {
    type: "application/json",
  });
  downloadBlobArtifact(filename, blob);
}

export function downloadHtmlArtifact(filename: string, html: string) {
  const blob = new Blob([html], {
    type: "text/html;charset=utf-8",
  });
  downloadBlobArtifact(filename, blob);
}

function downloadBlobArtifact(filename: string, blob: Blob) {
  const href = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = href;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(href);
}
