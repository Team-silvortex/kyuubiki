"use client";

import type {
  WorkflowCatalogEntryArtifact,
  WorkflowDatasetContract,
  WorkflowGraphDefinition,
  WorkflowGraphNode,
  WorkflowGraphPort,
} from "@/lib/api";

export type WorkflowGraphValidationIssue = {
  id: string;
  level: "warning";
  message: string;
  fix?:
    | { kind: "set_edge_artifact_type_from_source"; edgeId: string; artifactType: string }
    | { kind: "set_edge_artifact_type_from_target"; edgeId: string; artifactType: string }
    | {
        kind: "set_catalog_artifact_type";
        mode: "entry" | "output";
        nodeId: string;
        currentArtifactType: string;
        artifactType: string;
      }
    | { kind: "clear_port_dataset_value"; nodeId: string; portId: string; direction: "inputs" | "outputs" }
    | { kind: "clear_edge_dataset_value"; edgeId: string };
};

function buildNodeMap(graph: WorkflowGraphDefinition) {
  return new Map(graph.nodes.map((node) => [node.id, node] as const));
}

function findPort(
  node: WorkflowGraphNode | undefined,
  portId: string,
  direction: "inputs" | "outputs",
): WorkflowGraphPort | undefined {
  return node?.[direction]?.find((port) => port.id === portId);
}

function hasDatasetValue(contract: WorkflowDatasetContract | undefined, valueId: string | undefined) {
  if (!contract || !valueId) return true;
  return contract.values.some((value) => value.id === valueId);
}

function validateCatalogArtifacts(
  graph: WorkflowGraphDefinition,
  artifacts: WorkflowCatalogEntryArtifact[],
  mode: "entry" | "output",
): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];
  const nodeMap = buildNodeMap(graph);
  for (const artifact of artifacts) {
    const node = nodeMap.get(artifact.node_id);
    if (!node) {
      issues.push({
        id: `${mode}:missing-node:${artifact.node_id}:${artifact.artifact_type}`,
        level: "warning",
        message: `${mode === "entry" ? "Entry input" : "Output artifact"} "${artifact.node_id}" is not present in the graph.`,
      });
      continue;
    }
    const ports = mode === "entry" ? node.inputs ?? [] : node.outputs ?? [];
    if (!ports.some((port) => port.artifact_type === artifact.artifact_type)) {
      issues.push({
        id: `${mode}:missing-artifact:${artifact.node_id}:${artifact.artifact_type}`,
        level: "warning",
        message: `${mode === "entry" ? "Entry input" : "Output artifact"} "${artifact.artifact_type}" is not exposed on node "${artifact.node_id}".`,
        fix: ports[0]
          ? {
              kind: "set_catalog_artifact_type",
              mode,
              nodeId: artifact.node_id,
              currentArtifactType: artifact.artifact_type,
              artifactType: ports[0].artifact_type,
            }
          : undefined,
      });
    }
  }
  return issues;
}

export function validateWorkflowGraphDefinition(
  graph: WorkflowGraphDefinition | null,
  entryInputs: WorkflowCatalogEntryArtifact[],
  outputArtifacts: WorkflowCatalogEntryArtifact[],
): WorkflowGraphValidationIssue[] {
  if (!graph) return [];
  const issues: WorkflowGraphValidationIssue[] = [];
  const nodeMap = buildNodeMap(graph);
  const contract = graph.dataset_contract;

  for (const nodeId of graph.entry_nodes ?? []) {
    if (!nodeMap.has(nodeId)) {
      issues.push({
        id: `entry-node:${nodeId}`,
        level: "warning",
        message: `Entry node "${nodeId}" is not present in the graph.`,
      });
    }
  }

  for (const nodeId of graph.output_nodes ?? []) {
    if (!nodeMap.has(nodeId)) {
      issues.push({
        id: `output-node:${nodeId}`,
        level: "warning",
        message: `Output node "${nodeId}" is not present in the graph.`,
      });
    }
  }

  for (const node of graph.nodes) {
    for (const port of [...(node.inputs ?? []), ...(node.outputs ?? [])]) {
      if (!hasDatasetValue(contract, port.dataset_value)) {
        issues.push({
          id: `port-dataset:${node.id}:${port.id}:${port.dataset_value}`,
          level: "warning",
          message: `Port "${node.id}.${port.id}" references missing dataset value "${port.dataset_value}".`,
          fix: {
            kind: "clear_port_dataset_value",
            nodeId: node.id,
            portId: port.id,
            direction: (node.inputs ?? []).some((input) => input.id === port.id) ? "inputs" : "outputs",
          },
        });
      }
    }
  }

  for (const edge of graph.edges ?? []) {
    const fromNode = nodeMap.get(edge.from.node);
    const toNode = nodeMap.get(edge.to.node);
    const fromPort = findPort(fromNode, edge.from.port, "outputs");
    const toPort = findPort(toNode, edge.to.port, "inputs");

    if (!fromNode) {
      issues.push({
        id: `edge-from-node:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" references missing source node "${edge.from.node}".`,
      });
    }
    if (!toNode) {
      issues.push({
        id: `edge-to-node:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" references missing target node "${edge.to.node}".`,
      });
    }
    if (fromNode && !fromPort) {
      issues.push({
        id: `edge-from-port:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" references missing source port "${edge.from.node}.${edge.from.port}".`,
      });
    }
    if (toNode && !toPort) {
      issues.push({
        id: `edge-to-port:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" references missing target port "${edge.to.node}.${edge.to.port}".`,
      });
    }
    if (fromPort && fromPort.artifact_type !== edge.artifact_type) {
      issues.push({
        id: `edge-artifact-from:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" artifact type "${edge.artifact_type}" does not match source port "${fromPort.artifact_type}".`,
        fix: {
          kind: "set_edge_artifact_type_from_source",
          edgeId: edge.id,
          artifactType: fromPort.artifact_type,
        },
      });
    }
    if (toPort && toPort.artifact_type !== edge.artifact_type) {
      issues.push({
        id: `edge-artifact-to:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" artifact type "${edge.artifact_type}" does not match target port "${toPort.artifact_type}".`,
        fix: {
          kind: "set_edge_artifact_type_from_target",
          edgeId: edge.id,
          artifactType: toPort.artifact_type,
        },
      });
    }
    if (!hasDatasetValue(contract, edge.dataset_value)) {
      issues.push({
        id: `edge-dataset:${edge.id}:${edge.dataset_value}`,
        level: "warning",
        message: `Edge "${edge.id}" references missing dataset value "${edge.dataset_value}".`,
        fix: { kind: "clear_edge_dataset_value", edgeId: edge.id },
      });
    }
  }

  issues.push(...validateCatalogArtifacts(graph, entryInputs, "entry"));
  issues.push(...validateCatalogArtifacts(graph, outputArtifacts, "output"));

  return issues;
}
