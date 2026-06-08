"use client";
import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import type {
  WorkflowCatalogEntryArtifact,
  WorkflowDatasetAxis,
  WorkflowCatalogEntry,
  WorkflowDatasetValueInfo,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import {
  asWorkflowDatasetContract,
  asWorkflowGraphDefinition,
  mergeDatasetContractIntoGraph,
  readJsonFile,
} from "@/components/workbench/workflow/workbench-workflow-builder-import";
import {
  listStoredWorkflowDrafts,
  removeStoredWorkflowDraft,
  saveStoredWorkflowDraft,
  type StoredWorkflowDraft,
} from "@/components/workbench/workflow/workbench-workflow-draft-storage";
import {
  duplicateStoredLocalWorkflow,
  removeStoredLocalWorkflow,
  renameStoredLocalWorkflow,
  saveStoredLocalWorkflow,
  updateStoredLocalWorkflowMetadata,
} from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { WorkbenchWorkflowDraftCard } from "@/components/workbench/workflow/workbench-workflow-draft-card";
import { WorkbenchWorkflowBuilderToolbar } from "@/components/workbench/workflow/workbench-workflow-builder-toolbar";
import {
  buildWorkflowInputArtifactTexts,
  parseWorkflowInputArtifactTexts,
} from "@/components/workbench/workflow/workbench-workflow-input-artifacts";
import { WorkbenchWorkflowInputArtifactsCard } from "@/components/workbench/workflow/workbench-workflow-input-artifacts-card";
import { WorkbenchWorkflowArtifactCard } from "@/components/workbench/workflow/workbench-workflow-artifact-card";
import {
  buildDraftArtifact,
  buildDraftDatasetValue,
  cloneWorkflowGraph,
  downloadJsonArtifact,
  ensureDatasetContract,
  slugifyWorkflowAssetName,
} from "@/components/workbench/workflow/workbench-workflow-builder-utils";
import { createWorkflowTopologyActions } from "@/components/workbench/workflow/workbench-workflow-topology-actions";
import { validateWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import { WorkbenchWorkflowDatasetCard } from "@/components/workbench/workflow/workbench-workflow-dataset-card";
import { WorkbenchWorkflowGraphSummaryCard } from "@/components/workbench/workflow/workbench-workflow-graph-summary-card";
import { WorkbenchWorkflowLocalMetadataCard } from "@/components/workbench/workflow/workbench-workflow-local-metadata-card";
import { builtInWorkflowSampleInputArtifacts } from "@/components/workbench/workflow/workbench-workflow-sample-inputs";
import { WorkbenchWorkflowTopologyCard } from "@/components/workbench/workflow/workbench-workflow-topology-card";
import { WorkbenchWorkflowValidationCard } from "@/components/workbench/workflow/workbench-workflow-validation-card";

type WorkbenchWorkflowBuilderCardProps = {
  labels: WorkflowSidebarLabels;
  selectedWorkflow: WorkflowCatalogEntry | null;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  onRefreshWorkflowCatalog: () => void;
  onRunWorkflowCatalog: (workflowId: string) => void;
  onRunWorkflowDraft: (
    workflowId: string,
    graph: WorkflowGraphDefinition,
    inputArtifacts: Record<string, unknown>,
  ) => void;
};

export function WorkbenchWorkflowBuilderCard({
  labels,
  selectedWorkflow,
  operatorDescriptors,
  onRefreshWorkflowCatalog,
  onRunWorkflowCatalog,
  onRunWorkflowDraft,
}: WorkbenchWorkflowBuilderCardProps) {
  const [draftGraph, setDraftGraph] = useState<WorkflowGraphDefinition | null>(null);
  const [draftInputTexts, setDraftInputTexts] = useState<Record<string, string>>({});
  const [selectedDatasetValueId, setSelectedDatasetValueId] = useState<string | null>(null);
  const [importMessage, setImportMessage] = useState<string | null>(null);
  const [savedDrafts, setSavedDrafts] = useState<StoredWorkflowDraft[]>([]);
  const [focusedNodeId, setFocusedNodeId] = useState<string | null>(null);
  const [focusedEdgeId, setFocusedEdgeId] = useState<string | null>(null);
  const [focusedArtifactKey, setFocusedArtifactKey] = useState<string | null>(null);
  const [focusedDatasetValueId, setFocusedDatasetValueId] = useState<string | null>(null);
  const [highlightDatasetEditor, setHighlightDatasetEditor] = useState(false);
  const graphInputRef = useRef<HTMLInputElement | null>(null);
  const datasetInputRef = useRef<HTMLInputElement | null>(null);
  const builderRootRef = useRef<HTMLElement | null>(null);

  function resetBuilderFocus() {
    setFocusedNodeId(null);
    setFocusedEdgeId(null);
    setFocusedArtifactKey(null);
    setFocusedDatasetValueId(null);
    setHighlightDatasetEditor(false);
  }

  useEffect(() => {
    const nextDraft = cloneWorkflowGraph(selectedWorkflow?.graph ?? null);
    if (nextDraft) {
      nextDraft.entry_inputs = selectedWorkflow?.entry_inputs
        ? [...selectedWorkflow.entry_inputs]
        : nextDraft.entry_inputs ?? [];
      nextDraft.output_artifacts = selectedWorkflow?.output_artifacts
        ? [...selectedWorkflow.output_artifacts]
        : nextDraft.output_artifacts ?? [];
    }
    setDraftGraph(nextDraft);
    setDraftInputTexts(
      selectedWorkflow
        ? selectedWorkflow.local?.input_artifact_texts ??
          buildWorkflowInputArtifactTexts(
            nextDraft?.entry_inputs ?? [],
            builtInWorkflowSampleInputArtifacts(selectedWorkflow.local?.source_workflow_id ?? selectedWorkflow.id),
          )
        : {},
    );
    setSelectedDatasetValueId(nextDraft?.dataset_contract?.values?.[0]?.id ?? null);
    setImportMessage(null);
    resetBuilderFocus();
    setSavedDrafts(selectedWorkflow ? listStoredWorkflowDrafts(selectedWorkflow.id) : []);
  }, [selectedWorkflow]);

  const selectedGraph = draftGraph;
  const selectedNodes = selectedGraph?.nodes ?? [];
  const selectedEdges = selectedGraph?.edges ?? [];
  const selectedEntryInputs = selectedGraph?.entry_inputs ?? [];
  const selectedOutputArtifacts = selectedGraph?.output_artifacts ?? [];
  const selectedDatasetContract = selectedGraph?.dataset_contract ?? null;
  const selectedDatasetValues = selectedDatasetContract?.values ?? [];
  const parsedDraftInputs = useMemo(() => parseWorkflowInputArtifactTexts(draftInputTexts), [draftInputTexts]);
  const validationIssues = useMemo(
    () => validateWorkflowGraphDefinition(selectedGraph, selectedEntryInputs, selectedOutputArtifacts),
    [selectedGraph, selectedEntryInputs, selectedOutputArtifacts],
  );
  const selectedDatasetValue = useMemo(
    () => selectedDatasetValues.find((value) => value.id === selectedDatasetValueId) ?? selectedDatasetValues[0] ?? null,
    [selectedDatasetValueId, selectedDatasetValues],
  );
  const topologyActions = createWorkflowTopologyActions(setDraftGraph, operatorDescriptors);
  function updateDatasetValue(
    valueId: string,
    updater: (value: WorkflowDatasetValueInfo) => WorkflowDatasetValueInfo,
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      const contract = ensureDatasetContract(next);
      if (!contract) return current;
      contract.values = contract.values.map((value) => (value.id === valueId ? updater(value) : value));
      return next;
    });
  }

  function updateNodePortDatasetValue(
    nodeId: string,
    portId: string,
    direction: "inputs" | "outputs",
    datasetValue: string,
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      const node = next?.nodes.find((entry) => entry.id === nodeId);
      const ports = node?.[direction];
      const port = ports?.find((entry) => entry.id === portId);
      if (port) {
        port.dataset_value = datasetValue || undefined;
      }
      return next;
    });
  }
  function updateEdgeDatasetValue(edgeId: string, datasetValue: string) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      const edge = next?.edges?.find((entry) => entry.id === edgeId);
      if (edge) {
        edge.dataset_value = datasetValue || undefined;
      }
      return next;
    });
  }
  function updateArtifacts(
    field: "entry_inputs" | "output_artifacts",
    updater: (artifacts: WorkflowCatalogEntryArtifact[]) => WorkflowCatalogEntryArtifact[],
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      next[field] = updater([...(next[field] ?? [])]);
      return next;
    });
  }

  function addArtifact(field: "entry_inputs" | "output_artifacts") {
    const nextIndex = ((selectedGraph?.[field] ?? []).length || 0) + 1;
    updateArtifacts(field, (artifacts) => [...artifacts, buildDraftArtifact(nextIndex)]);
  }

  function removeArtifact(field: "entry_inputs" | "output_artifacts", index: number) {
    updateArtifacts(field, (artifacts) => artifacts.filter((_, artifactIndex) => artifactIndex !== index));
  }
  function updateArtifact(
    field: "entry_inputs" | "output_artifacts",
    index: number,
    updater: (artifact: WorkflowCatalogEntryArtifact) => WorkflowCatalogEntryArtifact,
  ) {
    updateArtifacts(field, (artifacts) => artifacts.map((artifact, artifactIndex) => (artifactIndex === index ? updater(artifact) : artifact)));
  }

  function applyValidationFix(issueId: string) {
    const issue = validationIssues.find((entry) => entry.id === issueId);
    if (!issue?.fix) return;
    const fix = issue.fix;
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      switch (fix.kind) {
        case "set_edge_artifact_type_from_source":
        case "set_edge_artifact_type_from_target": {
          const edge = next.edges?.find((entry) => entry.id === fix.edgeId);
          if (edge) edge.artifact_type = fix.artifactType;
          break;
        }
        case "set_catalog_artifact_type": {
          const artifacts = next[fix.mode === "entry" ? "entry_inputs" : "output_artifacts"] ?? [];
          const artifact = artifacts.find(
            (entry) =>
              entry.node_id === fix.nodeId &&
              entry.artifact_type === fix.currentArtifactType,
          );
          if (artifact) artifact.artifact_type = fix.artifactType;
          break;
        }
        case "clear_port_dataset_value": {
          const node = next.nodes.find((entry) => entry.id === fix.nodeId);
          const port = node?.[fix.direction]?.find((entry) => entry.id === fix.portId);
          if (port) port.dataset_value = undefined;
          break;
        }
        case "clear_edge_dataset_value": {
          const edge = next.edges?.find((entry) => entry.id === fix.edgeId);
          if (edge) edge.dataset_value = undefined;
          break;
        }
      }
      return next;
    });
  }
  function locateValidationIssue(issueId: string) {
    const issue = validationIssues.find((entry) => entry.id === issueId);
    if (!issue?.locate) return;
    const locate = issue.locate;
    resetBuilderFocus();
    if (locate.kind === "node") {
      setFocusedNodeId(locate.nodeId);
      queueMicrotask(() => {
        builderRootRef.current
          ?.querySelector<HTMLElement>(`[data-workflow-node-id="${locate.nodeId}"]`)
          ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      });
      return;
    }
    if (locate.kind === "edge") {
      setFocusedEdgeId(locate.edgeId);
      queueMicrotask(() => {
        builderRootRef.current
          ?.querySelector<HTMLElement>(`[data-workflow-edge-id="${locate.edgeId}"]`)
          ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      });
      return;
    }
    if (locate.kind === "dataset") {
      const existingValue = locate.datasetValueId
        ? selectedDatasetValues.find((value) => value.id === locate.datasetValueId)
        : null;
      if (existingValue) {
        setSelectedDatasetValueId(existingValue.id);
        setFocusedDatasetValueId(existingValue.id);
      }
      setHighlightDatasetEditor(true);
      queueMicrotask(() => {
        builderRootRef.current
          ?.querySelector<HTMLElement>('[data-workflow-dataset-editor="editor"]')
          ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      });
      return;
    }
    const artifactIndex = (locate.mode === "entry" ? selectedEntryInputs : selectedOutputArtifacts).findIndex(
      (artifact) =>
        artifact.node_id === locate.nodeId &&
        artifact.artifact_type === locate.artifactType,
    );
    if (artifactIndex >= 0) {
      const artifactKey = `${locate.mode}:${locate.nodeId}:${locate.artifactType}:${artifactIndex}`;
      setFocusedArtifactKey(artifactKey);
      queueMicrotask(() => {
        builderRootRef.current
          ?.querySelector<HTMLElement>(`[data-workflow-artifact-key="${artifactKey}"]`)
          ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      });
    }
  }
  function addDatasetValue() {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      const contract = ensureDatasetContract(next);
      if (!contract) return current;
      const nextValue = buildDraftDatasetValue(contract.values.length + 1);
      contract.values = [...contract.values, nextValue];
      setSelectedDatasetValueId(nextValue.id);
      return next;
    });
  }
  function removeSelectedDatasetValue() {
    if (!selectedDatasetValue) return;
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      const contract = ensureDatasetContract(next);
      if (!contract) return current;
      contract.values = contract.values.filter((value) => value.id !== selectedDatasetValue.id);
      for (const node of next.nodes) {
        for (const port of [...(node.inputs ?? []), ...(node.outputs ?? [])]) {
          if (port.dataset_value === selectedDatasetValue.id) port.dataset_value = undefined;
        }
      }
      for (const edge of next.edges ?? []) {
        if (edge.dataset_value === selectedDatasetValue.id) edge.dataset_value = undefined;
      }
      setSelectedDatasetValueId(contract.values[0]?.id ?? null);
      return next;
    });
  }
  function updateDatasetAxes(
    valueId: string,
    updater: (axes: WorkflowDatasetAxis[]) => WorkflowDatasetAxis[],
  ) {
    updateDatasetValue(valueId, (value) => ({
      ...value,
      shape: {
        ...(value.shape ?? {}),
        axes: updater(value.shape?.axes ?? []),
      },
    }));
  }
  function addDatasetAxis() {
    if (!selectedDatasetValue) return;
    updateDatasetAxes(selectedDatasetValue.id, (axes) => [
      ...axes,
      { id: `axis_${axes.length + 1}` },
    ]);
  }
  function removeDatasetAxis(axisId: string) {
    if (!selectedDatasetValue) return;
    updateDatasetAxes(selectedDatasetValue.id, (axes) => axes.filter((axis) => axis.id !== axisId));
  }
  function updateDatasetAxis(
    axisId: string,
    updater: (axis: WorkflowDatasetAxis) => WorkflowDatasetAxis,
  ) {
    if (!selectedDatasetValue) return;
    updateDatasetAxes(selectedDatasetValue.id, (axes) =>
      axes.map((axis) => (axis.id === axisId ? updater(axis) : axis)),
    );
  }
  function exportDraftWorkflowGraph() {
    if (!selectedGraph) return;
    downloadJsonArtifact(
      `${slugifyWorkflowAssetName(selectedGraph.id)}.workflow-graph.json`,
      selectedGraph,
    );
  }
  function saveCurrentDraft() {
    if (!selectedWorkflow || !selectedGraph) return;
    saveStoredWorkflowDraft({
      workflowId: selectedWorkflow.id,
      workflowName: selectedWorkflow.name,
      graph: selectedGraph,
      inputArtifactTexts: draftInputTexts,
    });
    setSavedDrafts(listStoredWorkflowDrafts(selectedWorkflow.id));
    setImportMessage(labels.draftSavedLabel);
  }
  function runCurrentDraft() {
    if (!selectedWorkflow || !selectedGraph) return;
    if (parsedDraftInputs.invalidKeys.length > 0) {
      setImportMessage(labels.runDraftInvalidInputsLabel);
      return;
    }
    onRunWorkflowDraft(selectedWorkflow.id, selectedGraph, parsedDraftInputs.inputArtifacts);
  }
  function promoteCurrentDraft() {
    if (!selectedWorkflow || !selectedGraph) return;
    saveStoredLocalWorkflow({
      sourceWorkflowId: selectedWorkflow.local?.source_workflow_id ?? selectedWorkflow.id,
      workflowName: selectedWorkflow.name,
      graph: selectedGraph,
      inputArtifactTexts: draftInputTexts,
    });
    onRefreshWorkflowCatalog();
    setImportMessage(labels.localWorkflowPromotedLabel);
  }
  function renameCurrentLocalWorkflow() {
    if (!selectedWorkflow?.local) return;
    const nextName = window.prompt(labels.localWorkflowRenamePrompt, selectedWorkflow.name);
    if (!nextName?.trim()) return;
    renameStoredLocalWorkflow(selectedWorkflow.local.storage_id, nextName);
    onRefreshWorkflowCatalog();
    setImportMessage(`${labels.localWorkflowBadgeLabel}: ${nextName.trim()}`);
  }
  function duplicateCurrentLocalWorkflow() {
    if (!selectedWorkflow?.local) return;
    duplicateStoredLocalWorkflow(selectedWorkflow.local.storage_id);
    onRefreshWorkflowCatalog();
    setImportMessage(labels.localWorkflowDuplicatedLabel);
  }
  function deleteCurrentLocalWorkflow() {
    if (!selectedWorkflow?.local) return;
    removeStoredLocalWorkflow(selectedWorkflow.local.storage_id);
    onRefreshWorkflowCatalog();
    setImportMessage(labels.localWorkflowDeletedLabel);
  }
  function saveCurrentLocalWorkflowMetadata(summary: string, notes: string) {
    if (!selectedWorkflow?.local) return;
    updateStoredLocalWorkflowMetadata(selectedWorkflow.local.storage_id, { notes, summary });
    onRefreshWorkflowCatalog();
    setImportMessage(labels.localWorkflowMetadataSavedLabel);
  }
  function loadSavedDraft(draftId: string) {
    const draft = savedDrafts.find((entry) => entry.id === draftId);
    if (!draft) return;
    const nextGraph = cloneWorkflowGraph(draft.graph);
    setDraftGraph(nextGraph);
    setDraftInputTexts(
      draft.inputArtifactTexts ??
        buildWorkflowInputArtifactTexts(
          nextGraph?.entry_inputs ?? [],
          selectedWorkflow ? builtInWorkflowSampleInputArtifacts(selectedWorkflow.id) : null,
        ),
    );
    setSelectedDatasetValueId(nextGraph?.dataset_contract?.values?.[0]?.id ?? null);
    resetBuilderFocus();
    setImportMessage(labels.draftLoadedLabel);
  }
  function deleteSavedDraft(draftId: string) {
    if (!selectedWorkflow) return;
    removeStoredWorkflowDraft(draftId);
    setSavedDrafts(listStoredWorkflowDrafts(selectedWorkflow.id));
    setImportMessage(labels.draftDeletedLabel);
  }
  function exportDraftDatasetContract() {
    if (!selectedDatasetContract) return;
    downloadJsonArtifact(
      `${slugifyWorkflowAssetName(selectedDatasetContract.id)}.workflow-dataset.json`,
      selectedDatasetContract,
    );
  }
  async function importWorkflowGraphFile(file: File) {
    try {
      const json = await readJsonFile(file);
      const graph = asWorkflowGraphDefinition(json);
      if (!graph) {
        setImportMessage(labels.importInvalidGraphLabel);
        return;
      }
      const nextGraph = cloneWorkflowGraph(graph);
      if (nextGraph) {
        nextGraph.entry_inputs = nextGraph.entry_inputs ?? selectedEntryInputs;
        nextGraph.output_artifacts = nextGraph.output_artifacts ?? selectedOutputArtifacts;
      }
      setDraftGraph(nextGraph);
      setSelectedDatasetValueId(nextGraph?.dataset_contract?.values?.[0]?.id ?? null);
      setImportMessage(labels.importSuccessLabel);
    } catch {
      setImportMessage(labels.importInvalidGraphLabel);
    }
  }
  async function importDatasetContractFile(file: File) {
    try {
      const json = await readJsonFile(file);
      const contract = asWorkflowDatasetContract(json);
      if (!contract) {
        setImportMessage(labels.importInvalidDatasetLabel);
        return;
      }
      setDraftGraph((current) => mergeDatasetContractIntoGraph(cloneWorkflowGraph(current), contract));
      setSelectedDatasetValueId(contract.values[0]?.id ?? null);
      setImportMessage(labels.importSuccessLabel);
    } catch {
      setImportMessage(labels.importInvalidDatasetLabel);
    }
  }
  function handleGraphFileChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) void importWorkflowGraphFile(file);
    event.target.value = "";
  }
  function handleDatasetFileChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) void importDatasetContractFile(file);
    event.target.value = "";
  }

  function updateDraftInputText(nodeId: string, value: string) { setDraftInputTexts((current) => ({ ...current, [nodeId]: value })); }
  if (!selectedWorkflow) return <section className="sidebar-card sidebar-card--compact"><p className="card-copy">{labels.noSelectionLabel}</p></section>;

  return (
    <section className="sidebar-card sidebar-card--compact" ref={builderRootRef}>
      <WorkbenchWorkflowBuilderToolbar
        canExportDataset={Boolean(selectedDatasetContract)}
        canRunDraft={Boolean(selectedGraph)}
        datasetInputRef={datasetInputRef}
        graphInputRef={graphInputRef}
        importMessage={importMessage}
        labels={labels}
        onDatasetFileChange={handleDatasetFileChange}
        onExportDataset={exportDraftDatasetContract}
        onDuplicateLocalWorkflow={duplicateCurrentLocalWorkflow}
        onExportGraph={exportDraftWorkflowGraph}
        onDeleteLocalWorkflow={deleteCurrentLocalWorkflow}
        onGraphFileChange={handleGraphFileChange}
        onPromoteDraft={promoteCurrentDraft}
        onRenameLocalWorkflow={renameCurrentLocalWorkflow}
        onRunCatalog={() => onRunWorkflowCatalog(selectedWorkflow.id)}
        onRunDraft={runCurrentDraft}
        onSaveDraft={saveCurrentDraft}
        selectedWorkflow={selectedWorkflow}
      />
      <WorkbenchWorkflowDraftCard
        drafts={savedDrafts}
        labels={labels}
        onDeleteDraft={deleteSavedDraft}
        onLoadDraft={loadSavedDraft}
        onSaveDraft={saveCurrentDraft}
      />
      {selectedWorkflow.local ? (
        <WorkbenchWorkflowLocalMetadataCard labels={labels} onSave={saveCurrentLocalWorkflowMetadata} workflow={selectedWorkflow} />
      ) : null}
      <WorkbenchWorkflowInputArtifactsCard
        entryInputs={selectedEntryInputs} inputTexts={draftInputTexts} invalidKeys={parsedDraftInputs.invalidKeys}
        labels={labels} onChangeInputText={updateDraftInputText} />
      <WorkbenchWorkflowValidationCard labels={labels} onApplyValidationFix={applyValidationFix} onLocateValidationIssue={locateValidationIssue} validationIssues={validationIssues} />
      <WorkbenchWorkflowGraphSummaryCard focusedEdgeId={focusedEdgeId} focusedNodeId={focusedNodeId} labels={labels} selectedEdges={selectedEdges} selectedEntryInputsCount={selectedEntryInputs.length} selectedNodes={selectedNodes} selectedOutputArtifactsCount={selectedOutputArtifacts.length} />
      <WorkbenchWorkflowTopologyCard
        focusedEdgeId={focusedEdgeId}
        focusedNodeId={focusedNodeId}
        labels={labels}
        operatorDescriptors={operatorDescriptors}
        onAddEdge={topologyActions.addEdge}
        onAddConnectedNode={topologyActions.addConnectedNode}
        onInsertTemplateChain={topologyActions.insertTemplateChain}
        onAddNode={topologyActions.addNode}
        onAddNodePort={topologyActions.addNodePort}
        onRemoveEdge={topologyActions.removeEdge}
        onRemoveNode={topologyActions.removeNode}
        onRemoveNodePort={topologyActions.removeNodePort}
        onUpdateEdge={topologyActions.updateEdge}
        onUpdateNode={topologyActions.updateNode}
        onUpdateNodePort={topologyActions.updateNodePort}
        selectedEdges={selectedEdges}
        selectedNodes={selectedNodes}
      />
      <WorkbenchWorkflowDatasetCard addDatasetAxis={addDatasetAxis} addDatasetValue={addDatasetValue} labels={labels} removeDatasetAxis={removeDatasetAxis} removeSelectedDatasetValue={removeSelectedDatasetValue} selectedDatasetContract={selectedDatasetContract} selectedDatasetValue={selectedDatasetValue} selectedDatasetValueId={selectedDatasetValueId} selectedDatasetValues={selectedDatasetValues} selectedEdges={selectedEdges} focusedDatasetValueId={focusedDatasetValueId} highlightDatasetEditor={highlightDatasetEditor} selectedNodes={selectedNodes} setSelectedDatasetValueId={setSelectedDatasetValueId} updateDatasetAxis={updateDatasetAxis} updateDatasetValue={updateDatasetValue} updateEdgeDatasetValue={updateEdgeDatasetValue} updateNodePortDatasetValue={updateNodePortDatasetValue} />
      <WorkbenchWorkflowArtifactCard
        addLabel={labels.artifactAddEntryLabel}
        artifacts={selectedEntryInputs}
        labels={labels}
        mode="entry"
        onAddArtifact={() => addArtifact("entry_inputs")}
        onRemoveArtifact={(index) => removeArtifact("entry_inputs", index)}
        onUpdateArtifact={(index, updater) => updateArtifact("entry_inputs", index, updater)}
        focusedArtifactKey={focusedArtifactKey}
        selectedNodes={selectedNodes}
        title={labels.entryInputsTitle}
      />
      <WorkbenchWorkflowArtifactCard
        addLabel={labels.artifactAddOutputLabel}
        artifacts={selectedOutputArtifacts}
        labels={labels}
        mode="output"
        onAddArtifact={() => addArtifact("output_artifacts")}
        onRemoveArtifact={(index) => removeArtifact("output_artifacts", index)}
        onUpdateArtifact={(index, updater) => updateArtifact("output_artifacts", index, updater)}
        focusedArtifactKey={focusedArtifactKey}
        selectedNodes={selectedNodes}
        title={labels.outputArtifactsTitle}
      />
    </section>
  );
}
