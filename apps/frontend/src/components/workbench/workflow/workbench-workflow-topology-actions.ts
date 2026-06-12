"use client";

import type { Dispatch, SetStateAction } from "react";
import type {
  WorkflowDatasetValueInfo,
  WorkflowGraphDefinition,
  WorkflowGraphEdge,
  WorkflowGraphNode,
  WorkflowOperatorDescriptor,
  WorkflowGraphPort,
} from "@/lib/api";
import {
  buildDraftEdge,
  buildDraftNode,
  buildDraftPort,
  cloneWorkflowGraph,
} from "@/components/workbench/workflow/workbench-workflow-builder-utils";
import {
  listWorkflowTemplateDatasetValues,
  type WorkflowNodeTemplateSelection,
} from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";
import { applyWorkflowNodeTemplateSync } from "@/components/workbench/workflow/workbench-workflow-template-impact";

type SetDraftGraph = Dispatch<SetStateAction<WorkflowGraphDefinition | null>>;

function findNode(nodes: WorkflowGraphNode[], nodeId: string) {
  return nodes.find((node) => node.id === nodeId);
}

function findPort(
  node: WorkflowGraphNode | undefined,
  direction: "inputs" | "outputs",
  portId: string,
) {
  return node?.[direction]?.find((port) => port.id === portId);
}

function syncEdgeFromPorts(edge: WorkflowGraphEdge, nodes: WorkflowGraphNode[]) {
  const sourceNode = findNode(nodes, edge.from.node);
  const sourcePort = findPort(sourceNode, "outputs", edge.from.port);
  const targetNode = findNode(nodes, edge.to.node);
  const targetPort = findPort(targetNode, "inputs", edge.to.port);

  return {
    ...edge,
    artifact_type:
      sourcePort?.artifact_type ??
      targetPort?.artifact_type ??
      edge.artifact_type,
    dataset_value:
      sourcePort?.dataset_value ??
      targetPort?.dataset_value ??
      edge.dataset_value,
  };
}

function pickConnectedPorts(sourceNode: WorkflowGraphNode, nextNode: WorkflowGraphNode) {
  const sourceOutputs = sourceNode.outputs ?? [];
  const targetInputs = nextNode.inputs ?? [];

  for (const sourcePort of sourceOutputs) {
    const datasetMatch = targetInputs.find(
      (port) =>
        sourcePort.dataset_value &&
        port.dataset_value &&
        port.dataset_value === sourcePort.dataset_value,
    );
    if (datasetMatch) {
      return { sourcePort, targetPort: datasetMatch };
    }
  }

  for (const sourcePort of sourceOutputs) {
    const artifactMatch = targetInputs.find(
      (port) => port.artifact_type === sourcePort.artifact_type,
    );
    if (artifactMatch) {
      return { sourcePort, targetPort: artifactMatch };
    }
  }

  return {
    sourcePort: sourceOutputs[0],
    targetPort: targetInputs[0],
  };
}

function mergeMissingDatasetValues(
  currentValues: WorkflowDatasetValueInfo[] | undefined,
  nextValues: WorkflowDatasetValueInfo[],
) {
  const existing = currentValues ?? [];
  const existingIds = new Set(existing.map((value) => value.id));
  const merged = [...existing];
  for (const value of nextValues) {
    if (!existingIds.has(value.id)) {
      merged.push(value);
      existingIds.add(value.id);
    }
  }
  return merged;
}

function ensureTemplateDatasetValues(
  graph: WorkflowGraphDefinition,
  template?: WorkflowNodeTemplateSelection,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  const datasetValues = listWorkflowTemplateDatasetValues(template, operatorDescriptors);
  if (datasetValues.length === 0) return;

  const currentContract = graph.dataset_contract;
  if (currentContract) {
    currentContract.values = mergeMissingDatasetValues(currentContract.values, datasetValues);
  } else {
    graph.dataset_contract = {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: `${graph.id}.dataset`,
      version: graph.version ?? "1.6.0",
      name: `${graph.name ?? graph.id} dataset contract`,
      values: datasetValues,
      metadata: {},
    };
  }
}

function appendConnectedNode(
  graph: WorkflowGraphDefinition,
  sourceNode: WorkflowGraphNode | null,
  template?: WorkflowNodeTemplateSelection,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  const createdNode = buildDraftNode(graph.nodes.length + 1, template, operatorDescriptors);
  graph.nodes = [...graph.nodes, createdNode];
  ensureTemplateDatasetValues(graph, template, operatorDescriptors);

  const previousNode = sourceNode;
  if (!previousNode) return createdNode;

  const ports = pickConnectedPorts(previousNode, createdNode);
  const baseEdge = buildDraftEdge((graph.edges ?? []).length + 1, [previousNode, createdNode]);
  const connectedEdge = syncEdgeFromPorts(
    {
      ...baseEdge,
      from: {
        node: previousNode.id,
        port: ports.sourcePort?.id ?? baseEdge.from.port,
      },
      to: {
        node: createdNode.id,
        port: ports.targetPort?.id ?? baseEdge.to.port,
      },
    },
    graph.nodes,
  );

  graph.edges = [...(graph.edges ?? []), connectedEdge];
  return createdNode;
}

function connectNodes(
  graph: WorkflowGraphDefinition,
  sourceNode: WorkflowGraphNode,
  targetNode: WorkflowGraphNode,
  sourcePortId?: string,
  targetPortId?: string,
) {
  const ports =
    sourcePortId && targetPortId
      ? { sourcePort: findPort(sourceNode, "outputs", sourcePortId), targetPort: findPort(targetNode, "inputs", targetPortId) }
      : pickConnectedPorts(sourceNode, targetNode);
  const baseEdge = buildDraftEdge((graph.edges ?? []).length + 1, [sourceNode, targetNode]);
  const connectedEdge = syncEdgeFromPorts(
    {
      ...baseEdge,
      from: { node: sourceNode.id, port: sourcePortId ?? ports.sourcePort?.id ?? baseEdge.from.port },
      to: { node: targetNode.id, port: targetPortId ?? ports.targetPort?.id ?? baseEdge.to.port },
    },
    graph.nodes,
  );
  graph.edges = [...(graph.edges ?? []), connectedEdge];
}

function connectNodesByPorts(
  graph: WorkflowGraphDefinition,
  sourceNodeId: string,
  sourcePortId: string,
  targetNodeId: string,
  targetPortId: string,
) {
  const sourceNode = findNode(graph.nodes, sourceNodeId);
  const targetNode = findNode(graph.nodes, targetNodeId);
  if (!sourceNode || !targetNode) return;
  connectNodes(graph, sourceNode, targetNode, sourcePortId, targetPortId);
}

function appendTemplateChainNodes(
  graph: WorkflowGraphDefinition,
  templates: WorkflowNodeTemplateSelection[],
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  const createdNodes: WorkflowGraphNode[] = [];
  for (const template of templates) {
    const createdNode = buildDraftNode(graph.nodes.length + 1, template, operatorDescriptors);
    graph.nodes = [...graph.nodes, createdNode];
    ensureTemplateDatasetValues(graph, template, operatorDescriptors);
    createdNodes.push(createdNode);
  }
  return createdNodes;
}

function appendLinearTemplateChain(
  graph: WorkflowGraphDefinition,
  sourceNode: WorkflowGraphNode | null,
  templates: WorkflowNodeTemplateSelection[],
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  let previousNode = sourceNode;
  for (const template of templates) {
    previousNode = appendConnectedNode(graph, previousNode, template, operatorDescriptors);
  }
}

function appendGraphTemplateChain(
  graph: WorkflowGraphDefinition,
  sourceNode: WorkflowGraphNode | null,
  chain: WorkflowTemplateChainDefinition,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  const createdNodes = appendTemplateChainNodes(graph, chain.templates, operatorDescriptors);
  if (createdNodes.length === 0) return;
  if (sourceNode) connectNodes(graph, sourceNode, createdNodes[0]);
  for (const connection of chain.connections ?? []) {
    const fromNode = createdNodes[connection.from];
    const toNode = createdNodes[connection.to];
    if (!fromNode || !toNode) continue;
    connectNodes(graph, fromNode, toNode, connection.fromPort, connection.toPort);
  }
}

function upsertControlFlowEdge(
  graph: WorkflowGraphDefinition,
  mode: "outgoing" | "incoming",
  nodeId: string,
  portId: string,
  remoteNodeId: string,
  remotePortId: string,
) {
  const existingIndex = (graph.edges ?? []).findIndex((edge) =>
    mode === "outgoing"
      ? edge.from.node === nodeId && edge.from.port === portId
      : edge.to.node === nodeId && edge.to.port === portId,
  );
  const existingEdges = graph.edges ?? [];

  if (!remoteNodeId || !remotePortId) {
    if (existingIndex >= 0) {
      graph.edges = existingEdges.filter((_, index) => index !== existingIndex);
    }
    return;
  }

  const baseEdge = buildDraftEdge(
    existingIndex >= 0 ? existingIndex + 1 : existingEdges.length + 1,
    graph.nodes,
  );
  const nextEdge = syncEdgeFromPorts(
    {
      ...(existingIndex >= 0 ? existingEdges[existingIndex] : baseEdge),
      from:
        mode === "outgoing"
          ? { node: nodeId, port: portId }
          : { node: remoteNodeId, port: remotePortId },
      to:
        mode === "outgoing"
          ? { node: remoteNodeId, port: remotePortId }
          : { node: nodeId, port: portId },
    },
    graph.nodes,
  );

  if (existingIndex >= 0) {
    graph.edges = existingEdges.map((edge, index) => (index === existingIndex ? nextEdge : edge));
    return;
  }

  graph.edges = [...existingEdges, nextEdge];
}

export function createWorkflowTopologyActions(
  setDraftGraph: SetDraftGraph,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  function updateNode(nodeId: string, updater: (node: WorkflowGraphNode) => WorkflowGraphNode) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      next.nodes = next.nodes.map((node) => (node.id === nodeId ? updater(node) : node));
      return next;
    });
  }

  function updateNodePort(
    nodeId: string,
    direction: "inputs" | "outputs",
    portId: string,
    updater: (port: WorkflowGraphPort) => WorkflowGraphPort,
  ) {
    updateNode(nodeId, (node) => ({
      ...node,
      [direction]: (node[direction] ?? []).map((port) =>
        port.id === portId ? updater(port) : port,
      ),
    }));
  }

  function addNode(template?: WorkflowNodeTemplateSelection) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      appendConnectedNode(next, null, template, operatorDescriptors);
      return next;
    });
  }

  function addConnectedNode(
    sourceNodeId: string,
    template?: WorkflowNodeTemplateSelection,
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;

      const sourceNode = next.nodes.find((node) => node.id === sourceNodeId);
      if (!sourceNode) return current;
      appendConnectedNode(next, sourceNode, template, operatorDescriptors);
      return next;
    });
  }

  function syncNodeTemplate(nodeId: string, template?: WorkflowNodeTemplateSelection) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      const changed = applyWorkflowNodeTemplateSync(next, nodeId, template, operatorDescriptors);
      if (!changed) return current;
      ensureTemplateDatasetValues(next, template, operatorDescriptors);
      return next;
    });
  }

  function insertTemplateChain(
    chain: WorkflowTemplateChainDefinition,
    sourceNodeId?: string | null,
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;

      const sourceNode =
        sourceNodeId ? next.nodes.find((node) => node.id === sourceNodeId) ?? null : null;
      if (chain.connections?.length) {
        appendGraphTemplateChain(next, sourceNode, chain, operatorDescriptors);
      } else {
        appendLinearTemplateChain(next, sourceNode, chain.templates, operatorDescriptors);
      }
      return next;
    });
  }

  function insertControlFlowPlane(sourceNodeId?: string | null) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;

      const sourceNode =
        sourceNodeId ? next.nodes.find((node) => node.id === sourceNodeId) ?? null : null;
      const conditionNode = appendConnectedNode(next, sourceNode, { kind: "condition" }, operatorDescriptors);
      const mergeNode = appendConnectedNode(
        next,
        null,
        { kind: "transform", operatorId: "transform.first_available" },
        operatorDescriptors,
      );

      connectNodesByPorts(next, conditionNode.id, "if_true", mergeNode.id, "left");
      connectNodesByPorts(next, conditionNode.id, "if_false", mergeNode.id, "right");
      return next;
    });
  }

  function setControlFlowEdge(
    mode: "outgoing" | "incoming",
    nodeId: string,
    portId: string,
    target: string,
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      const [remoteNodeId = "", remotePortId = ""] = target.split(".");
      upsertControlFlowEdge(next, mode, nodeId, portId, remoteNodeId, remotePortId);
      return next;
    });
  }

  function removeNode(nodeId: string) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      next.nodes = next.nodes.filter((node) => node.id !== nodeId);
      next.edges = (next.edges ?? []).filter(
        (edge) => edge.from.node !== nodeId && edge.to.node !== nodeId,
      );
      next.entry_inputs = (next.entry_inputs ?? []).filter((artifact) => artifact.node_id !== nodeId);
      next.output_artifacts = (next.output_artifacts ?? []).filter(
        (artifact) => artifact.node_id !== nodeId,
      );
      next.entry_nodes = (next.entry_nodes ?? []).filter((entry) => entry !== nodeId);
      next.output_nodes = (next.output_nodes ?? []).filter((entry) => entry !== nodeId);
      return next;
    });
  }

  function addNodePort(nodeId: string, direction: "inputs" | "outputs") {
    updateNode(nodeId, (node) => ({
      ...node,
      [direction]: [
        ...(node[direction] ?? []),
        buildDraftPort(direction === "inputs" ? "in" : "out", (node[direction] ?? []).length + 1),
      ],
    }));
  }

  function removeNodePort(nodeId: string, direction: "inputs" | "outputs", portId: string) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      const node = next?.nodes.find((entry) => entry.id === nodeId);
      if (!next || !node) return current;
      node[direction] = (node[direction] ?? []).filter((port) => port.id !== portId);
      next.edges = (next.edges ?? []).filter((edge) => {
        if (direction === "outputs") return !(edge.from.node === nodeId && edge.from.port === portId);
        return !(edge.to.node === nodeId && edge.to.port === portId);
      });
      return next;
    });
  }

  function updateEdge(edgeId: string, updater: (edge: WorkflowGraphEdge) => WorkflowGraphEdge) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      next.edges = (next.edges ?? []).map((edge) =>
        edge.id === edgeId ? syncEdgeFromPorts(updater(edge), next.nodes) : edge,
      );
      return next;
    });
  }

  function addEdge() {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      const nextEdge = syncEdgeFromPorts(
        buildDraftEdge((next.edges ?? []).length + 1, next.nodes),
        next.nodes,
      );
      next.edges = [...(next.edges ?? []), nextEdge];
      return next;
    });
  }

  function removeEdge(edgeId: string) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      next.edges = (next.edges ?? []).filter((edge) => edge.id !== edgeId);
      return next;
    });
  }

  return {
    addEdge,
    addConnectedNode,
    insertControlFlowPlane,
    insertTemplateChain,
    addNode,
    addNodePort,
    removeEdge,
    removeNode,
    removeNodePort,
    setControlFlowEdge,
    syncNodeTemplate,
    updateEdge,
    updateNode,
    updateNodePort,
  };
}
