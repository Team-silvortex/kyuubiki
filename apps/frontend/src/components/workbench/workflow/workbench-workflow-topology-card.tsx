"use client";
import { useState } from "react";
import type { WorkflowGraphEdge, WorkflowGraphNode, WorkflowOperatorDescriptor, WorkflowGraphPort } from "@/lib/api";
import type { HeatPlaneStudyJobInput, PlaneStudyJobInput, StudyKind } from "@/components/workbench/workbench-types";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import {
  listWorkflowNodeTemplatePresets,
  type WorkflowNodeTemplateSelection,
} from "@/components/workbench/workflow/workbench-workflow-node-templates";
import {
  buildOperatorOptionLabel,
  sortWorkflowOperatorOptionPresets,
  WorkbenchWorkflowOperatorDescriptorSummary,
} from "@/components/workbench/workflow/workbench-workflow-operator-descriptor-summary";
import {
  filterWorkflowOperatorOptionPresets,
  WorkbenchWorkflowOperatorSearch,
} from "@/components/workbench/workflow/workbench-workflow-operator-search";
import { WorkbenchWorkflowBridgeContractEditor } from "@/components/workbench/workflow/workbench-workflow-bridge-contract-editor";
import { WorkbenchWorkflowConditionEditor } from "@/components/workbench/workflow/workbench-workflow-condition-editor";
import { WorkbenchWorkflowControlFlowHint } from "@/components/workbench/workflow/workbench-workflow-control-flow-hint";
import { WorkbenchWorkflowTemplateChainActions } from "@/components/workbench/workflow/workbench-workflow-template-chain-actions";
import {
  describeWorkflowNodeTemplateSyncImpact,
  getWorkflowNodeTemplateSyncImpact,
  listAutoReconnectEdgeIds,
} from "@/components/workbench/workflow/workbench-workflow-template-impact";
type WorkbenchWorkflowTopologyCardProps = {
  labels: WorkflowSidebarLabels;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  selectedNodes: WorkflowGraphNode[];
  selectedEdges: WorkflowGraphEdge[];
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: HeatPlaneStudyJobInput;
  currentPlaneModel: PlaneStudyJobInput;
  highlightedEdgeIds?: string[];
  highlightedNodeIds?: string[];
  focusedNodeId?: string | null;
  focusedEdgeId?: string | null;
  onAddNode: (template?: WorkflowNodeTemplateSelection) => void;
  onAddConnectedNode: (sourceNodeId: string, template?: WorkflowNodeTemplateSelection) => void;
  onSyncNodeTemplate: (nodeId: string, template?: WorkflowNodeTemplateSelection) => void;
  onInsertTemplateChain: (templates: WorkflowNodeTemplateSelection[], sourceNodeId?: string | null) => void;
  onRemoveNode: (nodeId: string) => void;
  onUpdateNode: (nodeId: string, updater: (node: WorkflowGraphNode) => WorkflowGraphNode) => void;
  onAddNodePort: (nodeId: string, direction: "inputs" | "outputs") => void;
  onRemoveNodePort: (nodeId: string, direction: "inputs" | "outputs", portId: string) => void;
  onUpdateNodePort: (nodeId: string, direction: "inputs" | "outputs", portId: string, updater: (port: WorkflowGraphPort) => WorkflowGraphPort) => void;
  onAddEdge: () => void;
  onRemoveEdge: (edgeId: string) => void;
  onUpdateEdge: (edgeId: string, updater: (edge: WorkflowGraphEdge) => WorkflowGraphEdge) => void;
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
function buildEdgeHighlightStyle(
  edgeId: string,
  focusedEdgeId: string | null | undefined,
  highlightedEdgeIds: string[],
  localHighlightedEdgeIds: string[],
) {
  const highlighted = highlightedEdgeIds.includes(edgeId) || localHighlightedEdgeIds.includes(edgeId);
  if (!highlighted && focusedEdgeId !== edgeId) return undefined;
  return {
    outline: highlighted ? "2px solid rgba(34, 197, 94, 0.9)" : "2px solid var(--accent, #4f46e5)",
    outlineOffset: "2px",
    boxShadow: highlighted ? "0 0 0 1px rgba(34, 197, 94, 0.22), 0 0 18px rgba(34, 197, 94, 0.18)" : undefined,
  };
}
function buildNodeHighlightStyle(nodeId: string, focusedNodeId: string | null | undefined, highlightedNodeIds: string[]) {
  const highlighted = highlightedNodeIds.includes(nodeId);
  if (!highlighted && focusedNodeId !== nodeId) return undefined;
  return highlighted
    ? { outline: "2px solid rgba(34, 197, 94, 0.9)", outlineOffset: "2px", boxShadow: "0 0 0 1px rgba(34, 197, 94, 0.22), 0 0 18px rgba(34, 197, 94, 0.18)" }
    : { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" };
}
export function WorkbenchWorkflowTopologyCard({
  labels,
  operatorDescriptors,
  selectedNodes,
  selectedEdges,
  currentStudyKind,
  currentHeatPlaneModel,
  currentPlaneModel,
  highlightedEdgeIds = [],
  highlightedNodeIds = [],
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
  const [nextOperatorSearchQuery, setNextOperatorSearchQuery] = useState("");
  const [nextOperatorDomainFilter, setNextOperatorDomainFilter] = useState("");
  const [nextOperatorValidationFilter, setNextOperatorValidationFilter] = useState("");
  const [nextOperatorCapabilityFilter, setNextOperatorCapabilityFilter] = useState("");
  const [localHighlightedEdgeIds, setLocalHighlightedEdgeIds] = useState<string[]>([]);
  const nextKindTemplates = listWorkflowNodeTemplatePresets(nextNodeKind, operatorDescriptors);
  const nextOperatorTemplates = nextKindTemplates.filter((preset) => preset.operatorId);
  const selectedSourceNodeId = selectedNodes[0]?.id ?? null;
  const operatorDescriptorMap = new Map(
    (operatorDescriptors ?? []).map((descriptor) => [descriptor.id, descriptor] as const),
  );
  const availableDomains = [...new Set(nextOperatorTemplates.map((preset) => {
    const descriptor = preset.operatorId ? operatorDescriptorMap.get(preset.operatorId) : undefined;
    return descriptor?.domain;
  }).filter(Boolean))] as string[];
  const availableCapabilities = [...new Set(nextOperatorTemplates.flatMap((preset) => {
    const descriptor = preset.operatorId ? operatorDescriptorMap.get(preset.operatorId) : undefined;
    return descriptor?.capability_tags ?? [];
  }))];
  const nextOperatorDescriptor = nextOperatorId
    ? operatorDescriptorMap.get(nextOperatorId)
    : undefined;
  const sortedNextOperatorTemplates = filterWorkflowOperatorOptionPresets(
    nextOperatorTemplates,
    operatorDescriptorMap,
    nextOperatorSearchQuery,
    {
      domain: nextOperatorDomainFilter,
      validation: nextOperatorValidationFilter,
      capability: nextOperatorCapabilityFilter,
    },
  );
  const getPortOptions = (
    node: WorkflowGraphNode | undefined,
    direction: "inputs" | "outputs",
  ) => node?.[direction] ?? [];
  const confirmNodeTemplateSync = (node: WorkflowGraphNode, operatorId?: string) => {
    const impact = getWorkflowNodeTemplateSyncImpact(
      { nodes: selectedNodes, edges: selectedEdges },
      node.id,
      { kind: node.kind, operatorId },
      operatorDescriptors ?? [],
    );
    const preview = describeWorkflowNodeTemplateSyncImpact(impact);
    const accepted = preview ? window.confirm(preview) : true;
    if (accepted) {
      const edgeIds = listAutoReconnectEdgeIds(impact);
      if (edgeIds.length > 0) {
        setLocalHighlightedEdgeIds(edgeIds);
        window.setTimeout(() => setLocalHighlightedEdgeIds([]), 2200);
      }
    }
    return accepted;
  };

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
        <WorkbenchWorkflowOperatorSearch
          availableCapabilities={availableCapabilities}
          availableDomains={availableDomains}
          capabilityFilter={nextOperatorCapabilityFilter}
          domainFilter={nextOperatorDomainFilter}
          filteredPresets={sortedNextOperatorTemplates}
          labels={labels}
          operatorDescriptorMap={operatorDescriptorMap}
          operatorId={nextOperatorId}
          onCapabilityFilterChange={setNextOperatorCapabilityFilter}
          onDomainFilterChange={setNextOperatorDomainFilter}
          onOperatorIdChange={setNextOperatorId}
          onQueryChange={setNextOperatorSearchQuery}
          onQuickInsert={(operatorId) => {
            setNextOperatorId(operatorId);
            onAddNode({ kind: nextNodeKind, operatorId });
          }}
          onValidationFilterChange={setNextOperatorValidationFilter}
          query={nextOperatorSearchQuery}
          validationFilter={nextOperatorValidationFilter}
        />
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
      <WorkbenchWorkflowTemplateChainActions
        labels={labels}
        onInsertTemplateChain={onInsertTemplateChain}
        selectedSourceNodeId={selectedSourceNodeId}
        selectedNodes={selectedNodes}
      />
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
              style={buildNodeHighlightStyle(node.id, focusedNodeId, highlightedNodeIds)}
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
                        confirmNodeTemplateSync(node, event.target.value || undefined)
                          ? onSyncNodeTemplate(node.id, {
                              kind: node.kind,
                              operatorId: event.target.value || undefined,
                            })
                          : undefined
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
                <WorkbenchWorkflowControlFlowHint node={node} selectedEdges={selectedEdges} />
                <WorkbenchWorkflowBridgeContractEditor
                  currentHeatPlaneModel={currentHeatPlaneModel as unknown as Record<string, unknown>}
                  currentPlaneModel={currentPlaneModel as unknown as Record<string, unknown>}
                  currentStudyKind={currentStudyKind}
                  labels={labels}
                  node={node}
                  selectedNodes={selectedNodes}
                  onUpdateNode={onUpdateNode}
                />
                <WorkbenchWorkflowConditionEditor labels={labels} node={node} onUpdateNode={onUpdateNode} />
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
              style={buildEdgeHighlightStyle(edge.id, focusedEdgeId, highlightedEdgeIds, localHighlightedEdgeIds)}
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
