"use client";
import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import type { WorkflowCatalogEntryArtifact, WorkflowDatasetAxis, WorkflowCatalogEntry, WorkflowDatasetValueInfo, WorkflowGraphDefinition, WorkflowOperatorDescriptor } from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import { asWorkflowDatasetContract, asWorkflowGraphDefinition, mergeDatasetContractIntoGraph, normalizeImportedWorkflowGraph, readJsonFile } from "@/components/workbench/workflow/workbench-workflow-builder-import";
import { listStoredWorkflowDrafts, removeStoredWorkflowDraft, saveStoredWorkflowDraft, type StoredWorkflowDraft } from "@/components/workbench/workflow/workbench-workflow-draft-storage";
import { duplicateStoredLocalWorkflow, removeStoredLocalWorkflow, renameStoredLocalWorkflow, saveStoredLocalWorkflow, updateStoredLocalWorkflowMetadata } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { asWorkflowDraftBundle, buildWorkflowDraftBundle } from "@/components/workbench/workflow/workbench-workflow-draft-bundle";
import { WorkbenchWorkflowDraftCard } from "@/components/workbench/workflow/workbench-workflow-draft-card";
import { WorkbenchWorkflowBuilderToolbar } from "@/components/workbench/workflow/workbench-workflow-builder-toolbar";
import { buildWorkflowInputArtifactTexts, parseWorkflowInputArtifactTexts } from "@/components/workbench/workflow/workbench-workflow-input-artifacts";
import { WorkbenchWorkflowInputArtifactsCard } from "@/components/workbench/workflow/workbench-workflow-input-artifacts-card";
import { WorkbenchWorkflowArtifactCard } from "@/components/workbench/workflow/workbench-workflow-artifact-card";
import { buildDraftArtifact, buildDraftDatasetValue, cloneWorkflowGraph, downloadJsonArtifact, ensureDatasetContract, slugifyWorkflowAssetName } from "@/components/workbench/workflow/workbench-workflow-builder-utils";
import { createWorkflowTopologyActions } from "@/components/workbench/workflow/workbench-workflow-topology-actions";
import { applyAllWorkflowValidationFixes, applyWorkflowValidationFix, validateWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import { buildWorkflowValidationHighlightPlan } from "@/components/workbench/workflow/workbench-workflow-validation-highlights";
import { buildWorkflowValidationFixSummary } from "@/components/workbench/workflow/workbench-workflow-validation-summary";
import { listStoredWorkflowSnapshots, loadStoredWorkflowSnapshot, removeStoredWorkflowSnapshot, saveStoredWorkflowSnapshot, type StoredWorkflowSnapshotSummary } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";
import { describeWorkflowNodeTemplateSyncImpact, getWorkflowNodeTemplateSyncImpact, listAutoReconnectEdgeIds } from "@/components/workbench/workflow/workbench-workflow-template-impact";
import { WorkbenchWorkflowDatasetCard } from "@/components/workbench/workflow/workbench-workflow-dataset-card";
import { WorkbenchWorkflowControlFlowPlaneCard } from "@/components/workbench/workflow/workbench-workflow-control-flow-plane-card";
import { WorkbenchWorkflowGraphSummaryCard } from "@/components/workbench/workflow/workbench-workflow-graph-summary-card";
import { WorkbenchWorkflowIntegrityCard } from "@/components/workbench/workflow/workbench-workflow-integrity-card";
import { buildWorkflowIntegrityReport, type WorkflowIntegrityIssue } from "@/components/workbench/workflow/workbench-workflow-integrity";
import { WorkbenchWorkflowLocalMetadataCard } from "@/components/workbench/workflow/workbench-workflow-local-metadata-card";
import { WorkbenchWorkflowSnapshotCard } from "@/components/workbench/workflow/workbench-workflow-snapshot-card";
import { builtInWorkflowSampleInputArtifacts } from "@/components/workbench/workflow/workbench-workflow-sample-inputs";
import { WorkbenchWorkflowTopologyCard } from "@/components/workbench/workflow/workbench-workflow-topology-card";
import { readWorkflowTemplateChainPreferences, writeWorkflowTemplateChainPreferences } from "@/components/workbench/workflow/workbench-workflow-template-chain-storage";
import { WorkbenchWorkflowValidationCard } from "@/components/workbench/workflow/workbench-workflow-validation-card";
type WorkbenchWorkflowBuilderCardProps = {
  labels: WorkflowSidebarLabels;
  selectedWorkflow: WorkflowCatalogEntry | null;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  onRefreshWorkflowCatalog: () => void;
  onRunWorkflowCatalog: (workflowId: string) => void;
  onRunWorkflowDraft: (workflowId: string, graph: WorkflowGraphDefinition, inputArtifacts: Record<string, unknown>) => void; traceFocusNodeId?: string | null; traceFocusToken?: number; traceFocusBranchNodeId?: string | null; traceFocusBranchOutputId?: string | null; traceFocusBranchToken?: number; traceFocusDatasetNodeId?: string | null; traceFocusDatasetPortId?: string | null; traceFocusDatasetToken?: number;
};
export function WorkbenchWorkflowBuilderCard({
  labels,
  selectedWorkflow,
  operatorDescriptors,
  onRefreshWorkflowCatalog,
  onRunWorkflowCatalog,
  onRunWorkflowDraft,
  traceFocusNodeId,
  traceFocusToken,
  traceFocusBranchNodeId,
  traceFocusBranchOutputId,
  traceFocusBranchToken,
  traceFocusDatasetNodeId,
  traceFocusDatasetPortId,
  traceFocusDatasetToken,
}: WorkbenchWorkflowBuilderCardProps) {
  const [draftGraph, setDraftGraph] = useState<WorkflowGraphDefinition | null>(null);
  const [draftInputTexts, setDraftInputTexts] = useState<Record<string, string>>({});
  const [selectedDatasetValueId, setSelectedDatasetValueId] = useState<string | null>(null);
  const [importMessage, setImportMessage] = useState<string | null>(null);
  const [savedDrafts, setSavedDrafts] = useState<StoredWorkflowDraft[]>([]);
  const [savedSnapshots, setSavedSnapshots] = useState<StoredWorkflowSnapshotSummary[]>([]);
  const [recentFixSummary, setRecentFixSummary] = useState<string[]>([]);
  const [focusedNodeId, setFocusedNodeId] = useState<string | null>(null);
  const [focusedEdgeId, setFocusedEdgeId] = useState<string | null>(null);
  const [focusedArtifactKey, setFocusedArtifactKey] = useState<string | null>(null);
  const [focusedDatasetValueId, setFocusedDatasetValueId] = useState<string | null>(null);
  const [highlightedNodeIds, setHighlightedNodeIds] = useState<string[]>([]);
  const [highlightedEdgeIds, setHighlightedEdgeIds] = useState<string[]>([]);
  const [highlightedArtifactKeys, setHighlightedArtifactKeys] = useState<string[]>([]);
  const [highlightDatasetEditor, setHighlightDatasetEditor] = useState(false);
  const graphInputRef = useRef<HTMLInputElement | null>(null);
  const datasetInputRef = useRef<HTMLInputElement | null>(null);
  const builderRootRef = useRef<HTMLElement | null>(null);
  function resetBuilderFocus() { setFocusedNodeId(null); setFocusedEdgeId(null); setFocusedArtifactKey(null); setFocusedDatasetValueId(null); setHighlightedNodeIds([]); setHighlightedEdgeIds([]); setHighlightedArtifactKeys([]); setHighlightDatasetEditor(false); }
  function flashHighlightedEdges(edgeIds: string[]) {
    if (edgeIds.length === 0) return;
    setHighlightedEdgeIds(edgeIds);
    window.setTimeout(() => setHighlightedEdgeIds((current) => (current === edgeIds ? [] : current)), 2200);
  }
  function flashValidationHighlights(graph: WorkflowGraphDefinition | null, issues: typeof validationIssues) {
    const plan = buildWorkflowValidationHighlightPlan(graph, issues, operatorDescriptors ?? []);
    if (plan.firstNodeId) setFocusedNodeId(plan.firstNodeId);
    if (plan.firstEdgeId) setFocusedEdgeId(plan.firstEdgeId);
    if (plan.firstArtifactKey) setFocusedArtifactKey(plan.firstArtifactKey);
    if (plan.datasetValueId) setSelectedDatasetValueId(plan.datasetValueId);
    if (plan.datasetValueId || plan.highlightDatasetEditor) setFocusedDatasetValueId(plan.datasetValueId);
    setHighlightedNodeIds(plan.nodeIds);
    setHighlightedEdgeIds(plan.edgeIds);
    setHighlightedArtifactKeys(plan.artifactKeys);
    setHighlightDatasetEditor(plan.highlightDatasetEditor);
    queueMicrotask(() => {
      const root = builderRootRef.current;
      const target = plan.firstNodeId
        ? root?.querySelector<HTMLElement>(`[data-workflow-node-id="${plan.firstNodeId}"]`)
        : plan.firstEdgeId
          ? root?.querySelector<HTMLElement>(`[data-workflow-edge-id="${plan.firstEdgeId}"]`)
          : plan.firstArtifactKey
            ? root?.querySelector<HTMLElement>(`[data-workflow-artifact-key="${plan.firstArtifactKey}"]`)
            : plan.highlightDatasetEditor
              ? root?.querySelector<HTMLElement>('[data-workflow-dataset-editor="editor"]')
              : null;
      target?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
    window.setTimeout(() => {
      setHighlightedNodeIds((current) => (current === plan.nodeIds ? [] : current));
      setHighlightedEdgeIds((current) => (current === plan.edgeIds ? [] : current));
      setHighlightedArtifactKeys((current) => (current === plan.artifactKeys ? [] : current));
      setHighlightDatasetEditor((current) => (current && plan.highlightDatasetEditor ? false : current));
    }, 2200);
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
    setRecentFixSummary([]);
    resetBuilderFocus();
    setSavedDrafts(selectedWorkflow ? listStoredWorkflowDrafts(selectedWorkflow.id) : []);
    setSavedSnapshots(selectedWorkflow ? listStoredWorkflowSnapshots(selectedWorkflow.id) : []);
  }, [selectedWorkflow]);
  useEffect(() => { if (!traceFocusNodeId) return; setFocusedNodeId(traceFocusNodeId); setFocusedEdgeId(null); setHighlightedNodeIds([traceFocusNodeId]); queueMicrotask(() => builderRootRef.current?.querySelector<HTMLElement>(`[data-workflow-node-id="${traceFocusNodeId}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" })); window.setTimeout(() => setHighlightedNodeIds((current) => (current[0] === traceFocusNodeId ? [] : current)), 2200); }, [traceFocusNodeId, traceFocusToken]);
  useEffect(() => { if (!traceFocusBranchNodeId || !traceFocusBranchOutputId) return; const graph = draftGraph; const branchEdges = (graph?.edges ?? []).filter((edge) => edge.from.node === traceFocusBranchNodeId && edge.from.port === traceFocusBranchOutputId); const mergeNode = branchEdges.map((edge) => graph?.nodes.find((node) => node.id === edge.to.node && node.operator_id === "transform.first_available") ?? null).find(Boolean) ?? null; const downstreamEdgeIds = mergeNode ? (graph?.edges ?? []).filter((edge) => edge.from.node === mergeNode.id && edge.from.port === "merged").map((edge) => edge.id) : []; setFocusedNodeId(traceFocusBranchNodeId); setHighlightedNodeIds(mergeNode ? [traceFocusBranchNodeId, mergeNode.id] : [traceFocusBranchNodeId]); if (branchEdges.length + downstreamEdgeIds.length > 0) flashHighlightedEdges([...branchEdges.map((edge) => edge.id), ...downstreamEdgeIds]); }, [draftGraph, traceFocusBranchNodeId, traceFocusBranchOutputId, traceFocusBranchToken]);
  useEffect(() => { if (!traceFocusDatasetNodeId || !traceFocusDatasetPortId) return; const valueId = draftGraph?.nodes.find((node) => node.id === traceFocusDatasetNodeId)?.outputs?.find((port) => port.id === traceFocusDatasetPortId)?.dataset_value ?? null; if (!valueId) return; const nodeIds = (draftGraph?.nodes ?? []).filter((node) => [...(node.inputs ?? []), ...(node.outputs ?? [])].some((port) => port.dataset_value === valueId)).map((node) => node.id); const edgeIds = (draftGraph?.edges ?? []).filter((edge) => edge.dataset_value === valueId).map((edge) => edge.id); setSelectedDatasetValueId(valueId); setFocusedDatasetValueId(valueId); setHighlightDatasetEditor(true); setHighlightedNodeIds(nodeIds); if (edgeIds.length > 0) setHighlightedEdgeIds(edgeIds); queueMicrotask(() => builderRootRef.current?.querySelector<HTMLElement>('[data-workflow-dataset-editor="editor"]')?.scrollIntoView({ block: "nearest", behavior: "smooth" })); window.setTimeout(() => { setHighlightDatasetEditor((current) => (current ? false : current)); setHighlightedNodeIds((current) => (current.join(",") === nodeIds.join(",") ? [] : current)); setHighlightedEdgeIds((current) => (current.join(",") === edgeIds.join(",") ? [] : current)); }, 2200); }, [draftGraph, traceFocusDatasetNodeId, traceFocusDatasetPortId, traceFocusDatasetToken]);
  const selectedGraph = draftGraph;
  const selectedNodes = selectedGraph?.nodes ?? [], selectedEdges = selectedGraph?.edges ?? [], selectedEntryInputs = selectedGraph?.entry_inputs ?? [], selectedOutputArtifacts = selectedGraph?.output_artifacts ?? [], selectedDatasetContract = selectedGraph?.dataset_contract ?? null, selectedDatasetValues = selectedDatasetContract?.values ?? [];
  const parsedDraftInputs = useMemo(() => parseWorkflowInputArtifactTexts(draftInputTexts), [draftInputTexts]);
  const validationIssues = useMemo(() => validateWorkflowGraphDefinition(selectedGraph, selectedEntryInputs, selectedOutputArtifacts, operatorDescriptors ?? []), [operatorDescriptors, selectedGraph, selectedEntryInputs, selectedOutputArtifacts]);
  const integrityReport = useMemo(() => buildWorkflowIntegrityReport(selectedWorkflow, operatorDescriptors ?? []), [operatorDescriptors, selectedWorkflow]);
  const draftBlockingIssueCount = validationIssues.length + parsedDraftInputs.invalidKeys.length, canRunDraft = Boolean(selectedGraph) && draftBlockingIssueCount === 0;
  const selectedDatasetValue = useMemo(
    () => selectedDatasetValues.find((value) => value.id === selectedDatasetValueId) ?? selectedDatasetValues[0] ?? null,
    [selectedDatasetValueId, selectedDatasetValues],
  );
  const topologyActions = createWorkflowTopologyActions(setDraftGraph, operatorDescriptors);
  function updateDatasetValue(valueId: string, updater: (value: WorkflowDatasetValueInfo) => WorkflowDatasetValueInfo) {
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
  function updateArtifacts(field: "entry_inputs" | "output_artifacts", updater: (artifacts: WorkflowCatalogEntryArtifact[]) => WorkflowCatalogEntryArtifact[]) {
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
    if (issue.fix.kind === "sync_node_template_from_operator" && selectedGraph) {
      const impact = getWorkflowNodeTemplateSyncImpact(
        selectedGraph,
        issue.fix.nodeId,
        { kind: issue.fix.templateKind, operatorId: issue.fix.operatorId },
        operatorDescriptors ?? [],
      );
      const preview = describeWorkflowNodeTemplateSyncImpact(impact);
      if (preview && !window.confirm(preview)) return;
      flashHighlightedEdges(listAutoReconnectEdgeIds(impact));
    }
    const nextGraph = applyWorkflowValidationFix(selectedGraph, issue, operatorDescriptors ?? []);
    if (selectedWorkflow && nextGraph) {
      const summary = buildWorkflowValidationFixSummary([issue]);
      saveStoredWorkflowSnapshot({ workflowId: selectedWorkflow.id, workflowName: selectedWorkflow.name, reason: "single validation fix", graph: nextGraph, inputArtifactTexts: draftInputTexts, summary });
      setSavedSnapshots(listStoredWorkflowSnapshots(selectedWorkflow.id));
      setRecentFixSummary(summary);
    }
    setDraftGraph(nextGraph);
    flashValidationHighlights(nextGraph, [issue]);
  }
  function applyAllValidationFixes() {
    const { graph, appliedCount, appliedIssues } = applyAllWorkflowValidationFixes(
      selectedGraph,
      selectedEntryInputs,
      selectedOutputArtifacts,
      operatorDescriptors ?? [],
    );
    if (appliedCount === 0) return;
    const summary = buildWorkflowValidationFixSummary(appliedIssues);
    if (selectedWorkflow && graph) {
      saveStoredWorkflowSnapshot({ workflowId: selectedWorkflow.id, workflowName: selectedWorkflow.name, reason: "batch validation fixes", graph, inputArtifactTexts: draftInputTexts, summary });
      setSavedSnapshots(listStoredWorkflowSnapshots(selectedWorkflow.id));
    }
    setDraftGraph(graph);
    setRecentFixSummary(summary);
    flashValidationHighlights(graph, appliedIssues);
    const firstFixedMessage = appliedIssues[0]?.message;
    setImportMessage(
      [labels.validationAutoFixedLabel.replace("{count}", String(appliedCount)), firstFixedMessage].filter(Boolean).join(" "),
    );
  }
  function locateValidationIssue(issueId: string) { const issue = validationIssues.find((entry) => entry.id === issueId); if (issue?.locate) locateBuilderIssue(issue.locate); }
  function locateIntegrityIssue(issue: WorkflowIntegrityIssue) { if (issue.locate) locateBuilderIssue(issue.locate); }
  function locateBuilderIssue(locate: NonNullable<WorkflowIntegrityIssue["locate"]> | NonNullable<NonNullable<typeof validationIssues[number]>["locate"]>) {
    resetBuilderFocus();
    if (locate.kind === "node") {
      setFocusedNodeId(locate.nodeId);
      queueMicrotask(() => { builderRootRef.current?.querySelector<HTMLElement>(`[data-workflow-node-id="${locate.nodeId}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" }); });
      return;
    }
    if (locate.kind === "edge") {
      setFocusedEdgeId(locate.edgeId);
      queueMicrotask(() => { builderRootRef.current?.querySelector<HTMLElement>(`[data-workflow-edge-id="${locate.edgeId}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" }); });
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
      queueMicrotask(() => { builderRootRef.current?.querySelector<HTMLElement>('[data-workflow-dataset-editor="editor"]')?.scrollIntoView({ block: "nearest", behavior: "smooth" }); });
      return;
    }
    if (locate.kind === "snapshot" || locate.kind === "local") {
      queueMicrotask(() => { builderRootRef.current?.querySelector<HTMLElement>(locate.kind === "snapshot" ? '[data-workflow-snapshot-card="card"]' : '[data-workflow-local-card="card"]')?.scrollIntoView({ block: "nearest", behavior: "smooth" }); });
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
      queueMicrotask(() => { builderRootRef.current?.querySelector<HTMLElement>(`[data-workflow-artifact-key="${artifactKey}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" }); });
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
  function updateDatasetAxes(valueId: string, updater: (axes: WorkflowDatasetAxis[]) => WorkflowDatasetAxis[]) {
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
    updateDatasetAxes(selectedDatasetValue.id, (axes) => [...axes, { id: `axis_${axes.length + 1}` }]);
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
      buildWorkflowDraftBundle({
        graph: selectedGraph,
        inputArtifactTexts: draftInputTexts,
        templateChainPreferences: readWorkflowTemplateChainPreferences(),
      }),
    );
  }
  function saveCurrentDraft() {
    if (!selectedWorkflow || !selectedGraph) return;
    saveStoredWorkflowDraft({
      workflowId: selectedWorkflow.id,
      workflowName: selectedWorkflow.name,
      graph: selectedGraph,
      inputArtifactTexts: draftInputTexts,
      templateChainPreferences: readWorkflowTemplateChainPreferences(),
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
    if (validationIssues.length > 0) {
      const firstIssue = validationIssues[0];
      locateValidationIssue(firstIssue.id);
      setImportMessage(firstIssue.message);
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
    if (draft.templateChainPreferences) {
      writeWorkflowTemplateChainPreferences(draft.templateChainPreferences);
    }
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
  function restoreSnapshot(snapshotId: string) {
    const snapshot = loadStoredWorkflowSnapshot(snapshotId);
    if (!snapshot) {
      setImportMessage(labels.validationSnapshotRestoreUnavailableLabel);
      return;
    }
    const nextGraph = cloneWorkflowGraph(snapshot.graph);
    if (!nextGraph) return;
    setDraftGraph(nextGraph);
    setDraftInputTexts(snapshot.inputArtifactTexts ?? buildWorkflowInputArtifactTexts(nextGraph.entry_inputs ?? [], selectedWorkflow ? builtInWorkflowSampleInputArtifacts(selectedWorkflow.id) : null));
    setSelectedDatasetValueId(nextGraph.dataset_contract?.values?.[0]?.id ?? null);
    setRecentFixSummary(snapshot.summary);
    setImportMessage(snapshot.reason);
    resetBuilderFocus();
  }
  function deleteSnapshot(snapshotId: string) {
    if (!selectedWorkflow) return;
    removeStoredWorkflowSnapshot(snapshotId);
    setSavedSnapshots(listStoredWorkflowSnapshots(selectedWorkflow.id));
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
      const bundle = asWorkflowDraftBundle(json);
      const graph = bundle?.graph ?? asWorkflowGraphDefinition(json);
      if (!graph) {
        setImportMessage(labels.importInvalidGraphLabel);
        return;
      }
      const imported = normalizeImportedWorkflowGraph(graph, operatorDescriptors ?? []);
      const nextGraph = cloneWorkflowGraph(imported.graph);
      if (nextGraph) {
        nextGraph.entry_inputs = nextGraph.entry_inputs ?? selectedEntryInputs;
        nextGraph.output_artifacts = nextGraph.output_artifacts ?? selectedOutputArtifacts;
      }
      setDraftGraph(nextGraph);
      setDraftInputTexts(
        bundle?.input_artifact_texts ??
          buildWorkflowInputArtifactTexts(
            nextGraph?.entry_inputs ?? [],
            selectedWorkflow ? builtInWorkflowSampleInputArtifacts(selectedWorkflow.id) : null,
          ),
      );
      if (bundle?.template_chain_preferences) {
        writeWorkflowTemplateChainPreferences(bundle.template_chain_preferences);
      }
      setSelectedDatasetValueId(nextGraph?.dataset_contract?.values?.[0]?.id ?? null);
      flashHighlightedEdges(imported.autoReconnectEdgeIds);
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
        canRunDraft={canRunDraft}
        draftBlockingIssueCount={draftBlockingIssueCount}
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
      <WorkbenchWorkflowDraftCard drafts={savedDrafts} labels={labels} onDeleteDraft={deleteSavedDraft} onLoadDraft={loadSavedDraft} onSaveDraft={saveCurrentDraft} />
      {selectedWorkflow.local ? <WorkbenchWorkflowLocalMetadataCard labels={labels} onSave={saveCurrentLocalWorkflowMetadata} workflow={selectedWorkflow} /> : null}
      <WorkbenchWorkflowInputArtifactsCard entryInputs={selectedEntryInputs} inputTexts={draftInputTexts} invalidKeys={parsedDraftInputs.invalidKeys} labels={labels} onChangeInputText={updateDraftInputText} />
      <WorkbenchWorkflowValidationCard labels={labels} recentFixSummary={recentFixSummary} onApplyAllValidationFixes={applyAllValidationFixes} onApplyValidationFix={applyValidationFix} onLocateValidationIssue={locateValidationIssue} validationIssues={validationIssues} />
      <WorkbenchWorkflowIntegrityCard onLocateIssue={locateIntegrityIssue} report={integrityReport} />
      <WorkbenchWorkflowSnapshotCard labels={labels} onDeleteSnapshot={deleteSnapshot} onRestoreSnapshot={restoreSnapshot} snapshots={savedSnapshots} />
      <WorkbenchWorkflowGraphSummaryCard focusedEdgeId={focusedEdgeId} focusedNodeId={focusedNodeId} highlightedEdgeIds={highlightedEdgeIds} highlightedNodeIds={highlightedNodeIds} labels={labels} selectedEdges={selectedEdges} selectedEntryInputsCount={selectedEntryInputs.length} selectedNodes={selectedNodes} selectedOutputArtifactsCount={selectedOutputArtifacts.length} />
      <WorkbenchWorkflowControlFlowPlaneCard labels={labels} operatorDescriptors={operatorDescriptors} selectedEdges={selectedEdges} selectedNodes={selectedNodes} validationIssues={validationIssues} invalidInputCount={parsedDraftInputs.invalidKeys.length} traceFocusBranchNodeId={traceFocusBranchNodeId} traceFocusBranchOutputId={traceFocusBranchOutputId} traceFocusBranchToken={traceFocusBranchToken} onAddConditionNode={() => topologyActions.addNode({ kind: "condition" })} onAddMergeNode={() => topologyActions.addNode({ kind: "transform", operatorId: "transform.first_available" })} onAddNode={topologyActions.addNode} onSyncNodeTemplate={topologyActions.syncNodeTemplate} onInsertControlFlowPlane={topologyActions.insertControlFlowPlane} onSetControlFlowEdge={topologyActions.setControlFlowEdge} />
      <WorkbenchWorkflowTopologyCard focusedEdgeId={focusedEdgeId} focusedNodeId={focusedNodeId} highlightedNodeIds={highlightedNodeIds} labels={labels} operatorDescriptors={operatorDescriptors} onAddEdge={topologyActions.addEdge} onAddConnectedNode={topologyActions.addConnectedNode} onInsertTemplateChain={topologyActions.insertTemplateChain} onAddNode={topologyActions.addNode} onAddNodePort={topologyActions.addNodePort} onRemoveEdge={topologyActions.removeEdge} onRemoveNode={topologyActions.removeNode} onRemoveNodePort={topologyActions.removeNodePort} onSyncNodeTemplate={topologyActions.syncNodeTemplate} onUpdateEdge={topologyActions.updateEdge} onUpdateNode={topologyActions.updateNode} onUpdateNodePort={topologyActions.updateNodePort} highlightedEdgeIds={highlightedEdgeIds} selectedEdges={selectedEdges} selectedNodes={selectedNodes} />
      <WorkbenchWorkflowDatasetCard addDatasetAxis={addDatasetAxis} addDatasetValue={addDatasetValue} labels={labels} removeDatasetAxis={removeDatasetAxis} removeSelectedDatasetValue={removeSelectedDatasetValue} selectedDatasetContract={selectedDatasetContract} selectedDatasetValue={selectedDatasetValue} selectedDatasetValueId={selectedDatasetValueId} selectedDatasetValues={selectedDatasetValues} selectedEdges={selectedEdges} focusedDatasetValueId={focusedDatasetValueId} highlightDatasetEditor={highlightDatasetEditor} selectedNodes={selectedNodes} setSelectedDatasetValueId={setSelectedDatasetValueId} updateDatasetAxis={updateDatasetAxis} updateDatasetValue={updateDatasetValue} updateEdgeDatasetValue={updateEdgeDatasetValue} updateNodePortDatasetValue={updateNodePortDatasetValue} />
      <WorkbenchWorkflowArtifactCard addLabel={labels.artifactAddEntryLabel} artifacts={selectedEntryInputs} highlightedArtifactKeys={highlightedArtifactKeys} labels={labels} mode="entry" onAddArtifact={() => addArtifact("entry_inputs")} onRemoveArtifact={(index) => removeArtifact("entry_inputs", index)} onUpdateArtifact={(index, updater) => updateArtifact("entry_inputs", index, updater)} focusedArtifactKey={focusedArtifactKey} selectedNodes={selectedNodes} title={labels.entryInputsTitle} />
      <WorkbenchWorkflowArtifactCard addLabel={labels.artifactAddOutputLabel} artifacts={selectedOutputArtifacts} highlightedArtifactKeys={highlightedArtifactKeys} labels={labels} mode="output" onAddArtifact={() => addArtifact("output_artifacts")} onRemoveArtifact={(index) => removeArtifact("output_artifacts", index)} onUpdateArtifact={(index, updater) => updateArtifact("output_artifacts", index, updater)} focusedArtifactKey={focusedArtifactKey} selectedNodes={selectedNodes} title={labels.outputArtifactsTitle} />
    </section>
  );
}
