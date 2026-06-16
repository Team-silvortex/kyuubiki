"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";
import {
  buildNodeMap,
  findPort,
  hasDatasetValue,
} from "@/components/workbench/workflow/workbench-workflow-validation-graph";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-validation-types";

export function validateEdgeAndDatasetReferences(
  graph: WorkflowGraphDefinition,
): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];
  const nodeMap = buildNodeMap(graph);
  const contract = graph.dataset_contract;

  for (const node of graph.nodes) {
    for (const port of [...(node.inputs ?? []), ...(node.outputs ?? [])]) {
      if (!hasDatasetValue(contract, port.dataset_value)) {
        issues.push({
          id: `port-dataset:${node.id}:${port.id}:${port.dataset_value}`,
          level: "warning",
          message: `Port "${node.id}.${port.id}" references missing dataset value "${port.dataset_value}".`,
          locate: { kind: "dataset", datasetValueId: port.dataset_value },
          fix: {
            kind: "clear_port_dataset_value",
            nodeId: node.id,
            portId: port.id,
            direction: (node.inputs ?? []).some((input) => input.id === port.id)
              ? "inputs"
              : "outputs",
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
        locate: { kind: "edge", edgeId: edge.id },
      });
    }
    if (!toNode) {
      issues.push({
        id: `edge-to-node:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" references missing target node "${edge.to.node}".`,
        locate: { kind: "edge", edgeId: edge.id },
      });
    }
    if (fromNode && !fromPort) {
      issues.push({
        id: `edge-from-port:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" references missing source port "${edge.from.node}.${edge.from.port}".`,
        locate: { kind: "edge", edgeId: edge.id },
      });
    }
    if (toNode && !toPort) {
      issues.push({
        id: `edge-to-port:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" references missing target port "${edge.to.node}.${edge.to.port}".`,
        locate: { kind: "edge", edgeId: edge.id },
      });
    }
    if (fromPort && fromPort.artifact_type !== edge.artifact_type) {
      issues.push({
        id: `edge-artifact-from:${edge.id}`,
        level: "warning",
        message: `Edge "${edge.id}" artifact type "${edge.artifact_type}" does not match source port "${fromPort.artifact_type}".`,
        locate: { kind: "edge", edgeId: edge.id },
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
        locate: { kind: "edge", edgeId: edge.id },
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
        locate: { kind: "dataset", datasetValueId: edge.dataset_value },
        fix: { kind: "clear_edge_dataset_value", edgeId: edge.id },
      });
    }
  }

  return issues;
}
