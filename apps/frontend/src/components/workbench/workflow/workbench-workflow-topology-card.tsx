"use client";

import { useEffect, useMemo, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import type { WorkflowGraphEdge, WorkflowGraphJobResult, WorkflowGraphNode, WorkflowGraphPort, WorkflowOperatorDescriptor } from "@/lib/api";
import type { HeatPlaneStudyJobInput, PlaneStudyJobInput, StudyKind } from "@/components/workbench/workbench-types";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import { buildWorkflowBridgeRuntimeStatusMap } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-validation";
import { listWorkflowNodeTemplatePresets, type WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";
import type { WorkflowTemplateChainDefinition } from "@/components/workbench/workflow/workbench-workflow-template-chain-library";
import { sortWorkflowOperatorOptionPresets, WorkbenchWorkflowOperatorDescriptorSummary } from "@/components/workbench/workflow/workbench-workflow-operator-descriptor-summary";
import { filterWorkflowOperatorOptionPresets, WorkbenchWorkflowOperatorSearch } from "@/components/workbench/workflow/workbench-workflow-operator-search";
import { WorkbenchWorkflowTemplateChainActions } from "@/components/workbench/workflow/workbench-workflow-template-chain-actions";
import { describeWorkflowNodeTemplateSyncImpact, getWorkflowNodeTemplateSyncImpact, listAutoReconnectEdgeIds } from "@/components/workbench/workflow/workbench-workflow-template-impact";
import { WorkbenchWorkflowTopologyEdgeSection, WorkbenchWorkflowTopologyNodeSection } from "@/components/workbench/workflow/workbench-workflow-topology-sections";

type WorkbenchWorkflowTopologyCardProps = {
  labels: WorkflowSidebarLabels;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  selectedNodes: WorkflowGraphNode[];
  selectedEdges: WorkflowGraphEdge[];
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: HeatPlaneStudyJobInput;
  currentPlaneModel: PlaneStudyJobInput;
  bridgeRuntimeResult?: WorkflowGraphJobResult | null;
  highlightedEdgeIds?: string[];
  highlightedNodeIds?: string[];
  highlightedPortKeys?: string[];
  focusedNodeId?: string | null;
  focusedEdgeId?: string | null;
  onAddNode: (template?: WorkflowNodeTemplateSelection) => void;
  onAddConnectedNode: (sourceNodeId: string, template?: WorkflowNodeTemplateSelection) => void;
  onSyncNodeTemplate: (nodeId: string, template?: WorkflowNodeTemplateSelection) => void;
  onInsertTemplateChain: (chain: WorkflowTemplateChainDefinition, sourceNodeId?: string | null) => void;
  onRemoveNode: (nodeId: string) => void;
  onUpdateNode: (nodeId: string, updater: (node: WorkflowGraphNode) => WorkflowGraphNode) => void;
  onAddNodePort: (nodeId: string, direction: "inputs" | "outputs") => void;
  onRemoveNodePort: (nodeId: string, direction: "inputs" | "outputs", portId: string) => void;
  onUpdateNodePort: (nodeId: string, direction: "inputs" | "outputs", portId: string, updater: (port: WorkflowGraphPort) => WorkflowGraphPort) => void;
  onAddEdge: () => void;
  onRemoveEdge: (edgeId: string) => void;
  onUpdateEdge: (edgeId: string, updater: (edge: WorkflowGraphEdge) => WorkflowGraphEdge) => void;
  setSystemAlerts: Dispatch<SetStateAction<WorkbenchAlertItem[]>>;
};

const NODE_KIND_OPTIONS = ["input", "solve", "transform", "extract", "export", "output", "condition"];
type WorkflowTopologyView = "nodes" | "edges" | "add" | "templates";

function getSuggestedPorts(ports: WorkflowGraphPort[], edge: WorkflowGraphEdge, direction: "inputs" | "outputs") {
  const datasetMatched = edge.dataset_value ? ports.filter((port) => port.dataset_value === edge.dataset_value) : [];
  if (datasetMatched.length > 0) return datasetMatched;
  const artifactMatched = edge.artifact_type ? ports.filter((port) => port.artifact_type === edge.artifact_type) : [];
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
  currentStudyKind,
  currentHeatPlaneModel,
  currentPlaneModel,
  bridgeRuntimeResult,
  highlightedEdgeIds = [],
  highlightedNodeIds = [],
  highlightedPortKeys = [],
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
  setSystemAlerts,
}: WorkbenchWorkflowTopologyCardProps) {
  const [topologyView, setTopologyView] = useState<WorkflowTopologyView>("nodes");
  const [nextNodeKind, setNextNodeKind] = useState("transform");
  const [nextOperatorId, setNextOperatorId] = useState("");
  const [nextOperatorSearchQuery, setNextOperatorSearchQuery] = useState("");
  const [nextOperatorDomainFilter, setNextOperatorDomainFilter] = useState("");
  const [nextOperatorValidationFilter, setNextOperatorValidationFilter] = useState("");
  const [nextOperatorCapabilityFilter, setNextOperatorCapabilityFilter] = useState("");
  const [activeNodeId, setActiveNodeId] = useState(selectedNodes[0]?.id ?? "");
  const [activeEdgeId, setActiveEdgeId] = useState(selectedEdges[0]?.id ?? "");
  const [localHighlightedEdgeIds, setLocalHighlightedEdgeIds] = useState<string[]>([]);
  const operatorDescriptorMap = useMemo(() => new Map((operatorDescriptors ?? []).map((descriptor) => [descriptor.id, descriptor] as const)), [operatorDescriptors]);
  const selectedNodeMap = useMemo(() => new Map(selectedNodes.map((node) => [node.id, node] as const)), [selectedNodes]);
  const nextKindTemplates = useMemo(() => listWorkflowNodeTemplatePresets(nextNodeKind, operatorDescriptors), [nextNodeKind, operatorDescriptors]);
  const nextOperatorTemplates = nextKindTemplates.filter((preset) => preset.operatorId);
  const nodeSelectOptions = useMemo(() => selectedNodes.map((node) => ({ id: node.id })), [selectedNodes]);
  const nodeOperatorPresetMap = useMemo(() => {
    const byKind = new Map<string, ReturnType<typeof sortWorkflowOperatorOptionPresets>>();
    for (const kind of new Set(selectedNodes.map((node) => node.kind))) {
      byKind.set(kind, sortWorkflowOperatorOptionPresets(listWorkflowNodeTemplatePresets(kind, operatorDescriptors).filter((preset) => preset.operatorId), operatorDescriptorMap));
    }
    return byKind;
  }, [operatorDescriptorMap, operatorDescriptors, selectedNodes]);
  const bridgeRuntimeStatusMap = useMemo(() => buildWorkflowBridgeRuntimeStatusMap({ nodes: selectedNodes, edges: selectedEdges }, bridgeRuntimeResult), [bridgeRuntimeResult, selectedEdges, selectedNodes]);
  const controlFlowEdgesByNode = useMemo(() => {
    const byNode = new Map<string, WorkflowGraphEdge[]>();
    for (const node of selectedNodes) byNode.set(node.id, []);
    for (const edge of selectedEdges) {
      if (byNode.has(edge.from.node)) byNode.get(edge.from.node)?.push(edge);
      if (edge.to.node !== edge.from.node && byNode.has(edge.to.node)) byNode.get(edge.to.node)?.push(edge);
    }
    return byNode;
  }, [selectedEdges, selectedNodes]);
  const bridgePeerNodesByNode = useMemo(() => {
    const byNode = new Map<string, WorkflowGraphNode[]>();
    for (const node of selectedNodes) {
      const outputTypes = new Set((node.outputs ?? []).map((port) => port.artifact_type));
      byNode.set(node.id, selectedNodes.filter((entry) => (entry.inputs ?? []).some((port) => outputTypes.has(port.artifact_type))));
    }
    return byNode;
  }, [selectedNodes]);
  const availableDomains = useMemo(() => [...new Set(nextOperatorTemplates.map((preset) => {
    const descriptor = preset.operatorId ? operatorDescriptorMap.get(preset.operatorId) : undefined;
    return descriptor?.domain;
  }).filter(Boolean))] as string[], [nextOperatorTemplates, operatorDescriptorMap]);
  const availableCapabilities = useMemo(() => [...new Set(nextOperatorTemplates.flatMap((preset) => {
    const descriptor = preset.operatorId ? operatorDescriptorMap.get(preset.operatorId) : undefined;
    return descriptor?.capability_tags ?? [];
  }))], [nextOperatorTemplates, operatorDescriptorMap]);
  const nextOperatorDescriptor = nextOperatorId ? operatorDescriptorMap.get(nextOperatorId) : undefined;
  const sortedNextOperatorTemplates = useMemo(() => filterWorkflowOperatorOptionPresets(nextOperatorTemplates, operatorDescriptorMap, nextOperatorSearchQuery, { domain: nextOperatorDomainFilter, validation: nextOperatorValidationFilter, capability: nextOperatorCapabilityFilter }), [nextOperatorTemplates, operatorDescriptorMap, nextOperatorSearchQuery, nextOperatorDomainFilter, nextOperatorValidationFilter, nextOperatorCapabilityFilter]);
  const edgeViewModels = useMemo(
    () =>
      selectedEdges.map((edge) => {
        const sourceNode = selectedNodeMap.get(edge.from.node);
        const targetNode = selectedNodeMap.get(edge.to.node);
        return {
          edge,
          isFocused: focusedEdgeId === edge.id,
          isHighlighted: highlightedEdgeIds.includes(edge.id),
          isLocallyHighlighted: localHighlightedEdgeIds.includes(edge.id),
          sourcePorts: getSuggestedPorts(sourceNode?.outputs ?? [], edge, "outputs"),
          targetPorts: getSuggestedPorts(targetNode?.inputs ?? [], edge, "inputs"),
        };
      }),
    [focusedEdgeId, highlightedEdgeIds, localHighlightedEdgeIds, selectedEdges, selectedNodeMap],
  );
  const activeNode = selectedNodeMap.get(activeNodeId) ?? selectedNodes[0] ?? null;
  const activeEdgeView = edgeViewModels.find(({ edge }) => edge.id === activeEdgeId) ?? edgeViewModels[0] ?? null;

  useEffect(() => {
    if (focusedNodeId && selectedNodeMap.has(focusedNodeId)) {
      setActiveNodeId(focusedNodeId);
      setTopologyView("nodes");
    }
  }, [focusedNodeId, selectedNodeMap]);
  useEffect(() => {
    if (focusedEdgeId && edgeViewModels.some(({ edge }) => edge.id === focusedEdgeId)) {
      setActiveEdgeId(focusedEdgeId);
      setTopologyView("edges");
    }
  }, [edgeViewModels, focusedEdgeId]);
  useEffect(() => {
    if (!selectedNodeMap.has(activeNodeId)) setActiveNodeId(selectedNodes[0]?.id ?? "");
  }, [activeNodeId, selectedNodeMap, selectedNodes]);
  useEffect(() => {
    if (!edgeViewModels.some(({ edge }) => edge.id === activeEdgeId)) setActiveEdgeId(edgeViewModels[0]?.edge.id ?? "");
  }, [activeEdgeId, edgeViewModels]);
  const confirmNodeTemplateSync = (node: WorkflowGraphNode, operatorId?: string) => {
    const impact = getWorkflowNodeTemplateSyncImpact({ nodes: selectedNodes, edges: selectedEdges }, node.id, { kind: node.kind, operatorId }, operatorDescriptors ?? []);
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
    <section className="sidebar-card sidebar-card--compact workflow-topology-card" data-workflow-topology="editor" data-workflow-topology-view={topologyView}>
      <div className="card-head">
        <h2>{labels.topologyEditorTitle}</h2>
      </div>
      <nav aria-label={labels.topologyEditorTitle} className="workflow-topology-view-tabs">
        {([
          ["nodes", labels.nodesTitle, selectedNodes.length],
          ["edges", labels.edgesTitle, selectedEdges.length],
          ["add", labels.addNodeLabel, nextOperatorTemplates.length],
          ["templates", labels.templateChainLibraryLabel, null],
        ] as Array<[WorkflowTopologyView, string, number | null]>).map(([view, label, count]) => (
          <button
            aria-pressed={topologyView === view}
            className={topologyView === view ? "workflow-topology-view-tab workflow-topology-view-tab--active" : "workflow-topology-view-tab"}
            data-workflow-topology-view-count={count}
            data-workflow-topology-view-target={view}
            key={view}
            onClick={() => setTopologyView(view)}
            type="button"
          >
            <span>{label}</span>
            {count === null ? null : <strong>{count}</strong>}
          </button>
        ))}
      </nav>
      {topologyView === "add" ? (
        <>
          <div className="form-grid compact workflow-topology-toolbar" data-workflow-topology-toolbar="controls">
            <label>
              <span>{labels.kindLabel}</span>
              <select
                data-workflow-topology-kind="select"
                onChange={(event) => setNextNodeKind(event.target.value)}
                value={nextNodeKind}
              >
                {NODE_KIND_OPTIONS.map((kind) => <option key={kind} value={kind}>{kind}</option>)}
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
              onQuickInsert={(operatorId) => { setNextOperatorId(operatorId); onAddNode({ kind: nextNodeKind, operatorId }); }}
              onValidationFilterChange={setNextOperatorValidationFilter}
              query={nextOperatorSearchQuery}
              selectedSourceNode={activeNode}
              setSystemAlerts={setSystemAlerts}
              validationFilter={nextOperatorValidationFilter}
            />
            <button data-workflow-topology-action="add-node" onClick={() => onAddNode({ kind: nextNodeKind, operatorId: nextOperatorId || undefined })} type="button">{labels.addNodeLabel}</button>
          </div>
          <WorkbenchWorkflowOperatorDescriptorSummary descriptor={nextOperatorDescriptor} labels={labels} />
        </>
      ) : null}
      {topologyView === "templates" ? <WorkbenchWorkflowTemplateChainActions labels={labels} onInsertTemplateChain={onInsertTemplateChain} selectedSourceNodeId={activeNode?.id ?? null} selectedNodes={selectedNodes} setSystemAlerts={setSystemAlerts} /> : null}
      {topologyView === "nodes" ? (
        <div className="sidebar-stack workflow-topology-node-stack" data-workflow-topology-stack="nodes">
          <label className="workflow-topology-selection">
            <span>{labels.nodesTitle}</span>
            <select data-workflow-topology-node-select="active" onChange={(event) => setActiveNodeId(event.target.value)} value={activeNode?.id ?? ""}>
              {selectedNodes.map((node) => <option key={node.id} value={node.id}>{node.id}</option>)}
            </select>
          </label>
          {activeNode ? (
          <WorkbenchWorkflowTopologyNodeSection
            bridgePeerNodes={bridgePeerNodesByNode.get(activeNode.id) ?? []}
            controlFlowEdges={controlFlowEdgesByNode.get(activeNode.id) ?? []}
            currentHeatPlaneModel={currentHeatPlaneModel}
            currentPlaneModel={currentPlaneModel}
            currentStudyKind={currentStudyKind}
            bridgeRuntimeStatus={bridgeRuntimeStatusMap.get(activeNode.id)}
            isFocused={focusedNodeId === activeNode.id}
            isHighlighted={highlightedNodeIds.includes(activeNode.id)}
            highlightedPortKeys={highlightedPortKeys}
            labels={labels}
            nextNodeKind={nextNodeKind}
            nextOperatorId={nextOperatorId}
            node={activeNode}
            nodeOperatorPresets={nodeOperatorPresetMap.get(activeNode.kind) ?? []}
            onAddConnectedNode={onAddConnectedNode}
            onAddNodePort={onAddNodePort}
            onConfirmNodeTemplateSync={confirmNodeTemplateSync}
            onRemoveNode={onRemoveNode}
            onRemoveNodePort={onRemoveNodePort}
            onSyncNodeTemplate={onSyncNodeTemplate}
            onUpdateNode={onUpdateNode}
            onUpdateNodePort={onUpdateNodePort}
            operatorDescriptor={activeNode.operator_id ? operatorDescriptorMap.get(activeNode.operator_id) : undefined}
            operatorDescriptorMap={operatorDescriptorMap}
          />
          ) : <p className="card-copy">{labels.noSelectionLabel}</p>}
        </div>
      ) : null}
      {topologyView === "edges" ? (
        <div className="sidebar-stack workflow-topology-edge-stack" data-workflow-topology-stack="edges">
          <div className="button-row">
            <button data-workflow-topology-action="add-edge" onClick={onAddEdge} type="button">{labels.addEdgeLabel}</button>
          </div>
          <label className="workflow-topology-selection">
            <span>{labels.edgesTitle}</span>
            <select data-workflow-topology-edge-select="active" onChange={(event) => setActiveEdgeId(event.target.value)} value={activeEdgeView?.edge.id ?? ""}>
              {edgeViewModels.map(({ edge }) => <option key={edge.id} value={edge.id}>{edge.id}</option>)}
            </select>
          </label>
          {activeEdgeView ? (
          <WorkbenchWorkflowTopologyEdgeSection
            edge={activeEdgeView.edge}
            isFocused={activeEdgeView.isFocused}
            isHighlighted={activeEdgeView.isHighlighted}
            isLocallyHighlighted={activeEdgeView.isLocallyHighlighted}
            labels={labels}
            nodeSelectOptions={nodeSelectOptions}
            onRemoveEdge={onRemoveEdge}
            onUpdateEdge={onUpdateEdge}
            selectedNodeMap={selectedNodeMap}
            sourcePorts={activeEdgeView.sourcePorts}
            targetPorts={activeEdgeView.targetPorts}
          />
          ) : <p className="card-copy">{labels.noSelectionLabel}</p>}
        </div>
      ) : null}
    </section>
  );
}
