"use client";

import type { Dispatch, SetStateAction } from "react";
import type {
  WorkflowDatasetValueInfo,
  WorkflowGraphDefinition,
  WorkflowGraphEdge,
  WorkflowGraphNode,
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
} from "@/components/workbench/workflow/workbench-workflow-node-templates";

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
  template?: { kind?: string; operatorId?: string },
) {
  const datasetValues = listWorkflowTemplateDatasetValues(template);
  if (datasetValues.length === 0) return;

  const currentContract = graph.dataset_contract;
  if (currentContract) {
    currentContract.values = mergeMissingDatasetValues(currentContract.values, datasetValues);
  } else {
    graph.dataset_contract = {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: `${graph.id}.dataset`,
      version: graph.version ?? "1.4.0",
      name: `${graph.name ?? graph.id} dataset contract`,
      values: datasetValues,
      metadata: {},
    };
  }
}

function appendConnectedNode(
  graph: WorkflowGraphDefinition,
  sourceNode: WorkflowGraphNode | null,
  template?: { kind?: string; operatorId?: string },
) {
  const createdNode = buildDraftNode(graph.nodes.length + 1, template);
  graph.nodes = [...graph.nodes, createdNode];
  ensureTemplateDatasetValues(graph, template);

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

export function createWorkflowTopologyActions(setDraftGraph: SetDraftGraph) {
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

  function addNode(template?: { kind?: string; operatorId?: string }) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      appendConnectedNode(next, null, template);
      return next;
    });
  }

  function addConnectedNode(
    sourceNodeId: string,
    template?: { kind?: string; operatorId?: string },
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;

      const sourceNode = next.nodes.find((node) => node.id === sourceNodeId);
      if (!sourceNode) return current;
      appendConnectedNode(next, sourceNode, template);
      return next;
    });
  }

  function insertTemplateChain(
    templates: Array<{ kind?: string; operatorId?: string }>,
    sourceNodeId?: string | null,
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;

      let previousNode =
        sourceNodeId ? next.nodes.find((node) => node.id === sourceNodeId) ?? null : null;
      for (const template of templates) {
        previousNode = appendConnectedNode(next, previousNode, template);
      }
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
    insertTemplateChain,
    addNode,
    addNodePort,
    removeEdge,
    removeNode,
    removeNodePort,
    updateEdge,
    updateNode,
    updateNodePort,
  };
}
