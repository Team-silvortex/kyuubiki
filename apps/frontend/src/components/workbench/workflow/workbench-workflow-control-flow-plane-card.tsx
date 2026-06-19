"use client";

import { useEffect, useRef, useState } from "react";
import type { WorkflowGraphEdge, WorkflowGraphNode, WorkflowOperatorDescriptor } from "@/lib/api";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import { WorkbenchWorkflowControlFlowHint } from "@/components/workbench/workflow/workbench-workflow-control-flow-hint";
import { WorkbenchWorkflowControlFlowQuickAddCard } from "@/components/workbench/workflow/workbench-workflow-control-flow-quick-add-card";
import { WorkbenchWorkflowControlFlowReadinessCard } from "@/components/workbench/workflow/workbench-workflow-control-flow-readiness-card";
import { WorkbenchWorkflowControlFlowSnapshotCard } from "@/components/workbench/workflow/workbench-workflow-control-flow-snapshot-card";
import { WorkbenchWorkflowControlFlowTemplateSwapCard } from "@/components/workbench/workflow/workbench-workflow-control-flow-template-swap-card";
import { WorkbenchWorkflowControlFlowWireOverlay, type WorkflowControlFlowWirePoint } from "@/components/workbench/workflow/workbench-workflow-control-flow-wire-overlay";
import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowControlFlowPlaneCardProps = {
  labels: WorkflowSidebarLabels;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  selectedNodes: WorkflowGraphNode[];
  selectedEdges: WorkflowGraphEdge[];
  validationIssues: WorkflowGraphValidationIssue[];
  invalidInputCount: number;
  traceFocusBranchNodeId?: string | null;
  traceFocusBranchOutputId?: string | null;
  traceFocusBranchToken?: number;
  onAddConditionNode: () => void;
  onAddMergeNode: () => void;
  onAddNode: (template?: WorkflowNodeTemplateSelection) => void;
  onSyncNodeTemplate: (nodeId: string, template?: WorkflowNodeTemplateSelection) => void;
  onInsertControlFlowPlane: (sourceNodeId?: string | null) => void;
  onSetControlFlowEdge: (
    mode: "outgoing" | "incoming",
    nodeId: string,
    portId: string,
    target: string,
  ) => void;
};

type ArmedConnection = { mode: "outgoing" | "incoming"; nodeId: string; portId: string };
type HoveredTarget = { nodeId: string; portId: string };

function isControlFlowNode(node: WorkflowGraphNode) {
  return node.kind === "condition" || node.operator_id === "transform.first_available";
}

function listOutgoingTargets(
  nodeId: string,
  portId: string,
  selectedEdges: WorkflowGraphEdge[],
) {
  return selectedEdges
    .filter((edge) => edge.from.node === nodeId && edge.from.port === portId)
    .map((edge) => `${edge.to.node}.${edge.to.port}`);
}

function listIncomingSources(
  nodeId: string,
  portId: string,
  selectedEdges: WorkflowGraphEdge[],
) {
  return selectedEdges
    .filter((edge) => edge.to.node === nodeId && edge.to.port === portId)
    .map((edge) => `${edge.from.node}.${edge.from.port}`);
}

function renderEndpointChips(values: string[], tone: "good" | "risk" | "watch") {
  if (values.length === 0) return <span className="card-copy">--</span>;
  return (
    <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
      {values.map((value) => (
        <span className={`status-pill status-pill--${tone}`} key={value}>
          {value}
        </span>
      ))}
    </div>
  );
}

function buildSocketButtonStyle(active: boolean) {
  return {
    display: "inline-flex",
    alignItems: "center",
    gap: "0.45rem",
    padding: "0.42rem 0.62rem",
    borderRadius: "999px",
    border: "1px solid var(--line)",
    background: active
      ? "linear-gradient(180deg, rgba(125,211,252,0.16), rgba(0,0,0,0.22))"
      : "linear-gradient(180deg, rgba(255,255,255,0.05), rgba(0,0,0,0.16))",
    boxShadow: active ? "0 0 0 1px rgba(125,211,252,0.2), 0 0 16px rgba(125,211,252,0.12)" : undefined,
  };
}

function buildLaneHighlightStyle(active: boolean, tone: "good" | "risk" | "watch") {
  if (!active) return undefined;
  const color =
    tone === "good"
      ? "rgba(34, 197, 94, 0.9)"
      : tone === "risk"
        ? "rgba(248, 113, 113, 0.9)"
        : "rgba(125, 211, 252, 0.95)";
  return {
    outline: `2px solid ${color}`,
    outlineOffset: "2px",
    boxShadow: `0 0 0 1px ${color.replace("0.9", "0.22").replace("0.95", "0.24")}, 0 0 18px ${color.replace("0.9", "0.14").replace("0.95", "0.16")}`,
  };
}

function laneKey(nodeId: string, lane: string) { return `${nodeId}:${lane}`; }
function portKey(nodeId: string, portId: string) { return `${nodeId}.${portId}`; }
function findNodeById(selectedNodes: WorkflowGraphNode[], nodeId: string) { return selectedNodes.find((node) => node.id === nodeId) ?? null; }

function firstTargetNodeId(nodeId: string, portId: string, selectedEdges: WorkflowGraphEdge[]) {
  return (
    selectedEdges.find((edge) => edge.from.node === nodeId && edge.from.port === portId)?.to.node ??
    null
  );
}

function buildControlFlowPlaneSnapshot(
  node: WorkflowGraphNode,
  selectedNodes: WorkflowGraphNode[],
  selectedEdges: WorkflowGraphEdge[],
) {
  if (node.kind !== "condition") return null;
  const mergeNodeId =
    firstTargetNodeId(node.id, "if_true", selectedEdges) ??
    firstTargetNodeId(node.id, "if_false", selectedEdges);
  const mergeNode =
    mergeNodeId && findNodeById(selectedNodes, mergeNodeId)?.operator_id === "transform.first_available"
      ? findNodeById(selectedNodes, mergeNodeId)
      : null;
  return {
    source: listIncomingSources(node.id, "value", selectedEdges),
    conditionId: node.id,
    trueTargets: listOutgoingTargets(node.id, "if_true", selectedEdges),
    falseTargets: listOutgoingTargets(node.id, "if_false", selectedEdges),
    mergeId: mergeNode?.id ?? null,
    mergeOutgoing: mergeNode ? listOutgoingTargets(mergeNode.id, "merged", selectedEdges) : [],
  };
}

function listTargetPortOptions(
  selectedNodes: WorkflowGraphNode[],
  currentNodeId: string,
  direction: "inputs" | "outputs",
) {
  return selectedNodes
    .filter((node) => node.id !== currentNodeId)
    .flatMap((node) =>
      (node[direction] ?? []).map((port) => ({
        value: `${node.id}.${port.id}`,
        label: `${node.id}.${port.id}`,
      })),
    );
}

function resolveCurrentTarget(
  nodeId: string,
  portId: string,
  selectedEdges: WorkflowGraphEdge[],
  mode: "outgoing" | "incoming",
) {
  const edge = selectedEdges.find((entry) =>
    mode === "outgoing"
      ? entry.from.node === nodeId && entry.from.port === portId
      : entry.to.node === nodeId && entry.to.port === portId,
  );
  if (!edge) return "";
  return mode === "outgoing"
    ? `${edge.to.node}.${edge.to.port}`
    : `${edge.from.node}.${edge.from.port}`;
}

function renderConnectionEditor(
  labels: WorkflowSidebarLabels,
  mode: "outgoing" | "incoming",
  node: WorkflowGraphNode,
  portId: string,
  tone: "good" | "risk" | "watch",
  active: boolean,
  armed: boolean,
  onArmConnection: (connection: ArmedConnection | null) => void,
  onConnectionCommitted: (sourcePortKey: string, targetPortKey: string) => void,
  recentConnectedSource: string | null,
  onRegisterPortButton: (key: string, element: HTMLButtonElement | null) => void,
  selectedNodes: WorkflowGraphNode[],
  selectedEdges: WorkflowGraphEdge[],
  onSetControlFlowEdge: WorkbenchWorkflowControlFlowPlaneCardProps["onSetControlFlowEdge"],
) {
  const options = listTargetPortOptions(
    selectedNodes,
    node.id,
    mode === "outgoing" ? "inputs" : "outputs",
  );
  return (
    <label
      className={`workflow-control-flow-connection-row${active ? " workflow-control-flow-connection-row--active" : ""}${armed ? " workflow-control-flow-connection-row--armed" : ""}`}
      style={{ display: "grid", gap: "0.35rem", gridTemplateColumns: "88px minmax(0,1fr)", padding: "0.2rem", borderRadius: "8px", ...buildLaneHighlightStyle(active, tone) }}
    >
      <button
        className={`workflow-control-flow-source-port${armed ? " workflow-control-flow-source-port--armed" : ""}${recentConnectedSource === portKey(node.id, portId) ? " workflow-control-flow-source-port--connected" : ""}`}
        onClick={() =>
          onArmConnection(
            armed ? null : { mode, nodeId: node.id, portId },
          )
        }
        ref={(element) => onRegisterPortButton(portKey(node.id, portId), element)}
        style={{ all: "unset", cursor: "pointer" }}
        type="button"
      >
        <span className={`status-pill status-pill--${armed ? "watch" : tone}`}>{portId}</span>
      </button>
      <select
        onChange={(event) => {
          onSetControlFlowEdge(mode, node.id, portId, event.target.value);
          if (event.target.value) {
            onConnectionCommitted(portKey(node.id, portId), event.target.value);
          }
        }}
        value={resolveCurrentTarget(node.id, portId, selectedEdges, mode)}
      >
        <option value="">{labels.addEdgeLabel}</option>
        {options.map((option) => (
          <option key={`${node.id}:${portId}:${option.value}`} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function renderControlPlaneCanvas(
  node: WorkflowGraphNode,
  selectedEdges: WorkflowGraphEdge[],
  labels: WorkflowSidebarLabels,
  selectedNodes: WorkflowGraphNode[],
  onSetControlFlowEdge: WorkbenchWorkflowControlFlowPlaneCardProps["onSetControlFlowEdge"],
  selectedLaneKey: string | null,
  armedConnection: ArmedConnection | null,
  onArmConnection: (connection: ArmedConnection | null) => void,
  onConnectionCommitted: (sourcePortKey: string, targetPortKey: string) => void,
  recentConnectedSource: string | null,
  onRegisterPortButton: (key: string, element: HTMLButtonElement | null) => void,
) {
  if (node.kind === "condition") {
    const incoming = listIncomingSources(node.id, "value", selectedEdges);
    const trueTargets = listOutgoingTargets(node.id, "if_true", selectedEdges);
    const falseTargets = listOutgoingTargets(node.id, "if_false", selectedEdges);
    const trueLaneActive = selectedLaneKey === laneKey(node.id, "true");
    const falseLaneActive = selectedLaneKey === laneKey(node.id, "false");
    return (
      <div
        className="workflow-control-flow-canvas workflow-control-flow-canvas--condition"
        style={{
          display: "grid",
          gap: "0.55rem",
          padding: "0.75rem",
          border: "1px solid var(--line)",
          borderRadius: "10px",
          background: "linear-gradient(180deg, rgba(255,255,255,0.03), rgba(0,0,0,0.16))",
          marginBottom: "0.75rem",
        }}
      >
        <div style={{ display: "grid", gap: "0.35rem", gridTemplateColumns: "88px minmax(0,1fr)" }}>
          <span className="card-copy">{labels.localWorkflowSourceLabel}</span>
          {renderEndpointChips(incoming, "watch")}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
          <span className="status-pill status-pill--watch">{node.id}</span>
          <span className="card-copy">value to branch</span>
        </div>
        <div style={{ display: "grid", gap: "0.35rem", gridTemplateColumns: "88px minmax(0,1fr)" }}>
          <span className="card-copy">true</span>
          {renderEndpointChips(trueTargets, "good")}
        </div>
        <div className="workflow-control-flow-lane workflow-control-flow-lane--true">
          {renderConnectionEditor(labels, "outgoing", node, "if_true", "good", trueLaneActive, armedConnection?.nodeId === node.id && armedConnection?.portId === "if_true", onArmConnection, onConnectionCommitted, recentConnectedSource, onRegisterPortButton, selectedNodes, selectedEdges, onSetControlFlowEdge)}
        </div>
        <div style={{ display: "grid", gap: "0.35rem", gridTemplateColumns: "88px minmax(0,1fr)" }}>
          <span className="card-copy">false</span>
          {renderEndpointChips(falseTargets, "risk")}
        </div>
        <div className="workflow-control-flow-lane workflow-control-flow-lane--false">
          {renderConnectionEditor(labels, "outgoing", node, "if_false", "risk", falseLaneActive, armedConnection?.nodeId === node.id && armedConnection?.portId === "if_false", onArmConnection, onConnectionCommitted, recentConnectedSource, onRegisterPortButton, selectedNodes, selectedEdges, onSetControlFlowEdge)}
        </div>
      </div>
    );
  }

  const leftSources = listIncomingSources(node.id, "left", selectedEdges);
  const rightSources = listIncomingSources(node.id, "right", selectedEdges);
  const mergedTargets = listOutgoingTargets(node.id, "merged", selectedEdges);
  const mergeLaneActive = selectedLaneKey === laneKey(node.id, "merge");
  const downstreamLaneActive = selectedLaneKey === laneKey(node.id, "downstream");
  return (
    <div
      className="workflow-control-flow-canvas workflow-control-flow-canvas--merge"
      style={{
        display: "grid",
        gap: "0.55rem",
        padding: "0.75rem",
        border: "1px solid var(--line)",
        borderRadius: "10px",
        background: "linear-gradient(180deg, rgba(255,255,255,0.03), rgba(0,0,0,0.16))",
        marginBottom: "0.75rem",
      }}
    >
      <div style={{ display: "grid", gap: "0.35rem", gridTemplateColumns: "88px minmax(0,1fr)" }}>
        <span className="card-copy">left</span>
        {renderEndpointChips(leftSources, "watch")}
      </div>
      <div className="workflow-control-flow-lane workflow-control-flow-lane--merge">
        {renderConnectionEditor(labels, "incoming", node, "left", "watch", mergeLaneActive, armedConnection?.nodeId === node.id && armedConnection?.portId === "left", onArmConnection, onConnectionCommitted, recentConnectedSource, onRegisterPortButton, selectedNodes, selectedEdges, onSetControlFlowEdge)}
      </div>
      <div style={{ display: "grid", gap: "0.35rem", gridTemplateColumns: "88px minmax(0,1fr)" }}>
        <span className="card-copy">right</span>
        {renderEndpointChips(rightSources, "watch")}
      </div>
      <div className="workflow-control-flow-lane workflow-control-flow-lane--merge">
        {renderConnectionEditor(labels, "incoming", node, "right", "watch", mergeLaneActive, armedConnection?.nodeId === node.id && armedConnection?.portId === "right", onArmConnection, onConnectionCommitted, recentConnectedSource, onRegisterPortButton, selectedNodes, selectedEdges, onSetControlFlowEdge)}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", flexWrap: "wrap" }}>
        <span className="status-pill status-pill--good">{node.id}</span>
        <span className="card-copy">{labels.controlFlowPlaneMergeLabel}</span>
      </div>
      <div style={{ display: "grid", gap: "0.35rem", gridTemplateColumns: "88px minmax(0,1fr)" }}>
        <span className="card-copy">merged</span>
        {renderEndpointChips(mergedTargets, "good")}
      </div>
      <div className="workflow-control-flow-lane workflow-control-flow-lane--downstream">
        {renderConnectionEditor(labels, "outgoing", node, "merged", "good", downstreamLaneActive, armedConnection?.nodeId === node.id && armedConnection?.portId === "merged", onArmConnection, onConnectionCommitted, recentConnectedSource, onRegisterPortButton, selectedNodes, selectedEdges, onSetControlFlowEdge)}
      </div>
    </div>
  );
}

function renderArmableTargetList(
  node: WorkflowGraphNode,
  armedConnection: ArmedConnection | null,
  selectedNodes: WorkflowGraphNode[],
  onSetControlFlowEdge: WorkbenchWorkflowControlFlowPlaneCardProps["onSetControlFlowEdge"],
  onArmConnection: (connection: ArmedConnection | null) => void,
  onConnectionCommitted: (sourcePortKey: string, targetPortKey: string) => void,
  hoveredTarget: HoveredTarget | null,
  onHoverTarget: (target: HoveredTarget | null) => void,
  onRegisterPortButton: (key: string, element: HTMLButtonElement | null) => void,
  recentConnectedTarget: string | null,
) {
  if (!armedConnection || armedConnection.nodeId === node.id) return null;
  const ports = node[
    armedConnection.mode === "outgoing" ? "inputs" : "outputs"
  ] ?? [];
  if (ports.length === 0) return null;
  return (
    <div style={{ display: "grid", gap: "0.35rem", marginBottom: "0.75rem" }}>
      <p className="card-copy">Connect armed port to this node:</p>
      <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
        {ports.map((port) => (
          <button
            className={`workflow-control-flow-target-port${hoveredTarget?.nodeId === node.id && hoveredTarget?.portId === port.id ? " workflow-control-flow-target-port--active" : ""}${recentConnectedTarget === portKey(node.id, port.id) ? " workflow-control-flow-target-port--connected" : ""}`}
            key={`${node.id}:${port.id}`}
            onMouseEnter={() => onHoverTarget({ nodeId: node.id, portId: port.id })}
            onMouseLeave={() => onHoverTarget(null)}
            onClick={() => {
              onSetControlFlowEdge(
                armedConnection.mode,
                armedConnection.nodeId,
                armedConnection.portId,
                `${node.id}.${port.id}`,
              );
              onConnectionCommitted(portKey(armedConnection.nodeId, armedConnection.portId), portKey(node.id, port.id));
              onArmConnection(null);
              onHoverTarget(null);
            }}
            style={{
              ...buildSocketButtonStyle(
                hoveredTarget?.nodeId === node.id && hoveredTarget?.portId === port.id,
              ),
              ...(hoveredTarget?.nodeId === node.id && hoveredTarget?.portId === port.id
                ? buildLaneHighlightStyle(true, "watch")
                : undefined),
            }}
            ref={(element) => onRegisterPortButton(portKey(node.id, port.id), element)}
            type="button"
          >
            <span aria-hidden="true" style={{ width: "0.5rem", height: "0.5rem", borderRadius: "999px", background: "rgba(125,211,252,0.9)", boxShadow: "0 0 0 3px rgba(125,211,252,0.12)" }} />
            <span>{node.id}.{port.id}</span>
          </button>
        ))}
      </div>
    </div>
  );
}

function canAcceptArmedConnection(node: WorkflowGraphNode, armedConnection: ArmedConnection | null) {
  if (!armedConnection || armedConnection.nodeId === node.id) return false;
  return (node[armedConnection.mode === "outgoing" ? "inputs" : "outputs"] ?? []).length > 0;
}

export function WorkbenchWorkflowControlFlowPlaneCard({
  labels,
  operatorDescriptors,
  selectedNodes,
  selectedEdges,
  validationIssues,
  invalidInputCount,
  traceFocusBranchNodeId,
  traceFocusBranchOutputId,
  traceFocusBranchToken,
  onAddConditionNode,
  onAddMergeNode,
  onAddNode,
  onSyncNodeTemplate,
  onInsertControlFlowPlane,
  onSetControlFlowEdge,
}: WorkbenchWorkflowControlFlowPlaneCardProps) {
  const [selectedLaneKey, setSelectedLaneKey] = useState<string | null>(null);
  const [armedConnection, setArmedConnection] = useState<ArmedConnection | null>(null);
  const [hoveredTarget, setHoveredTarget] = useState<HoveredTarget | null>(null);
  const [pointerPoint, setPointerPoint] = useState<WorkflowControlFlowWirePoint | null>(null);
  const [recentConnectedSource, setRecentConnectedSource] = useState<string | null>(null);
  const [recentConnectedTarget, setRecentConnectedTarget] = useState<string | null>(null);
  const [recentConnectedNodeId, setRecentConnectedNodeId] = useState<string | null>(null);
  const sectionRef = useRef<HTMLElement | null>(null);
  const portButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const controlNodes = selectedNodes.filter(isControlFlowNode);
  const selectedSourceNodeId =
    selectedNodes.find((node) => !isControlFlowNode(node))?.id ??
    selectedNodes[0]?.id ??
    null;
  const controlSnapshots = controlNodes
    .map((node) => buildControlFlowPlaneSnapshot(node, selectedNodes, selectedEdges))
    .filter((value): value is NonNullable<typeof value> => Boolean(value));

  function registerPortButton(key: string, element: HTMLButtonElement | null) {
    portButtonRefs.current[key] = element;
  }

  function resolvePortCenter(key: string): WorkflowControlFlowWirePoint | null {
    const button = portButtonRefs.current[key];
    const section = sectionRef.current;
    if (!button || !section) return null;
    const buttonRect = button.getBoundingClientRect();
    const sectionRect = section.getBoundingClientRect();
    return {
      x: buttonRect.left - sectionRect.left + buttonRect.width / 2,
      y: buttonRect.top - sectionRect.top + buttonRect.height / 2,
    };
  }

  const wireSource = armedConnection
    ? resolvePortCenter(portKey(armedConnection.nodeId, armedConnection.portId))
    : null;
  const wireTarget = hoveredTarget
    ? resolvePortCenter(portKey(hoveredTarget.nodeId, hoveredTarget.portId))
    : pointerPoint;

  useEffect(() => {
    if (!armedConnection) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setArmedConnection(null); setHoveredTarget(null); setPointerPoint(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [armedConnection]);
  useEffect(() => {
    if (!recentConnectedSource && !recentConnectedTarget) return;
    const handle = window.setTimeout(() => {
      setRecentConnectedSource(null);
      setRecentConnectedTarget(null);
      setRecentConnectedNodeId(null);
    }, 1200);
    return () => window.clearTimeout(handle);
  }, [recentConnectedSource, recentConnectedTarget, recentConnectedNodeId]);
  useEffect(() => { if (!traceFocusBranchNodeId || !traceFocusBranchOutputId) return; setSelectedLaneKey(laneKey(traceFocusBranchNodeId, traceFocusBranchOutputId === "if_false" ? "false" : "true")); }, [traceFocusBranchNodeId, traceFocusBranchOutputId, traceFocusBranchToken]);

  function handleConnectionCommitted(sourcePortKey: string, targetPortKey: string) {
    setRecentConnectedSource(sourcePortKey);
    setRecentConnectedTarget(targetPortKey);
    setRecentConnectedNodeId(targetPortKey.split(".")[0] ?? null);
  }

  return (
    <section
      className="sidebar-card sidebar-card--compact workflow-control-flow-card"
      data-workflow-control-flow="plane"
      onPointerLeave={() => setPointerPoint(null)}
      onPointerMove={(event) => {
        const section = sectionRef.current;
        if (!section || !armedConnection) return;
        const rect = section.getBoundingClientRect();
        setPointerPoint({
          x: event.clientX - rect.left,
          y: event.clientY - rect.top,
        });
      }}
      ref={sectionRef}
      style={{ position: "relative" }}
    >
      <WorkbenchWorkflowControlFlowWireOverlay source={armedConnection ? wireSource : null} target={armedConnection ? wireTarget : null} />
      <div className="card-head">
        <h2>{labels.controlFlowPlaneTitle}</h2>
        <span className={`status-pill status-pill--${controlNodes.length > 0 ? "good" : "watch"}`}>
          {controlNodes.length}
        </span>
      </div>
      <p className="card-copy">{labels.controlFlowPlaneHint}</p>
      {selectedSourceNodeId ? (
        <div className="sidebar-list workflow-control-flow-deck">
          <div className="sidebar-list__row">
            <span>{labels.localWorkflowSourceLabel}</span>
            <strong>{selectedSourceNodeId}</strong>
          </div>
        </div>
      ) : null}
      <div className="button-row button-row--adaptive workflow-control-flow-deck">
        <button onClick={() => onInsertControlFlowPlane(selectedSourceNodeId)} type="button">
          {labels.controlFlowPlaneInsertLabel}
        </button>
        <button onClick={onAddConditionNode} type="button">
          {labels.controlFlowPlaneConditionLabel}
        </button>
        <button onClick={onAddMergeNode} type="button">
          {labels.controlFlowPlaneMergeLabel}
        </button>
      </div>
      <WorkbenchWorkflowControlFlowReadinessCard invalidInputCount={invalidInputCount} labels={labels} selectedEdges={selectedEdges} selectedNodes={selectedNodes} validationIssues={validationIssues} />
      <WorkbenchWorkflowControlFlowQuickAddCard labels={labels} onAddNode={onAddNode} operatorDescriptors={operatorDescriptors} />
      {armedConnection ? (
        <div
          className="workflow-control-flow-arm-panel"
          style={{
            display: "grid",
            gap: "0.35rem",
            marginTop: "0.75rem",
            marginBottom: "0.75rem",
            padding: "0.7rem 0.8rem",
            border: "1px dashed rgba(125, 211, 252, 0.75)",
            borderRadius: "10px",
            background: "linear-gradient(180deg, rgba(125, 211, 252, 0.08), rgba(0, 0, 0, 0.18))",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "0.45rem", flexWrap: "wrap" }}>
            <span className="status-pill status-pill--watch">armed</span>
            <strong>{armedConnection.nodeId}.{armedConnection.portId}</strong>
            <span className="card-copy">Select a compatible target port below to complete the connection.</span>
          </div>
          <div className="card-copy">
            {hoveredTarget
              ? `Preview: ${armedConnection.nodeId}.${armedConnection.portId} -> ${hoveredTarget.nodeId}.${hoveredTarget.portId}`
              : "Preview: waiting for target"}
          </div>
        </div>
      ) : null}
      {controlNodes.length === 0 ? (
        <p className="card-copy">{labels.controlFlowPlaneEmptyLabel}</p>
      ) : null}
      {controlSnapshots.map((snapshot) => (
        <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={`snapshot:${snapshot.conditionId}`}>
          <div className="card-head">
            <h2>{snapshot.conditionId}</h2>
            <span className="status-pill status-pill--watch">{labels.controlFlowPlaneTitle}</span>
          </div>
          <WorkbenchWorkflowControlFlowSnapshotCard labels={labels} onSelectLane={setSelectedLaneKey} selectedLaneKey={selectedLaneKey} snapshot={snapshot} />
        </section>
      ))}
      <div className="runtime-overview-grid">
      {controlNodes.map((node) => (
          <section
            className={`sidebar-card sidebar-card--compact runtime-overview-card${canAcceptArmedConnection(node, armedConnection) ? " workflow-control-flow-accepting-node" : ""}${recentConnectedNodeId === node.id ? " workflow-control-flow-node--connected" : ""}`}
            key={`control:${node.id}`}
            style={canAcceptArmedConnection(node, armedConnection) ? buildLaneHighlightStyle(true, "watch") : undefined}
          >
            <div className="card-head">
              <h2>{node.id}</h2>
              <span
                className={`status-pill status-pill--${node.kind === "condition" ? "watch" : "good"}`}
              >
                {node.kind === "condition"
                  ? labels.controlFlowPlaneConditionLabel
                  : labels.controlFlowPlaneMergeLabel}
              </span>
            </div>
            <WorkbenchWorkflowControlFlowTemplateSwapCard labels={labels} node={node} onSyncNodeTemplate={onSyncNodeTemplate} operatorDescriptors={operatorDescriptors} />
            {renderArmableTargetList(node, armedConnection, selectedNodes, onSetControlFlowEdge, setArmedConnection, handleConnectionCommitted, hoveredTarget, setHoveredTarget, registerPortButton, recentConnectedTarget)}
            {renderControlPlaneCanvas(node, selectedEdges, labels, selectedNodes, onSetControlFlowEdge, selectedLaneKey, armedConnection, setArmedConnection, handleConnectionCommitted, recentConnectedSource, registerPortButton)}
            <WorkbenchWorkflowControlFlowHint node={node} selectedEdges={selectedEdges} />
          </section>
        ))}
      </div>
    </section>
  );
}
