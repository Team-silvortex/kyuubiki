"use client";

import { useMemo, useState } from "react";
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
};

const NODE_KIND_OPTIONS = ["input", "solve", "transform", "extract", "export", "output", "condition"];

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
}: WorkbenchWorkflowTopologyCardProps) {
  const [nextNodeKind, setNextNodeKind] = useState("transform");
  const [nextOperatorId, setNextOperatorId] = useState("");
  const [nextOperatorSearchQuery, setNextOperatorSearchQuery] = useState("");
  const [nextOperatorDomainFilter, setNextOperatorDomainFilter] = useState("");
  const [nextOperatorValidationFilter, setNextOperatorValidationFilter] = useState("");
  const [nextOperatorCapabilityFilter, setNextOperatorCapabilityFilter] = useState("");
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
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{labels.topologyEditorTitle}</h2>
        <div className="button-row">
          <button onClick={onAddEdge} type="button">{labels.addEdgeLabel}</button>
        </div>
      </div>
      <div className="form-grid compact">
        <label>
          <span>{labels.kindLabel}</span>
          <select onChange={(event) => setNextNodeKind(event.target.value)} value={nextNodeKind}>
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
          selectedSourceNode={selectedNodes[0] ?? null}
          validationFilter={nextOperatorValidationFilter}
        />
        <button onClick={() => onAddNode({ kind: nextNodeKind, operatorId: nextOperatorId || undefined })} type="button">{labels.addNodeLabel}</button>
      </div>
      <WorkbenchWorkflowOperatorDescriptorSummary descriptor={nextOperatorDescriptor} labels={labels} />
      <WorkbenchWorkflowTemplateChainActions labels={labels} onInsertTemplateChain={onInsertTemplateChain} selectedSourceNodeId={selectedNodes[0]?.id ?? null} selectedNodes={selectedNodes} />
      <div className="sidebar-stack">
        {selectedNodes.map((node) => (
          <WorkbenchWorkflowTopologyNodeSection
            bridgePeerNodes={bridgePeerNodesByNode.get(node.id) ?? []}
            controlFlowEdges={controlFlowEdgesByNode.get(node.id) ?? []}
            currentHeatPlaneModel={currentHeatPlaneModel}
            currentPlaneModel={currentPlaneModel}
            currentStudyKind={currentStudyKind}
            bridgeRuntimeStatus={bridgeRuntimeStatusMap.get(node.id)}
            isFocused={focusedNodeId === node.id}
            isHighlighted={highlightedNodeIds.includes(node.id)}
            highlightedPortKeys={highlightedPortKeys}
            key={node.id}
            labels={labels}
            nextNodeKind={nextNodeKind}
            nextOperatorId={nextOperatorId}
            node={node}
            nodeOperatorPresets={nodeOperatorPresetMap.get(node.kind) ?? []}
            onAddConnectedNode={onAddConnectedNode}
            onAddNodePort={onAddNodePort}
            onConfirmNodeTemplateSync={confirmNodeTemplateSync}
            onRemoveNode={onRemoveNode}
            onRemoveNodePort={onRemoveNodePort}
            onSyncNodeTemplate={onSyncNodeTemplate}
            onUpdateNode={onUpdateNode}
            onUpdateNodePort={onUpdateNodePort}
            operatorDescriptor={node.operator_id ? operatorDescriptorMap.get(node.operator_id) : undefined}
            operatorDescriptorMap={operatorDescriptorMap}
          />
        ))}
      </div>
      <div className="sidebar-stack">
        {edgeViewModels.map(({ edge, isFocused, isHighlighted, isLocallyHighlighted, sourcePorts, targetPorts }) => (
          <WorkbenchWorkflowTopologyEdgeSection
            edge={edge}
            isFocused={isFocused}
            isHighlighted={isHighlighted}
            isLocallyHighlighted={isLocallyHighlighted}
            key={edge.id}
            labels={labels}
            nodeSelectOptions={nodeSelectOptions}
            onRemoveEdge={onRemoveEdge}
            onUpdateEdge={onUpdateEdge}
            selectedNodeMap={selectedNodeMap}
            sourcePorts={sourcePorts}
            targetPorts={targetPorts}
          />
        ))}
      </div>
    </section>
  );
}
