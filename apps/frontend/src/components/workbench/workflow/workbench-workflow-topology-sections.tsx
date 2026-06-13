"use client";

import { memo, useEffect, useState } from "react";
import type {
  WorkflowGraphEdge,
  WorkflowGraphNode,
  WorkflowGraphPort,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import type {
  HeatPlaneStudyJobInput,
  PlaneStudyJobInput,
  StudyKind,
} from "@/components/workbench/workbench-types";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import {
  buildOperatorOptionLabel,
  WorkbenchWorkflowOperatorDescriptorSummary,
} from "@/components/workbench/workflow/workbench-workflow-operator-descriptor-summary";
import { WorkbenchWorkflowBridgeContractEditor } from "@/components/workbench/workflow/workbench-workflow-bridge-contract-editor";
import { WorkbenchWorkflowConditionEditor } from "@/components/workbench/workflow/workbench-workflow-condition-editor";
import { WorkbenchWorkflowControlFlowHint } from "@/components/workbench/workflow/workbench-workflow-control-flow-hint";

type WorkflowOperatorOptionPreset = {
  id: string;
  label: string;
  operatorId?: string;
};

function WorkbenchWorkflowPortEditor(props: {
  labels: WorkflowSidebarLabels;
  nodeId: string;
  ports: WorkflowGraphPort[];
  direction: "inputs" | "outputs";
  title: string;
  locked?: boolean;
  onAdd: () => void;
  onRemove: (portId: string) => void;
  onUpdate: (portId: string, updater: (port: WorkflowGraphPort) => WorkflowGraphPort) => void;
}) {
  const { labels, nodeId, ports, direction, title, locked = false, onAdd, onRemove, onUpdate } = props;
  return (
    <div className="sidebar-stack">
      <div className="card-head">
        <h3>{title}</h3>
        {locked ? <span className="status-pill status-pill--watch">descriptor</span> : <button onClick={onAdd} type="button">{direction === "inputs" ? labels.addInputPortLabel : labels.addOutputPortLabel}</button>}
      </div>
      {ports.map((port, index) => (
        <div className="form-grid compact" key={`${nodeId}:${direction}:${port.id}`}>
          <label>
            <span>{labels.portIdLabel}</span>
            <input data-workflow-port-field={`${nodeId}:${direction}:${port.id}:id`} data-workflow-port-stable={`${nodeId}:${direction}:${index}:id`} disabled={locked} onChange={(event) => onUpdate(port.id, (current) => ({ ...current, id: event.target.value }))} value={port.id} />
          </label>
          <label>
            <span>{labels.artifactTypeLabel}</span>
            <input data-workflow-port-field={`${nodeId}:${direction}:${port.id}:artifact_type`} data-workflow-port-stable={`${nodeId}:${direction}:${index}:artifact_type`} disabled={locked} onChange={(event) => onUpdate(port.id, (current) => ({ ...current, artifact_type: event.target.value }))} value={port.artifact_type} />
          </label>
          <label>
            <span>{labels.artifactDescriptionLabel}</span>
            <input data-workflow-port-field={`${nodeId}:${direction}:${port.id}:description`} data-workflow-port-stable={`${nodeId}:${direction}:${index}:description`} disabled={locked} onChange={(event) => onUpdate(port.id, (current) => ({ ...current, description: event.target.value || undefined }))} value={port.description ?? ""} />
          </label>
          {locked ? null : <button onClick={() => onRemove(port.id)} type="button">{labels.removePortLabel}</button>}
        </div>
      ))}
    </div>
  );
}

function buildNodeHighlightStyle(isFocused: boolean, isHighlighted: boolean) {
  if (!isHighlighted && !isFocused) return undefined;
  const highlighted = isHighlighted;
  if (!highlighted && !isFocused) return undefined;
  return highlighted
    ? { outline: "2px solid rgba(34, 197, 94, 0.9)", outlineOffset: "2px", boxShadow: "0 0 0 1px rgba(34, 197, 94, 0.22), 0 0 18px rgba(34, 197, 94, 0.18)" }
    : { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" };
}

function buildEdgeHighlightStyle(
  isFocused: boolean,
  isHighlighted: boolean,
  isLocallyHighlighted: boolean,
) {
  const highlighted = isHighlighted || isLocallyHighlighted;
  if (!highlighted && !isFocused) return undefined;
  return {
    outline: highlighted ? "2px solid rgba(34, 197, 94, 0.9)" : "2px solid var(--accent, #4f46e5)",
    outlineOffset: "2px",
    boxShadow: highlighted ? "0 0 0 1px rgba(34, 197, 94, 0.22), 0 0 18px rgba(34, 197, 94, 0.18)" : undefined,
  };
}

type WorkbenchWorkflowTopologyNodeSectionProps = {
  labels: WorkflowSidebarLabels;
  node: WorkflowGraphNode;
  operatorDescriptor?: WorkflowOperatorDescriptor;
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>;
  nodeOperatorPresets: WorkflowOperatorOptionPreset[];
  isFocused: boolean;
  isHighlighted: boolean;
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: HeatPlaneStudyJobInput;
  currentPlaneModel: PlaneStudyJobInput;
  controlFlowEdges: WorkflowGraphEdge[];
  bridgePeerNodes: WorkflowGraphNode[];
  nextNodeKind: string;
  nextOperatorId: string;
  onAddConnectedNode: (sourceNodeId: string, template?: { kind: string; operatorId?: string }) => void;
  onConfirmNodeTemplateSync: (node: WorkflowGraphNode, operatorId?: string) => boolean;
  onRemoveNode: (nodeId: string) => void;
  onUpdateNode: (nodeId: string, updater: (node: WorkflowGraphNode) => WorkflowGraphNode) => void;
  onSyncNodeTemplate: (nodeId: string, template?: { kind: string; operatorId?: string }) => void;
  onAddNodePort: (nodeId: string, direction: "inputs" | "outputs") => void;
  onRemoveNodePort: (nodeId: string, direction: "inputs" | "outputs", portId: string) => void;
  onUpdateNodePort: (nodeId: string, direction: "inputs" | "outputs", portId: string, updater: (port: WorkflowGraphPort) => WorkflowGraphPort) => void;
};

function WorkbenchWorkflowTopologyNodeSectionImpl({
  labels,
  node,
  operatorDescriptor,
  operatorDescriptorMap,
  nodeOperatorPresets,
  isFocused,
  isHighlighted,
  currentStudyKind,
  currentHeatPlaneModel,
  currentPlaneModel,
  controlFlowEdges,
  bridgePeerNodes,
  nextNodeKind,
  nextOperatorId,
  onAddConnectedNode,
  onConfirmNodeTemplateSync,
  onRemoveNode,
  onUpdateNode,
  onSyncNodeTemplate,
  onAddNodePort,
  onRemoveNodePort,
  onUpdateNodePort,
}: WorkbenchWorkflowTopologyNodeSectionProps) {
  const templateLocked = Boolean(operatorDescriptor);
  return (
    <section className="sidebar-card sidebar-card--compact" data-workflow-node-id={node.id} style={buildNodeHighlightStyle(isFocused, isHighlighted)}>
      <div className="card-head">
        <h2>{node.id}</h2>
        <div className="button-row">
          <button onClick={() => onAddConnectedNode(node.id, { kind: nextNodeKind, operatorId: nextOperatorId || undefined })} type="button">{labels.addConnectedNodeLabel}</button>
          <button onClick={() => onRemoveNode(node.id)} type="button">{labels.removeNodeLabel}</button>
        </div>
      </div>
      <div className="form-grid compact">
        <label>
          <span>{labels.nodeIdLabel}</span>
          <input data-workflow-node-field={`${node.id}:id`} onChange={(event) => onUpdateNode(node.id, (current) => ({ ...current, id: event.target.value }))} value={node.id} />
        </label>
        <label>
          <span>{labels.kindLabel}</span>
          <input data-workflow-node-field={`${node.id}:kind`} disabled={templateLocked} onChange={(event) => onUpdateNode(node.id, (current) => ({ ...current, kind: event.target.value }))} value={node.kind} />
        </label>
        <label>
          <span>{labels.operatorLabel}</span>
          <input data-workflow-node-field={`${node.id}:operator_id`} list={`workflow-node-operators-${node.id}`} onChange={(event) => onConfirmNodeTemplateSync(node, event.target.value || undefined) ? onSyncNodeTemplate(node.id, { kind: node.kind, operatorId: event.target.value || undefined }) : undefined} value={node.operator_id ?? ""} />
          <datalist id={`workflow-node-operators-${node.id}`}>
            {nodeOperatorPresets.map((preset) => (
              <option key={preset.id} label={buildOperatorOptionLabel(labels, preset.label, preset.operatorId ? operatorDescriptorMap.get(preset.operatorId) : undefined)} value={preset.operatorId} />
            ))}
          </datalist>
        </label>
      </div>
      <WorkbenchWorkflowOperatorDescriptorSummary descriptor={operatorDescriptor} labels={labels} />
      <WorkbenchWorkflowControlFlowHint node={node} selectedEdges={controlFlowEdges} />
      <WorkbenchWorkflowBridgeContractEditor currentHeatPlaneModel={currentHeatPlaneModel as unknown as Record<string, unknown>} currentPlaneModel={currentPlaneModel as unknown as Record<string, unknown>} currentStudyKind={currentStudyKind} labels={labels} node={node} selectedNodes={bridgePeerNodes} onUpdateNode={onUpdateNode} />
      <WorkbenchWorkflowConditionEditor labels={labels} node={node} onUpdateNode={onUpdateNode} />
      <WorkbenchWorkflowPortEditor direction="inputs" labels={labels} locked={templateLocked} nodeId={node.id} onAdd={() => onAddNodePort(node.id, "inputs")} onRemove={(portId) => onRemoveNodePort(node.id, "inputs", portId)} onUpdate={(portId, updater) => onUpdateNodePort(node.id, "inputs", portId, updater)} ports={node.inputs ?? []} title={labels.inputsTitle} />
      <WorkbenchWorkflowPortEditor direction="outputs" labels={labels} locked={templateLocked} nodeId={node.id} onAdd={() => onAddNodePort(node.id, "outputs")} onRemove={(portId) => onRemoveNodePort(node.id, "outputs", portId)} onUpdate={(portId, updater) => onUpdateNodePort(node.id, "outputs", portId, updater)} ports={node.outputs ?? []} title={labels.outputsTitle} />
    </section>
  );
}

export const WorkbenchWorkflowTopologyNodeSection = memo(WorkbenchWorkflowTopologyNodeSectionImpl);

type WorkbenchWorkflowTopologyEdgeSectionProps = {
  labels: WorkflowSidebarLabels;
  edge: WorkflowGraphEdge;
  nodeSelectOptions: Array<{ id: string }>;
  sourcePorts: WorkflowGraphPort[];
  targetPorts: WorkflowGraphPort[];
  selectedNodeMap: Map<string, WorkflowGraphNode>;
  isFocused: boolean;
  isHighlighted: boolean;
  isLocallyHighlighted: boolean;
  onRemoveEdge: (edgeId: string) => void;
  onUpdateEdge: (edgeId: string, updater: (edge: WorkflowGraphEdge) => WorkflowGraphEdge) => void;
};

function WorkbenchWorkflowTopologyEdgeSectionImpl({
  labels,
  edge,
  nodeSelectOptions,
  sourcePorts,
  targetPorts,
  selectedNodeMap,
  isFocused,
  isHighlighted,
  isLocallyHighlighted,
  onRemoveEdge,
  onUpdateEdge,
}: WorkbenchWorkflowTopologyEdgeSectionProps) {
  const [artifactTypeDraft, setArtifactTypeDraft] = useState(edge.artifact_type);
  useEffect(() => {
    setArtifactTypeDraft(edge.artifact_type);
  }, [edge.artifact_type, edge.id]);
  function commitArtifactTypeDraft() {
    if (artifactTypeDraft === edge.artifact_type) return;
    onUpdateEdge(edge.id, (current) => ({
      ...current,
      artifact_type: artifactTypeDraft,
    }));
  }
  return (
    <section className="sidebar-card sidebar-card--compact" data-workflow-edge-id={edge.id} style={buildEdgeHighlightStyle(isFocused, isHighlighted, isLocallyHighlighted)}>
      <div className="card-head">
        <h2>{edge.id}</h2>
        <button onClick={() => onRemoveEdge(edge.id)} type="button">{labels.removeEdgeLabel}</button>
      </div>
      <div className="form-grid compact">
        <label>
          <span>{labels.edgeIdLabel}</span>
          <input data-workflow-edge-field={`${edge.id}:id`} onChange={(event) => onUpdateEdge(edge.id, (current) => ({ ...current, id: event.target.value }))} value={edge.id} />
        </label>
        <label>
          <span>{labels.fromLabel}</span>
          <select data-workflow-edge-select={`${edge.id}:from.node`} onChange={(event) => onUpdateEdge(edge.id, (current) => ({ ...current, from: { node: event.target.value, port: selectedNodeMap.get(event.target.value)?.outputs?.[0]?.id ?? "" } }))} value={edge.from.node}>
            <option value="">--</option>
            {nodeSelectOptions.map((node) => <option key={`from:${node.id}`} value={node.id}>{node.id}</option>)}
          </select>
        </label>
        <label>
          <span>{labels.portIdLabel}</span>
          <select data-workflow-edge-select={`${edge.id}:from.port`} onChange={(event) => onUpdateEdge(edge.id, (current) => ({ ...current, from: { ...current.from, port: event.target.value } }))} value={edge.from.port}>
            <option value="">--</option>
            {sourcePorts.map((port) => <option key={`from:${edge.id}:${port.id}`} value={port.id}>{port.id}</option>)}
          </select>
        </label>
        <label>
          <span>{labels.toLabel}</span>
          <select data-workflow-edge-select={`${edge.id}:to.node`} onChange={(event) => onUpdateEdge(edge.id, (current) => ({ ...current, to: { node: event.target.value, port: selectedNodeMap.get(event.target.value)?.inputs?.[0]?.id ?? "" } }))} value={edge.to.node}>
            <option value="">--</option>
            {nodeSelectOptions.map((node) => <option key={`to:${node.id}`} value={node.id}>{node.id}</option>)}
          </select>
        </label>
        <label>
          <span>{labels.portIdLabel}</span>
          <select data-workflow-edge-select={`${edge.id}:to.port`} onChange={(event) => onUpdateEdge(edge.id, (current) => ({ ...current, to: { ...current.to, port: event.target.value } }))} value={edge.to.port}>
            <option value="">--</option>
            {targetPorts.map((port) => <option key={`to:${edge.id}:${port.id}`} value={port.id}>{port.id}</option>)}
          </select>
        </label>
        <label>
          <span>{labels.artifactTypeLabel}</span>
          <input
            data-workflow-edge-field={`${edge.id}:artifact_type`}
            onBlur={commitArtifactTypeDraft}
            onChange={(event) => setArtifactTypeDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key !== "Enter") return;
              event.currentTarget.blur();
            }}
            value={artifactTypeDraft}
          />
        </label>
      </div>
    </section>
  );
}

export const WorkbenchWorkflowTopologyEdgeSection = memo(WorkbenchWorkflowTopologyEdgeSectionImpl);
