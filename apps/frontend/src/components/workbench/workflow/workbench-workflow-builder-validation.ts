"use client";

import type {
  WorkflowCatalogEntryArtifact,
  WorkflowDatasetContract,
  WorkflowGraphDefinition,
  WorkflowGraphNode,
  WorkflowGraphPort,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import {
  conditionOperatorNeedsValue,
  isWorkflowConditionNode,
  resolveWorkflowConditionConfig,
  WORKFLOW_CONDITION_OPERATORS,
} from "@/components/workbench/workflow/workbench-workflow-condition";
import { hasBridgeSeedModelConfig } from "@/components/workbench/workflow/workbench-workflow-bridge-contract";
import { buildPortsForWorkflowNodeTemplate } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import { isWorkflowNodeSupportedInRuntime } from "@/components/workbench/workflow/workbench-workflow-runtime-support";
import { applyWorkflowNodeTemplateSync } from "@/components/workbench/workflow/workbench-workflow-template-impact";

export type WorkflowGraphValidationIssue = {
  id: string;
  level: "warning";
  message: string;
  locate?:
    | { kind: "node"; nodeId: string }
    | { kind: "edge"; edgeId: string }
    | { kind: "dataset"; datasetValueId?: string }
    | { kind: "artifact"; mode: "entry" | "output"; nodeId: string; artifactType: string };
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
    | {
        kind: "set_node_port_artifact_type_from_operator";
        nodeId: string;
        portId: string;
        direction: "inputs" | "outputs";
        artifactType: string;
      }
    | {
        kind: "set_node_port_dataset_value_from_operator";
        nodeId: string;
        portId: string;
        direction: "inputs" | "outputs";
        datasetValue?: string;
      }
    | {
        kind: "sync_node_template_from_operator";
        nodeId: string;
        operatorId: string;
        templateKind?: string;
      }
    | { kind: "clear_port_dataset_value"; nodeId: string; portId: string; direction: "inputs" | "outputs" }
    | { kind: "clear_edge_dataset_value"; edgeId: string };
};

export type WorkflowValidationFixBatchResult = {
  graph: WorkflowGraphDefinition | null;
  appliedCount: number;
  appliedIssues: WorkflowGraphValidationIssue[];
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

function validateOperatorDescriptorContracts(
  graph: WorkflowGraphDefinition,
  operatorDescriptors: WorkflowOperatorDescriptor[],
): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];
  if (operatorDescriptors.length === 0) return issues;
  const descriptorMap = new Map(operatorDescriptors.map((descriptor) => [descriptor.id, descriptor] as const));

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
        locate: {
          kind: "artifact",
          mode,
          nodeId: artifact.node_id,
          artifactType: artifact.artifact_type,
        },
      });
      continue;
    }
    const ports = mode === "entry" ? node.inputs ?? [] : node.outputs ?? [];
    if (!ports.some((port) => port.artifact_type === artifact.artifact_type)) {
      issues.push({
        id: `${mode}:missing-artifact:${artifact.node_id}:${artifact.artifact_type}`,
        level: "warning",
        message: `${mode === "entry" ? "Entry input" : "Output artifact"} "${artifact.artifact_type}" is not exposed on node "${artifact.node_id}".`,
        locate: {
          kind: "artifact",
          mode,
          nodeId: artifact.node_id,
          artifactType: artifact.artifact_type,
        },
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

function validateConditionNodes(graph: WorkflowGraphDefinition): WorkflowGraphValidationIssue[] {
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

function validateRuntimeSupport(graph: WorkflowGraphDefinition): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];
  for (const node of graph.nodes) {
    if (isWorkflowNodeSupportedInRuntime(node)) continue;
    issues.push({
      id: `runtime:unsupported:${node.id}`,
      level: "warning",
      message: node.operator_id
        ? `Node "${node.id}" uses operator "${node.operator_id}" which is not supported by the current workflow executor.`
        : `Node "${node.id}" uses node kind "${node.kind}" which is not supported by the current workflow executor.`,
      locate: { kind: "node", nodeId: node.id },
    });
  }
  return issues;
}

function validateBridgeConfigs(graph: WorkflowGraphDefinition): WorkflowGraphValidationIssue[] {
  const issues: WorkflowGraphValidationIssue[] = [];
  for (const node of graph.nodes) {
    if (!node.operator_id?.startsWith("bridge.")) continue;
    if (
      hasBridgeSeedModelConfig(
        node.operator_id,
        node.config as Record<string, unknown> | null | undefined,
      )
    ) {
      continue;
    }
    issues.push({
      id: `bridge:seed-model:${node.id}`,
      level: "warning",
      message:
        node.operator_id === "bridge.electrostatic_field_to_heat_quad_2d"
          ? `Bridge node "${node.id}" is missing config.seed_model for the downstream heat quad seed model.`
          : `Bridge node "${node.id}" is missing downstream thermo seed-model fields in config.`,
      locate: { kind: "node", nodeId: node.id },
    });
  }
  return issues;
}

export function validateWorkflowGraphDefinition(
  graph: WorkflowGraphDefinition | null,
  entryInputs: WorkflowCatalogEntryArtifact[],
  outputArtifacts: WorkflowCatalogEntryArtifact[],
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
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

  issues.push(...validateRuntimeSupport(graph));
  issues.push(...validateBridgeConfigs(graph));
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
      applyWorkflowNodeTemplateSync(
        next,
        fix.nodeId,
        { kind: fix.templateKind, operatorId: fix.operatorId },
        operatorDescriptors,
      );
      break;
    }
    case "set_edge_artifact_type_from_source":
    case "set_edge_artifact_type_from_target": {
      const edge = next.edges?.find((entry) => entry.id === fix.edgeId);
      if (edge) edge.artifact_type = fix.artifactType;
      break;
    }
    case "set_catalog_artifact_type": {
      const artifacts = next[fix.mode === "entry" ? "entry_inputs" : "output_artifacts"] ?? [];
      const artifact = artifacts.find(
        (entry) => entry.node_id === fix.nodeId && entry.artifact_type === fix.currentArtifactType,
      );
      if (artifact) artifact.artifact_type = fix.artifactType;
      break;
    }
    case "set_node_port_artifact_type_from_operator":
    case "set_node_port_dataset_value_from_operator":
    case "clear_port_dataset_value": {
      const node = next.nodes.find((entry) => entry.id === fix.nodeId);
      const port = node?.[fix.direction]?.find((entry) => entry.id === fix.portId);
      if (!port) break;
      if (fix.kind === "set_node_port_artifact_type_from_operator") port.artifact_type = fix.artifactType;
      else if (fix.kind === "set_node_port_dataset_value_from_operator") port.dataset_value = fix.datasetValue;
      else port.dataset_value = undefined;
      break;
    }
    case "clear_edge_dataset_value": {
      const edge = next.edges?.find((entry) => entry.id === fix.edgeId);
      if (edge) edge.dataset_value = undefined;
      break;
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
