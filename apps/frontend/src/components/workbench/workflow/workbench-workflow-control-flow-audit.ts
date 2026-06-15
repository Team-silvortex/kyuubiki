"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";

type ControlFlowAuditPayload = {
  context?: Record<string, unknown>;
  detail?: string;
  kind: "control_flow_edge_updated" | "control_flow_node_added" | "control_flow_plane_inserted";
  message: string;
};

function findEdge(graph: WorkflowGraphDefinition | null | undefined, mode: "outgoing" | "incoming", nodeId: string, portId: string) {
  return (graph?.edges ?? []).find((edge) =>
    mode === "outgoing"
      ? edge.from.node === nodeId && edge.from.port === portId
      : edge.to.node === nodeId && edge.to.port === portId,
  ) ?? null;
}

function formatEdgeEndpoint(nodeId: string, portId: string) {
  return nodeId && portId ? `${nodeId}.${portId}` : "--";
}

export function buildControlFlowEdgeAuditPayload(params: {
  graph: WorkflowGraphDefinition | null | undefined;
  mode: "outgoing" | "incoming";
  nodeId: string;
  portId: string;
  target: string;
}) : ControlFlowAuditPayload {
  const existing = findEdge(params.graph, params.mode, params.nodeId, params.portId);
  const [remoteNodeId = "", remotePortId = ""] = params.target.split(".");
  const branchNodeId = params.portId === "if_true" || params.portId === "if_false" ? params.nodeId : existing?.from.port === "if_true" || existing?.from.port === "if_false" ? existing.from.node : null;
  const branchOutputId = params.portId === "if_true" || params.portId === "if_false" ? params.portId : existing?.from.port === "if_true" || existing?.from.port === "if_false" ? existing.from.port : null;
  const previousTarget = existing ? formatEdgeEndpoint(existing.to.node, existing.to.port) : null;
  const nextTarget = params.target || null;
  return {
    kind: "control_flow_edge_updated",
    message: params.target ? "Updated control-flow wire." : "Removed control-flow wire.",
    detail: params.target ? `${params.nodeId}.${params.portId} -> ${params.target}` : `${params.nodeId}.${params.portId} disconnected`,
    context: {
      edgeId: existing?.id ?? null,
      branchNodeId,
      branchOutputId,
      controlFlowMode: params.mode,
      edgeFromNodeId: params.mode === "outgoing" ? params.nodeId : remoteNodeId,
      edgeFromPortId: params.mode === "outgoing" ? params.portId : remotePortId,
      edgeToNodeId: params.mode === "outgoing" ? remoteNodeId : params.nodeId,
      edgeToPortId: params.mode === "outgoing" ? remotePortId : params.portId,
      previousTarget,
      nextTarget,
    },
  };
}

export function buildControlFlowPlaneInsertAuditPayload(sourceNodeId?: string | null): ControlFlowAuditPayload {
  return {
    kind: "control_flow_plane_inserted",
    message: "Inserted control-flow plane.",
    detail: sourceNodeId ? `Connected from ${sourceNodeId}` : "Inserted detached condition/merge plane",
    context: { nodeId: sourceNodeId ?? null },
  };
}

export function buildControlFlowNodeAddAuditPayload(kind: "condition" | "merge"): ControlFlowAuditPayload {
  return {
    kind: "control_flow_node_added",
    message: kind === "condition" ? "Added condition control node." : "Added merge control node.",
    detail: kind === "condition" ? "Condition branch node inserted into the workflow graph." : "First-available merge node inserted into the workflow graph.",
    context: { controlFlowNodeKind: kind },
  };
}
