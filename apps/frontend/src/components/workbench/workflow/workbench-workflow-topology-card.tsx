"use client";

import { useState } from "react";
import type {
  WorkflowGraphEdge,
  WorkflowGraphNode,
  WorkflowOperatorDescriptor,
  WorkflowGraphPort,
} from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import { listWorkflowNodeTemplatePresets } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import {
  buildOperatorOptionLabel,
  sortWorkflowOperatorOptionPresets,
  WorkbenchWorkflowOperatorDescriptorSummary,
} from "@/components/workbench/workflow/workbench-workflow-operator-descriptor-summary";

type WorkbenchWorkflowTopologyCardProps = {
  labels: WorkflowSidebarLabels;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  selectedNodes: WorkflowGraphNode[];
  selectedEdges: WorkflowGraphEdge[];
  focusedNodeId?: string | null;
  focusedEdgeId?: string | null;
  onAddNode: (template?: { kind?: string; operatorId?: string }) => void;
  onAddConnectedNode: (
    sourceNodeId: string,
    template?: { kind?: string; operatorId?: string },
  ) => void;
  onSyncNodeTemplate: (nodeId: string, template?: { kind?: string; operatorId?: string }) => void;
  onInsertTemplateChain: (
    templates: Array<{ kind?: string; operatorId?: string }>,
    sourceNodeId?: string | null,
  ) => void;
  onRemoveNode: (nodeId: string) => void;
  onUpdateNode: (
    nodeId: string,
    updater: (node: WorkflowGraphNode) => WorkflowGraphNode,
  ) => void;
  onAddNodePort: (nodeId: string, direction: "inputs" | "outputs") => void;
  onRemoveNodePort: (
    nodeId: string,
    direction: "inputs" | "outputs",
    portId: string,
  ) => void;
  onUpdateNodePort: (
    nodeId: string,
    direction: "inputs" | "outputs",
    portId: string,
    updater: (port: WorkflowGraphPort) => WorkflowGraphPort,
  ) => void;
  onAddEdge: () => void;
  onRemoveEdge: (edgeId: string) => void;
  onUpdateEdge: (
    edgeId: string,
    updater: (edge: WorkflowGraphEdge) => WorkflowGraphEdge,
  ) => void;
};

const NODE_KIND_OPTIONS = ["input", "solve", "transform", "extract", "export", "output", "condition"];

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
        {locked ? (
          <span className="status-pill status-pill--watch">descriptor</span>
        ) : (
          <button onClick={onAdd} type="button">
            {direction === "inputs" ? labels.addInputPortLabel : labels.addOutputPortLabel}
          </button>
        )}
      </div>
      {ports.map((port) => (
        <div className="form-grid compact" key={`${nodeId}:${direction}:${port.id}`}>
          <label>
            <span>{labels.portIdLabel}</span>
            <input
              disabled={locked}
              onChange={(event) =>
                onUpdate(port.id, (current) => ({
                  ...current,
                  id: event.target.value,
                }))
              }
              value={port.id}
            />
          </label>
          <label>
            <span>{labels.artifactTypeLabel}</span>
            <input
              disabled={locked}
              onChange={(event) =>
                onUpdate(port.id, (current) => ({
                  ...current,
                  artifact_type: event.target.value,
                }))
              }
              value={port.artifact_type}
            />
          </label>
          <label>
            <span>{labels.artifactDescriptionLabel}</span>
            <input
              disabled={locked}
              onChange={(event) =>
                onUpdate(port.id, (current) => ({
                  ...current,
                  description: event.target.value || undefined,
                }))
              }
              value={port.description ?? ""}
            />
          </label>
          {locked ? null : (
            <button onClick={() => onRemove(port.id)} type="button">
              {labels.removePortLabel}
            </button>
          )}
        </div>
      ))}
    </div>
  );
}

function getSuggestedPorts(
  ports: WorkflowGraphPort[],
  edge: WorkflowGraphEdge,
  direction: "inputs" | "outputs",
) {
  const datasetMatched = edge.dataset_value
    ? ports.filter((port) => port.dataset_value === edge.dataset_value)
    : [];
  if (datasetMatched.length > 0) return datasetMatched;

  const artifactMatched = edge.artifact_type
    ? ports.filter((port) => port.artifact_type === edge.artifact_type)
    : [];
  if (artifactMatched.length > 0) return artifactMatched;

  if (direction === "outputs" && edge.artifact_type) {
    const looseOutputs = ports.filter((port) => port.artifact_type === edge.artifact_type);
    if (looseOutputs.length > 0) return looseOutputs;
  }

  return ports;
}

export function WorkbenchWorkflowTopologyCard({
  labels,
  operatorDescriptors,
  selectedNodes,
  selectedEdges,
  focusedNodeId,
  focusedEdgeId,
  onAddNode,
  onAddConnectedNode,
  onSyncNodeTemplate,
  onInsertTemplateChain,
  onRemoveNode,
  onUpdateNode,
  onAddNodePort,
  onRemoveNodePort,
  onUpdateNodePort,
  onAddEdge,
  onRemoveEdge,
  onUpdateEdge,
}: WorkbenchWorkflowTopologyCardProps) {
  const [nextNodeKind, setNextNodeKind] = useState("transform");
  const [nextOperatorId, setNextOperatorId] = useState("");
  const nextKindTemplates = listWorkflowNodeTemplatePresets(nextNodeKind, operatorDescriptors);
  const nextOperatorTemplates = nextKindTemplates.filter((preset) => preset.operatorId);
  const selectedSourceNodeId = selectedNodes[0]?.id ?? null;
  const operatorDescriptorMap = new Map(
    (operatorDescriptors ?? []).map((descriptor) => [descriptor.id, descriptor] as const),
  );
  const nextOperatorDescriptor = nextOperatorId
    ? operatorDescriptorMap.get(nextOperatorId)
    : undefined;
  const sortedNextOperatorTemplates = sortWorkflowOperatorOptionPresets(
    nextOperatorTemplates,
    operatorDescriptorMap,
  );
  const getPortOptions = (
    node: WorkflowGraphNode | undefined,
    direction: "inputs" | "outputs",
  ) => node?.[direction] ?? [];

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{labels.topologyEditorTitle}</h2>
        <div className="button-row">
          <button onClick={onAddEdge} type="button">
            {labels.addEdgeLabel}
          </button>
        </div>
      </div>
      <div className="form-grid compact">
        <label>
          <span>{labels.kindLabel}</span>
          <select onChange={(event) => setNextNodeKind(event.target.value)} value={nextNodeKind}>
            {NODE_KIND_OPTIONS.map((kind) => (
              <option key={kind} value={kind}>
                {kind}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{labels.operatorLabel}</span>
          <input
            list="workflow-topology-operator-templates"
            onChange={(event) => setNextOperatorId(event.target.value)}
            value={nextOperatorId}
          />
          <datalist id="workflow-topology-operator-templates">
            {sortedNextOperatorTemplates.map((preset) => (
              <option
                key={preset.id}
                label={buildOperatorOptionLabel(
                  labels,
                  preset.label,
                  preset.operatorId ? operatorDescriptorMap.get(preset.operatorId) : undefined,
                )}
                value={preset.operatorId}
              />
            ))}
          </datalist>
        </label>
        <button
          onClick={() =>
            onAddNode({
              kind: nextNodeKind,
              operatorId: nextOperatorId || undefined,
            })
          }
          type="button"
        >
          {labels.addNodeLabel}
        </button>
      </div>
      <WorkbenchWorkflowOperatorDescriptorSummary
        descriptor={nextOperatorDescriptor}
        labels={labels}
      />
      <div className="button-row">
        <button
          onClick={() =>
            onInsertTemplateChain(
              [
                { kind: "solve", operatorId: "solve.frame_3d" },
                { kind: "extract", operatorId: "extract.result_summary" },
                { kind: "export", operatorId: "export.summary_json" },
              ],
              selectedSourceNodeId,
            )
          }
          type="button"
        >
          {labels.insertSolveExtractExportLabel}
        </button>
        <button
          onClick={() =>
            onInsertTemplateChain(
              [
                { kind: "solve", operatorId: "solve.heat_plane_quad_2d" },
                { kind: "transform", operatorId: "bridge.temperature_field_to_thermo_quad_2d" },
                { kind: "solve", operatorId: "solve.thermal_plane_quad_2d" },
              ],
              selectedSourceNodeId,
            )
          }
          type="button"
        >
          {labels.insertHeatBridgeThermoLabel}
        </button>
      </div>

      <div className="sidebar-stack">
        {selectedNodes.map((node) => {
          const operatorDescriptor = node.operator_id
            ? operatorDescriptorMap.get(node.operator_id)
            : undefined;
          const templateLocked = Boolean(operatorDescriptor);

          return (
            <section
              className="sidebar-card sidebar-card--compact"
              data-workflow-node-id={node.id}
              key={node.id}
              style={
                focusedNodeId === node.id
                  ? { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" }
                  : undefined
              }
            >
                <div className="card-head">
                  <h2>{node.id}</h2>
                  <div className="button-row">
                    <button
                      onClick={() =>
                        onAddConnectedNode(node.id, {
                          kind: nextNodeKind,
                          operatorId: nextOperatorId || undefined,
                        })
                      }
                      type="button"
                    >
                      {labels.addConnectedNodeLabel}
                    </button>
                    <button onClick={() => onRemoveNode(node.id)} type="button">
                      {labels.removeNodeLabel}
                    </button>
                  </div>
                </div>
                <div className="form-grid compact">
                  <label>
                    <span>{labels.nodeIdLabel}</span>
                    <input
                      onChange={(event) =>
                        onUpdateNode(node.id, (current) => ({
                          ...current,
                          id: event.target.value,
                        }))
                      }
                      value={node.id}
                    />
                  </label>
                  <label>
                    <span>{labels.kindLabel}</span>
                    <input
                      disabled={templateLocked}
                      onChange={(event) =>
                        onUpdateNode(node.id, (current) => ({
                          ...current,
                          kind: event.target.value,
                        }))
                      }
                      value={node.kind}
                    />
                  </label>
                  <label>
                    <span>{labels.operatorLabel}</span>
                    <input
                      list={`workflow-node-operators-${node.id}`}
                      onChange={(event) =>
                        onSyncNodeTemplate(node.id, {
                          kind: node.kind,
                          operatorId: event.target.value || undefined,
                        })
                      }
                      value={node.operator_id ?? ""}
                    />
                    <datalist id={`workflow-node-operators-${node.id}`}>
                      {sortWorkflowOperatorOptionPresets(
                        listWorkflowNodeTemplatePresets(node.kind, operatorDescriptors).filter(
                          (preset) => preset.operatorId,
                        ),
                        operatorDescriptorMap,
                      )
                        .map((preset) => (
                          <option
                            key={preset.id}
                            label={buildOperatorOptionLabel(
                              labels,
                              preset.label,
                              preset.operatorId
                                ? operatorDescriptorMap.get(preset.operatorId)
                                : undefined,
                            )}
                            value={preset.operatorId}
                          />
                        ))}
                    </datalist>
                  </label>
                </div>
                <WorkbenchWorkflowOperatorDescriptorSummary
                  descriptor={operatorDescriptor}
                  labels={labels}
                />
                <WorkbenchWorkflowPortEditor
                  direction="inputs"
                  labels={labels}
                  locked={templateLocked}
                  nodeId={node.id}
                  onAdd={() => onAddNodePort(node.id, "inputs")}
                  onRemove={(portId) => onRemoveNodePort(node.id, "inputs", portId)}
                  onUpdate={(portId, updater) => onUpdateNodePort(node.id, "inputs", portId, updater)}
                  ports={node.inputs ?? []}
                  title={labels.inputsTitle}
                />
                <WorkbenchWorkflowPortEditor
                  direction="outputs"
                  labels={labels}
                  locked={templateLocked}
                  nodeId={node.id}
                  onAdd={() => onAddNodePort(node.id, "outputs")}
                  onRemove={(portId) => onRemoveNodePort(node.id, "outputs", portId)}
                  onUpdate={(portId, updater) => onUpdateNodePort(node.id, "outputs", portId, updater)}
                  ports={node.outputs ?? []}
                  title={labels.outputsTitle}
                />
            </section>
          );
        })}
      </div>

      <div className="sidebar-stack">
        {selectedEdges.map((edge) => {
          const sourceNode = selectedNodes.find((node) => node.id === edge.from.node);
          const targetNode = selectedNodes.find((node) => node.id === edge.to.node);
          const sourcePorts = getSuggestedPorts(
            getPortOptions(sourceNode, "outputs"),
            edge,
            "outputs",
          );
          const targetPorts = getSuggestedPorts(
            getPortOptions(targetNode, "inputs"),
            edge,
            "inputs",
          );
          return (
            <section
              className="sidebar-card sidebar-card--compact"
              data-workflow-edge-id={edge.id}
              key={edge.id}
              style={
                focusedEdgeId === edge.id
                  ? { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" }
                  : undefined
              }
            >
              <div className="card-head">
                <h2>{edge.id}</h2>
                <button onClick={() => onRemoveEdge(edge.id)} type="button">
                  {labels.removeEdgeLabel}
                </button>
              </div>
              <div className="form-grid compact">
                <label>
                  <span>{labels.edgeIdLabel}</span>
                  <input
                    onChange={(event) =>
                      onUpdateEdge(edge.id, (current) => ({
                        ...current,
                        id: event.target.value,
                      }))
                    }
                    value={edge.id}
                  />
                </label>
                <label>
                  <span>{labels.fromLabel}</span>
                  <select
                    onChange={(event) =>
                      onUpdateEdge(edge.id, (current) => ({
                        ...current,
                        from: {
                          node: event.target.value,
                          port:
                            selectedNodes.find((node) => node.id === event.target.value)?.outputs?.[0]
                              ?.id ?? "",
                        },
                      }))
                    }
                    value={edge.from.node}
                  >
                    <option value="">--</option>
                    {selectedNodes.map((node) => (
                      <option key={`from:${node.id}`} value={node.id}>
                        {node.id}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>{labels.portIdLabel}</span>
                  <select
                    onChange={(event) =>
                      onUpdateEdge(edge.id, (current) => ({
                        ...current,
                        from: { ...current.from, port: event.target.value },
                      }))
                    }
                    value={edge.from.port}
                  >
                    <option value="">--</option>
                    {sourcePorts.map((port: WorkflowGraphPort) => (
                      <option key={`from:${edge.id}:${port.id}`} value={port.id}>
                        {port.id}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>{labels.toLabel}</span>
                  <select
                    onChange={(event) =>
                      onUpdateEdge(edge.id, (current) => ({
                        ...current,
                        to: {
                          node: event.target.value,
                          port:
                            selectedNodes.find((node) => node.id === event.target.value)?.inputs?.[0]
                              ?.id ?? "",
                        },
                      }))
                    }
                    value={edge.to.node}
                  >
                    <option value="">--</option>
                    {selectedNodes.map((node) => (
                      <option key={`to:${node.id}`} value={node.id}>
                        {node.id}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>{labels.portIdLabel}</span>
                  <select
                    onChange={(event) =>
                      onUpdateEdge(edge.id, (current) => ({
                        ...current,
                        to: { ...current.to, port: event.target.value },
                      }))
                    }
                    value={edge.to.port}
                  >
                    <option value="">--</option>
                    {targetPorts.map((port: WorkflowGraphPort) => (
                      <option key={`to:${edge.id}:${port.id}`} value={port.id}>
                        {port.id}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  <span>{labels.artifactTypeLabel}</span>
                  <input
                    onChange={(event) =>
                      onUpdateEdge(edge.id, (current) => ({
                        ...current,
                        artifact_type: event.target.value,
                      }))
                    }
                    value={edge.artifact_type}
                  />
                </label>
              </div>
            </section>
          );
        })}
      </div>
    </section>
  );
}
