"use client";

import type {
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-validation-types";

export function validateOperatorDescriptorContracts(
  graph: WorkflowGraphDefinition,
  operatorDescriptors: WorkflowOperatorDescriptor[],
): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];
  if (operatorDescriptors.length === 0) return issues;
  const descriptorMap = new Map(
    operatorDescriptors.map((descriptor) => [descriptor.id, descriptor] as const),
  );

  for (const node of graph.nodes) {
    const operatorId = node.operator_id?.trim();
    if (!operatorId) continue;
    const descriptor = descriptorMap.get(operatorId);
    if (!descriptor) {
      issues.push({
        id: `operator:missing:${node.id}:${operatorId}`,
        level: "warning",
        message: `Node "${node.id}" references unknown operator "${operatorId}".`,
        locate: { kind: "node", nodeId: node.id },
      });
      continue;
    }

    for (const direction of ["inputs", "outputs"] as const) {
      const descriptorPorts = descriptor[direction];
      const nodePorts = node[direction] ?? [];

      for (const descriptorPort of descriptorPorts) {
        const nodePort = nodePorts.find((port) => port.id === descriptorPort.id);
        if (!nodePort) {
          issues.push({
            id: `operator:missing-port:${node.id}:${direction}:${descriptorPort.id}`,
            level: "warning",
            message: `Node "${node.id}" is missing ${direction === "inputs" ? "input" : "output"} port "${descriptorPort.id}" required by operator "${operatorId}".`,
            locate: { kind: "node", nodeId: node.id },
            fix: {
              kind: "sync_node_template_from_operator",
              nodeId: node.id,
              operatorId,
              templateKind: node.kind,
            },
          });
          continue;
        }
        if (nodePort.artifact_type !== descriptorPort.artifact_type) {
          issues.push({
            id: `operator:artifact:${node.id}:${direction}:${descriptorPort.id}`,
            level: "warning",
            message: `Node "${node.id}" port "${descriptorPort.id}" uses artifact type "${nodePort.artifact_type}" but operator "${operatorId}" requires "${descriptorPort.artifact_type}".`,
            locate: { kind: "node", nodeId: node.id },
            fix: {
              kind: "set_node_port_artifact_type_from_operator",
              nodeId: node.id,
              portId: descriptorPort.id,
              direction,
              artifactType: descriptorPort.artifact_type,
            },
          });
        }
        if ((nodePort.dataset_value ?? "") !== (descriptorPort.dataset_value ?? "")) {
          issues.push({
            id: `operator:dataset:${node.id}:${direction}:${descriptorPort.id}`,
            level: "warning",
            message: descriptorPort.dataset_value
              ? `Node "${node.id}" port "${descriptorPort.id}" should bind dataset value "${descriptorPort.dataset_value}" from operator "${operatorId}".`
              : `Node "${node.id}" port "${descriptorPort.id}" should not bind a dataset value for operator "${operatorId}".`,
            locate: { kind: "node", nodeId: node.id },
            fix: {
              kind: "set_node_port_dataset_value_from_operator",
              nodeId: node.id,
              portId: descriptorPort.id,
              direction,
              datasetValue: descriptorPort.dataset_value ?? undefined,
            },
          });
        }
      }

      for (const nodePort of nodePorts) {
        if (!descriptorPorts.some((port) => port.id === nodePort.id)) {
          issues.push({
            id: `operator:extra-port:${node.id}:${direction}:${nodePort.id}`,
            level: "warning",
            message: `Node "${node.id}" exposes ${direction === "inputs" ? "input" : "output"} port "${nodePort.id}" that is not declared by operator "${operatorId}".`,
            locate: { kind: "node", nodeId: node.id },
            fix: {
              kind: "sync_node_template_from_operator",
              nodeId: node.id,
              operatorId,
              templateKind: node.kind,
            },
          });
        }
      }
    }
  }

  return issues;
}
