"use client";

import type {
  WorkflowGraphDefinition,
  WorkflowGraphEdge,
  WorkflowGraphNode,
  WorkflowGraphPort,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import {
  buildPortsForWorkflowNodeTemplate,
  type WorkflowNodeTemplateSelection,
} from "@/components/workbench/workflow/workbench-workflow-node-templates";

type WorkflowTemplateReconnectSuggestion = {
  edgeId: string;
  direction: "incoming" | "outgoing";
  suggestedPortId?: string;
};

export type WorkflowNodeTemplateSyncImpact = {
  nodeId: string;
  nextKind: string;
  nextOperatorId?: string;
  addedInputs: string[];
  removedInputs: string[];
  addedOutputs: string[];
  removedOutputs: string[];
  droppedIncomingEdges: string[];
  droppedOutgoingEdges: string[];
  reconnectSuggestions: WorkflowTemplateReconnectSuggestion[];
};

function suggestPort(edgeArtifactType: string, edgeDatasetValue: string | undefined, ports: WorkflowGraphPort[]) {
  if (edgeDatasetValue) {
    const datasetMatch = ports.find((port) => port.dataset_value === edgeDatasetValue);
    if (datasetMatch) return datasetMatch.id;
  }
  const artifactMatch = ports.find((port) => port.artifact_type === edgeArtifactType);
  if (artifactMatch) return artifactMatch.id;
  return ports[0]?.id;
}

function findNode(nodes: WorkflowGraphNode[], nodeId: string) {
  return nodes.find((entry) => entry.id === nodeId);
}

function findPort(node: WorkflowGraphNode | undefined, direction: "inputs" | "outputs", portId: string) {
  return node?.[direction]?.find((port) => port.id === portId);
}

function syncEdgeFromPorts(edge: WorkflowGraphEdge, nodes: WorkflowGraphNode[]) {
  const sourceNode = findNode(nodes, edge.from.node);
  const sourcePort = findPort(sourceNode, "outputs", edge.from.port);
  const targetNode = findNode(nodes, edge.to.node);
  const targetPort = findPort(targetNode, "inputs", edge.to.port);

  return {
    ...edge,
    artifact_type: sourcePort?.artifact_type ?? targetPort?.artifact_type ?? edge.artifact_type,
    dataset_value: sourcePort?.dataset_value ?? targetPort?.dataset_value ?? edge.dataset_value,
  };
}

export function getWorkflowNodeTemplateSyncImpact(
  graph: Pick<WorkflowGraphDefinition, "nodes" | "edges">,
  nodeId: string,
  template: WorkflowNodeTemplateSelection | undefined,
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
): WorkflowNodeTemplateSyncImpact | null {
  const node = graph.nodes.find((entry) => entry.id === nodeId);
  if (!node) return null;
  const resolved = buildPortsForWorkflowNodeTemplate(template, operatorDescriptors);
  const currentInputs = new Set((node.inputs ?? []).map((port) => port.id));
  const currentOutputs = new Set((node.outputs ?? []).map((port) => port.id));
  const nextInputs = new Set(resolved.inputs.map((port) => port.id));
  const nextOutputs = new Set(resolved.outputs.map((port) => port.id));

  const reconnectSuggestions: WorkflowTemplateReconnectSuggestion[] = [];
  for (const edge of graph.edges ?? []) {
    if (edge.to.node === nodeId && !nextInputs.has(edge.to.port)) {
      reconnectSuggestions.push({
        edgeId: edge.id,
        direction: "incoming",
        suggestedPortId: suggestPort(edge.artifact_type, edge.dataset_value, resolved.inputs),
      });
    } else if (edge.from.node === nodeId && !nextOutputs.has(edge.from.port)) {
      reconnectSuggestions.push({
        edgeId: edge.id,
        direction: "outgoing",
        suggestedPortId: suggestPort(edge.artifact_type, edge.dataset_value, resolved.outputs),
      });
    }
  }

  return {
    nodeId,
    nextKind: resolved.kind,
    nextOperatorId: resolved.operatorId,
    addedInputs: resolved.inputs.map((port) => port.id).filter((portId) => !currentInputs.has(portId)),
    removedInputs: [...currentInputs].filter((portId) => !nextInputs.has(portId)),
    addedOutputs: resolved.outputs.map((port) => port.id).filter((portId) => !currentOutputs.has(portId)),
    removedOutputs: [...currentOutputs].filter((portId) => !nextOutputs.has(portId)),
    droppedIncomingEdges: (graph.edges ?? [])
      .filter((edge) => edge.to.node === nodeId && !nextInputs.has(edge.to.port))
      .map((edge) => edge.id),
    droppedOutgoingEdges: (graph.edges ?? [])
      .filter((edge) => edge.from.node === nodeId && !nextOutputs.has(edge.from.port))
      .map((edge) => edge.id),
    reconnectSuggestions,
  };
}

export function describeWorkflowNodeTemplateSyncImpact(
  impact: WorkflowNodeTemplateSyncImpact | null,
): string | null {
  if (!impact) return null;
  const lines = [
    `Sync node "${impact.nodeId}" to ${impact.nextOperatorId ? `operator "${impact.nextOperatorId}"` : `kind "${impact.nextKind}"`} template?`,
  ];
  if (impact.addedInputs.length > 0) lines.push(`Add input ports: ${impact.addedInputs.join(", ")}`);
  if (impact.removedInputs.length > 0) lines.push(`Remove input ports: ${impact.removedInputs.join(", ")}`);
  if (impact.addedOutputs.length > 0) lines.push(`Add output ports: ${impact.addedOutputs.join(", ")}`);
  if (impact.removedOutputs.length > 0) lines.push(`Remove output ports: ${impact.removedOutputs.join(", ")}`);
  if (impact.droppedIncomingEdges.length > 0) lines.push(`Disconnect incoming edges: ${impact.droppedIncomingEdges.join(", ")}`);
  if (impact.droppedOutgoingEdges.length > 0) lines.push(`Disconnect outgoing edges: ${impact.droppedOutgoingEdges.join(", ")}`);
  if (impact.reconnectSuggestions.length > 0) {
    lines.push("Suggested reconnects:");
    for (const suggestion of impact.reconnectSuggestions) {
      lines.push(
        `- ${suggestion.edgeId} (${suggestion.direction}) -> ${suggestion.suggestedPortId ?? "no matching port"}`,
      );
    }
  }
  return lines.length > 1 ? lines.join("\n") : null;
}

export function listAutoReconnectEdgeIds(impact: WorkflowNodeTemplateSyncImpact | null): string[] {
  if (!impact) return [];
  return impact.reconnectSuggestions
    .filter((suggestion) => suggestion.suggestedPortId)
    .map((suggestion) => suggestion.edgeId);
}

export function applyWorkflowNodeTemplateSync(
  graph: WorkflowGraphDefinition,
  nodeId: string,
  template: WorkflowNodeTemplateSelection | undefined,
  operatorDescriptors: WorkflowOperatorDescriptor[] = [],
) {
  const node = graph.nodes.find((entry) => entry.id === nodeId);
  if (!node) return false;
  const impact = getWorkflowNodeTemplateSyncImpact(graph, nodeId, template, operatorDescriptors);
  const resolved = buildPortsForWorkflowNodeTemplate(template, operatorDescriptors);

  node.kind = resolved.kind;
  node.operator_id = resolved.operatorId;
  node.config = resolved.config;
  node.inputs = resolved.inputs;
  node.outputs = resolved.outputs;

  const reconnectMap = new Map(
    (impact?.reconnectSuggestions ?? [])
      .filter((suggestion) => suggestion.suggestedPortId)
      .map((suggestion) => [suggestion.edgeId, suggestion] as const),
  );

  graph.edges = (graph.edges ?? [])
    .map((edge) => {
      if (edge.from.node === nodeId && !(node.outputs ?? []).some((port) => port.id === edge.from.port)) {
        const suggestion = reconnectMap.get(edge.id);
        if (suggestion?.direction === "outgoing" && suggestion.suggestedPortId) {
          return syncEdgeFromPorts({ ...edge, from: { ...edge.from, port: suggestion.suggestedPortId } }, graph.nodes);
        }
        return null;
      }
      if (edge.to.node === nodeId && !(node.inputs ?? []).some((port) => port.id === edge.to.port)) {
        const suggestion = reconnectMap.get(edge.id);
        if (suggestion?.direction === "incoming" && suggestion.suggestedPortId) {
          return syncEdgeFromPorts({ ...edge, to: { ...edge.to, port: suggestion.suggestedPortId } }, graph.nodes);
        }
        return null;
      }
      return syncEdgeFromPorts(edge, graph.nodes);
    })
    .filter((edge): edge is NonNullable<typeof edge> => Boolean(edge));

  return true;
}
