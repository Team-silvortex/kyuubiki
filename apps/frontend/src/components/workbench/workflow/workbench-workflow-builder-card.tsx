"use client";
import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import type { ProtocolAgentDescriptor, WorkflowCatalogEntryArtifact, WorkflowDatasetAxis, WorkflowCatalogEntry, WorkflowDatasetValueInfo, WorkflowGraphDefinition, WorkflowOperatorDescriptor } from "@/lib/api";
import type { HeatPlaneStudyJobInput, PlaneStudyJobInput, StudyKind } from "@/components/workbench/workbench-types";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import { asWorkflowDatasetContract, countWorkflowBridgeNormalizationAdjustments, mergeDatasetContractIntoGraph, normalizeImportedWorkflowGraph, readJsonFile } from "@/components/workbench/workflow/workbench-workflow-builder-import";
import { listStoredWorkflowDrafts, removeStoredWorkflowDraft, saveStoredWorkflowDraft, type StoredWorkflowDraft } from "@/components/workbench/workflow/workbench-workflow-draft-storage";
import { duplicateStoredLocalWorkflow, removeStoredLocalWorkflow, renameStoredLocalWorkflow, saveStoredLocalWorkflow, updateStoredLocalWorkflowMetadata } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { type WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import { buildExportedWorkflowPackage, buildPromotedWorkflowParams, parseImportedWorkflowPayload, type WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";
import { WorkbenchWorkflowDraftCard } from "@/components/workbench/workflow/workbench-workflow-draft-card"; import { WorkbenchWorkflowBuilderToolbar } from "@/components/workbench/workflow/workbench-workflow-builder-toolbar";
import { buildWorkflowInputArtifactTexts, parseWorkflowInputArtifactTexts } from "@/components/workbench/workflow/workbench-workflow-input-artifacts";
import { WorkbenchWorkflowInputArtifactsCard } from "@/components/workbench/workflow/workbench-workflow-input-artifacts-card";
import { WorkbenchWorkflowArtifactCard } from "@/components/workbench/workflow/workbench-workflow-artifact-card";
import { buildDraftArtifact, buildDraftDatasetValue, cloneWorkflowGraph, downloadJsonArtifact, ensureDatasetContract, slugifyWorkflowAssetName } from "@/components/workbench/workflow/workbench-workflow-builder-utils";
import { createWorkflowTopologyActions } from "@/components/workbench/workflow/workbench-workflow-topology-actions";
import { applyAllWorkflowValidationFixes, applyWorkflowValidationFix, validateWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import { buildWorkflowValidationHighlightPlan } from "@/components/workbench/workflow/workbench-workflow-validation-highlights"; import { buildWorkflowValidationFixSummary, type WorkflowValidationFixSummaryEntry } from "@/components/workbench/workflow/workbench-workflow-validation-summary";
import { useWorkflowBuilderFocus } from "@/components/workbench/workflow/workbench-workflow-builder-focus";
import { WorkbenchWorkflowFocusStrip } from "@/components/workbench/workflow/workbench-workflow-focus-strip";
import { useWorkflowPolicyFeedback } from "@/components/workbench/workflow/workbench-workflow-policy-feedback";
import { listStoredWorkflowSnapshots, loadStoredWorkflowSnapshot, removeStoredWorkflowSnapshot, removeStoredWorkflowSnapshotsByWorkflowId, removeStoredWorkflowSummaryOnlySnapshots, saveStoredWorkflowSnapshot, type StoredWorkflowSnapshotSummary } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";
import { describeWorkflowNodeTemplateSyncImpact, getWorkflowNodeTemplateSyncImpact, listAutoReconnectEdgeIds } from "@/components/workbench/workflow/workbench-workflow-template-impact";
import { WorkbenchWorkflowDiagnosticsPlane } from "@/components/workbench/workflow/workbench-workflow-diagnostics-plane";
import { WorkbenchWorkflowDatasetCard } from "@/components/workbench/workflow/workbench-workflow-dataset-card";
import { WorkbenchWorkflowControlFlowPlaneCard } from "@/components/workbench/workflow/workbench-workflow-control-flow-plane-card"; import { WorkbenchWorkflowGraphSummaryCard } from "@/components/workbench/workflow/workbench-workflow-graph-summary-card"; import { flashWorkflowFixReceiptHighlights } from "@/components/workbench/workflow/workbench-workflow-fix-receipt-flash";
import { buildWorkflowIntegrityReport, type WorkflowIntegrityIssue } from "@/components/workbench/workflow/workbench-workflow-integrity";
import { WorkbenchWorkflowLocalMetadataCard } from "@/components/workbench/workflow/workbench-workflow-local-metadata-card"; import { WorkbenchWorkflowPackageManifestCard } from "@/components/workbench/workflow/workbench-workflow-package-manifest-card";
import { buildWorkflowPackageInstallReport, scanWorkflowPackageResiduals } from "@/components/workbench/workflow/workbench-workflow-package-install-report";
import { WorkbenchWorkflowSnapshotCard } from "@/components/workbench/workflow/workbench-workflow-snapshot-card"; import { builtInWorkflowSampleInputArtifacts } from "@/components/workbench/workflow/workbench-workflow-sample-inputs"; import { WorkbenchWorkflowTopologyCard } from "@/components/workbench/workflow/workbench-workflow-topology-card"; import { buildWorkflowAuditReplayPlan } from "@/components/workbench/workflow/workbench-workflow-audit-replay";
import { readWorkflowTemplateChainPreferences, writeWorkflowTemplateChainPreferences } from "@/components/workbench/workflow/workbench-workflow-template-chain-storage"; import { locateWorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-import-diagnostic-locate";
import { buildControlFlowEdgeAuditPayload, buildControlFlowNodeAddAuditPayload, buildControlFlowPlaneInsertAuditPayload } from "@/components/workbench/workflow/workbench-workflow-control-flow-audit"; import { buildWorkflowAuditContextFromImportDiagnostic, buildWorkflowAuditContextFromValidationIssue, parseWorkflowArtifactFocusKey, type WorkflowAuditNavigationTarget } from "@/components/workbench/workflow/workbench-workflow-audit-targets";
import { buildImportedWorkflowContractHealthMessage, buildWorkflowDraftContractWarningMessage, countWorkflowContractWarnings } from "@/components/workbench/workflow/workbench-workflow-contract-health"; import { collectWorkflowInputArtifactContractWarnings } from "@/components/workbench/workflow/workbench-workflow-fem-validation"; import { appendWorkflowActivityLogEntry, buildWorkflowActivityCountSummary } from "@/lib/workbench/workflow-activity-log"; const EMPTY_LIST: never[] = [];
type WorkbenchWorkflowBuilderCardProps = {
  labels: WorkflowSidebarLabels;
  selectedWorkflow: WorkflowCatalogEntry | null;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: HeatPlaneStudyJobInput; currentPlaneModel: PlaneStudyJobInput; recentRunStatus?: string | null; protocolAgents?: ProtocolAgentDescriptor[]; frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  onRefreshWorkflowCatalog: () => void;
  onRunWorkflowCatalog: (workflowId: string) => void;
  onRunWorkflowDraft: (workflowId: string, graph: WorkflowGraphDefinition, inputArtifacts: Record<string, unknown>) => void; traceFocusNodeId?: string | null; traceFocusToken?: number; traceFocusBranchNodeId?: string | null; traceFocusBranchOutputId?: string | null; traceFocusBranchToken?: number; traceFocusDatasetNodeId?: string | null; traceFocusDatasetPortId?: string | null; traceFocusDatasetToken?: number; onLocateAuditTarget?: (target: WorkflowAuditNavigationTarget) => void;
}; export function WorkbenchWorkflowBuilderCard({
  labels,
  selectedWorkflow,
  operatorDescriptors,
  currentStudyKind,
  currentHeatPlaneModel,
  currentPlaneModel,
  recentRunStatus,
  protocolAgents = EMPTY_LIST as unknown as ProtocolAgentDescriptor[],
  frontendRuntimeMode,
  onRefreshWorkflowCatalog,
  onRunWorkflowCatalog,
  onRunWorkflowDraft,
  traceFocusNodeId,
  traceFocusToken,
  traceFocusBranchNodeId,
  traceFocusBranchOutputId,
  traceFocusBranchToken,
  traceFocusDatasetNodeId,
  traceFocusDatasetPortId, traceFocusDatasetToken, onLocateAuditTarget,
}: WorkbenchWorkflowBuilderCardProps) { const [draftGraph, setDraftGraph] = useState<WorkflowGraphDefinition | null>(null), [draftInputTexts, setDraftInputTexts] = useState<Record<string, string>>({}), [selectedDatasetValueId, setSelectedDatasetValueId] = useState<string | null>(null);
  const [importMessage, setImportMessage] = useState<string | null>(null), [importDiagnostics, setImportDiagnostics] = useState<WorkflowPackageImportDiagnostic[]>([]), [savedDrafts, setSavedDrafts] = useState<StoredWorkflowDraft[]>([]), [savedSnapshots, setSavedSnapshots] = useState<StoredWorkflowSnapshotSummary[]>([]), [recentFixSummary, setRecentFixSummary] = useState<WorkflowValidationFixSummaryEntry[]>([]);
  const [focusedNodeId, setFocusedNodeId] = useState<string | null>(null), [focusedEdgeId, setFocusedEdgeId] = useState<string | null>(null), [focusedArtifactKey, setFocusedArtifactKey] = useState<string | null>(null), [focusedDatasetValueId, setFocusedDatasetValueId] = useState<string | null>(null);
  const [highlightedNodeIds, setHighlightedNodeIds] = useState<string[]>([]), [highlightedEdgeIds, setHighlightedEdgeIds] = useState<string[]>([]), [highlightedArtifactKeys, setHighlightedArtifactKeys] = useState<string[]>([]), [highlightedPortKeys, setHighlightedPortKeys] = useState<string[]>([]), [highlightDatasetEditor, setHighlightDatasetEditor] = useState(false), [importedPackage, setImportedPackage] = useState<WorkflowPackage | null>(null), [showDeferredPanels, setShowDeferredPanels] = useState(false);
  const graphInputRef = useRef<HTMLInputElement | null>(null), datasetInputRef = useRef<HTMLInputElement | null>(null), builderRootRef = useRef<HTMLElement | null>(null);
  const activeFocusTarget = useWorkflowBuilderFocus(builderRootRef);
  const { policyFeedback, setPolicyFeedback } = useWorkflowPolicyFeedback();
  function resetBuilderFocus() { setFocusedNodeId(null); setFocusedEdgeId(null); setFocusedArtifactKey(null); setFocusedDatasetValueId(null); setHighlightedNodeIds([]); setHighlightedEdgeIds([]); setHighlightedArtifactKeys([]); setHighlightedPortKeys([]); setHighlightDatasetEditor(false); }
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
    setHighlightedPortKeys([]);
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
      setHighlightedPortKeys((current) => (current.length > 0 ? [] : current));
      setHighlightDatasetEditor((current) => (current && plan.highlightDatasetEditor ? false : current));
    }, 2200);
  }
  function flashFixReceiptHighlights(summary: WorkflowValidationFixSummaryEntry[]) { flashWorkflowFixReceiptHighlights({ builderRootRef, summary, setFocusedNodeId, setFocusedEdgeId, setFocusedArtifactKey, setHighlightedNodeIds, setHighlightedEdgeIds, setHighlightedPortKeys, setHighlightedArtifactKeys }); }
  useEffect(() => {
    const normalizedSelectedGraph = normalizeImportedWorkflowGraph(
      cloneWorkflowGraph(selectedWorkflow?.graph ?? null),
      operatorDescriptors ?? [],
    ).graph;
    const nextDraft = cloneWorkflowGraph(normalizedSelectedGraph);
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
    setImportMessage(null); setImportDiagnostics([]);
    setImportedPackage(null);
    setRecentFixSummary([]);
    setPolicyFeedback(null);
    resetBuilderFocus();
    setSavedDrafts(selectedWorkflow ? listStoredWorkflowDrafts(selectedWorkflow.id) : []);
    setSavedSnapshots(selectedWorkflow ? listStoredWorkflowSnapshots(selectedWorkflow.id) : []);
  }, [operatorDescriptors, selectedWorkflow]);
  useEffect(() => { setShowDeferredPanels(false); const handle = window.setTimeout(() => setShowDeferredPanels(true), 140); return () => window.clearTimeout(handle); }, [selectedWorkflow?.id]);
  useEffect(() => { if (!traceFocusNodeId) return; setFocusedNodeId(traceFocusNodeId); setFocusedEdgeId(null); setHighlightedNodeIds([traceFocusNodeId]); queueMicrotask(() => builderRootRef.current?.querySelector<HTMLElement>(`[data-workflow-node-id="${traceFocusNodeId}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" })); window.setTimeout(() => setHighlightedNodeIds((current) => (current[0] === traceFocusNodeId ? [] : current)), 2200); }, [traceFocusNodeId, traceFocusToken]);
  useEffect(() => { if (!traceFocusBranchNodeId || !traceFocusBranchOutputId) return; const graph = draftGraph; const branchEdges = (graph?.edges ?? []).filter((edge) => edge.from.node === traceFocusBranchNodeId && edge.from.port === traceFocusBranchOutputId); const mergeNode = branchEdges.map((edge) => graph?.nodes.find((node) => node.id === edge.to.node && node.operator_id === "transform.first_available") ?? null).find(Boolean) ?? null; const downstreamEdgeIds = mergeNode ? (graph?.edges ?? []).filter((edge) => edge.from.node === mergeNode.id && edge.from.port === "merged").map((edge) => edge.id) : []; setFocusedNodeId(traceFocusBranchNodeId); setHighlightedNodeIds(mergeNode ? [traceFocusBranchNodeId, mergeNode.id] : [traceFocusBranchNodeId]); if (branchEdges.length + downstreamEdgeIds.length > 0) flashHighlightedEdges([...branchEdges.map((edge) => edge.id), ...downstreamEdgeIds]); }, [draftGraph, traceFocusBranchNodeId, traceFocusBranchOutputId, traceFocusBranchToken]);
  useEffect(() => { if (!traceFocusDatasetNodeId || !traceFocusDatasetPortId) return; const valueId = draftGraph?.nodes.find((node) => node.id === traceFocusDatasetNodeId)?.outputs?.find((port) => port.id === traceFocusDatasetPortId)?.dataset_value ?? null; if (!valueId) return; const nodeIds = (draftGraph?.nodes ?? []).filter((node) => [...(node.inputs ?? []), ...(node.outputs ?? [])].some((port) => port.dataset_value === valueId)).map((node) => node.id); const edgeIds = (draftGraph?.edges ?? []).filter((edge) => edge.dataset_value === valueId).map((edge) => edge.id); setSelectedDatasetValueId(valueId); setFocusedDatasetValueId(valueId); setHighlightDatasetEditor(true); setHighlightedNodeIds(nodeIds); if (edgeIds.length > 0) setHighlightedEdgeIds(edgeIds); queueMicrotask(() => builderRootRef.current?.querySelector<HTMLElement>('[data-workflow-dataset-editor="editor"]')?.scrollIntoView({ block: "nearest", behavior: "smooth" })); window.setTimeout(() => { setHighlightDatasetEditor((current) => (current ? false : current)); setHighlightedNodeIds((current) => (current.join(",") === nodeIds.join(",") ? [] : current)); setHighlightedEdgeIds((current) => (current.join(",") === edgeIds.join(",") ? [] : current)); }, 2200); }, [draftGraph, traceFocusDatasetNodeId, traceFocusDatasetPortId, traceFocusDatasetToken]);
  const selectedGraph = draftGraph;
  const selectedNodes = selectedGraph?.nodes ?? EMPTY_LIST, selectedEdges = selectedGraph?.edges ?? EMPTY_LIST, selectedEntryInputs = selectedGraph?.entry_inputs ?? EMPTY_LIST, selectedOutputArtifacts = selectedGraph?.output_artifacts ?? EMPTY_LIST, selectedDatasetContract = selectedGraph?.dataset_contract ?? null, selectedDatasetValues = selectedDatasetContract?.values ?? EMPTY_LIST;
  const parsedDraftInputs = useMemo(() => parseWorkflowInputArtifactTexts(draftInputTexts), [draftInputTexts]);
  const inputArtifactWarnings = useMemo(() => collectWorkflowInputArtifactContractWarnings({ entryInputs: selectedEntryInputs, inputArtifactTexts: draftInputTexts }), [selectedEntryInputs, draftInputTexts]);
  const inputArtifactWarningCount = useMemo(() => countWorkflowContractWarnings(inputArtifactWarnings), [inputArtifactWarnings]);
  const validationIssues = useMemo(() => validateWorkflowGraphDefinition(selectedGraph, selectedEntryInputs, selectedOutputArtifacts, operatorDescriptors ?? []), [operatorDescriptors, selectedGraph, selectedEntryInputs, selectedOutputArtifacts]);
  const integrityReport = useMemo(() => buildWorkflowIntegrityReport(selectedWorkflow, operatorDescriptors ?? []), [operatorDescriptors, selectedWorkflow]);
  const packageResiduals = useMemo(() => selectedWorkflow ? scanWorkflowPackageResiduals({ workflow: selectedWorkflow, importedPackage, integrityReport }) : [], [importedPackage, integrityReport, selectedWorkflow]);
  const draftBlockingIssueCount = validationIssues.length + parsedDraftInputs.invalidKeys.length, canRunDraft = Boolean(selectedGraph) && draftBlockingIssueCount === 0;
  const selectedDatasetValue = useMemo(() => selectedDatasetValues.find((value) => value.id === selectedDatasetValueId) ?? selectedDatasetValues[0] ?? null, [selectedDatasetValueId, selectedDatasetValues]);
  const snapshotContractSummary = `contract warnings: ${inputArtifactWarningCount}`;
  const topologyActions = useMemo(() => createWorkflowTopologyActions(setDraftGraph, operatorDescriptors), [operatorDescriptors]);
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
    const { fix } = issue;
    if (fix.kind === "sync_node_template_from_operator" && selectedGraph) {
      const currentNode = selectedGraph.nodes.find((node) => node.id === fix.nodeId);
      const impact = getWorkflowNodeTemplateSyncImpact(
        selectedGraph,
        fix.nodeId,
        {
          kind: fix.templateKind,
          operatorId: fix.operatorId,
          config:
            currentNode?.config && typeof currentNode.config === "object"
              ? { ...(currentNode.config as Record<string, unknown>) }
              : undefined,
        },
        operatorDescriptors ?? [],
      );
      const preview = describeWorkflowNodeTemplateSyncImpact(impact);
      if (preview && !window.confirm(preview)) return;
      flashHighlightedEdges(listAutoReconnectEdgeIds(impact));
    }
    const nextGraph = applyWorkflowValidationFix(selectedGraph, issue, operatorDescriptors ?? []);
    if (selectedWorkflow && nextGraph) {
      const summary = buildWorkflowValidationFixSummary([issue], selectedGraph, operatorDescriptors ?? []);
      saveStoredWorkflowSnapshot({ workflowId: selectedWorkflow.id, workflowName: selectedWorkflow.name, reason: "single validation fix", graph: nextGraph, inputArtifactTexts: draftInputTexts, summary: [...summary.map((entry) => entry.title), snapshotContractSummary, buildWorkflowActivityCountSummary(countWorkflowBridgeNormalizationAdjustments(nextGraph), "bridge auto-fixes")] }); appendWorkflowActivityLogEntry({ workflowId: selectedWorkflow.id, kind: "validation_fix_applied", message: "Applied single workflow validation fix.", detail: issue.message, count: 1, context: buildWorkflowAuditContextFromValidationIssue(issue, selectedGraph) });
      setSavedSnapshots(listStoredWorkflowSnapshots(selectedWorkflow.id));
      setRecentFixSummary(summary);
      flashFixReceiptHighlights(summary);
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
    const summary = buildWorkflowValidationFixSummary(appliedIssues, selectedGraph, operatorDescriptors ?? []);
    if (selectedWorkflow && graph) {
      saveStoredWorkflowSnapshot({ workflowId: selectedWorkflow.id, workflowName: selectedWorkflow.name, reason: "batch validation fixes", graph, inputArtifactTexts: draftInputTexts, summary: [...summary.map((entry) => entry.title), snapshotContractSummary, buildWorkflowActivityCountSummary(countWorkflowBridgeNormalizationAdjustments(graph), "bridge auto-fixes")] }); appendWorkflowActivityLogEntry({ workflowId: selectedWorkflow.id, kind: "validation_fix_applied", message: "Applied batch workflow validation fixes.", count: appliedIssues.length, context: buildWorkflowAuditContextFromValidationIssue(appliedIssues[0]!, selectedGraph) });
      setSavedSnapshots(listStoredWorkflowSnapshots(selectedWorkflow.id));
    }
    setDraftGraph(graph);
    setRecentFixSummary(summary);
    flashValidationHighlights(graph, appliedIssues);
    flashFixReceiptHighlights(summary);
    const firstFixedMessage = summary[0]?.title ?? appliedIssues[0]?.message;
    setImportMessage([labels.validationAutoFixedLabel.replace("{count}", String(appliedCount)), firstFixedMessage].filter(Boolean).join(" "));
  }
  function locateValidationIssue(issueId: string) { const issue = validationIssues.find((entry) => entry.id === issueId); if (issue?.locate) locateBuilderIssue(issue.locate); }
  function locateIntegrityIssue(issue: WorkflowIntegrityIssue) { if (issue.locate) locateBuilderIssue(issue.locate); }
  function locateAuditTarget(target: WorkflowAuditNavigationTarget) { if (target.kind === "run") { onLocateAuditTarget?.(target); return; } locateBuilderIssue(target.kind === "node" ? { kind: "node", nodeId: target.nodeId } : { kind: "dataset", datasetValueId: target.datasetValueId }); }
  function replayAuditEntry(entry: import("@/lib/workbench/workbench-audit-timeline").WorkbenchAuditTimelineEntry) { const plan = buildWorkflowAuditReplayPlan(entry); if (plan.nodeId) setFocusedNodeId(plan.nodeId); if (plan.edgeIds[0]) setFocusedEdgeId(plan.edgeIds[0]); if (plan.nodeIds.length > 0) setHighlightedNodeIds(plan.nodeIds); if (plan.edgeIds.length > 0) flashHighlightedEdges(plan.edgeIds); window.setTimeout(() => setHighlightedNodeIds((current) => current.join(",") === plan.nodeIds.join(",") ? [] : current), 2200); }
  function appendControlFlowAudit(payload: ReturnType<typeof buildControlFlowEdgeAuditPayload> | ReturnType<typeof buildControlFlowNodeAddAuditPayload> | ReturnType<typeof buildControlFlowPlaneInsertAuditPayload>) { if (!selectedWorkflow) return; appendWorkflowActivityLogEntry({ workflowId: selectedWorkflow.id, kind: payload.kind, message: payload.message, detail: payload.detail, context: payload.context }); }
  function addControlFlowNode(kind: "condition" | "merge") { topologyActions.addNode(kind === "condition" ? { kind: "condition" } : { kind: "transform", operatorId: "transform.first_available" }); appendControlFlowAudit(buildControlFlowNodeAddAuditPayload(kind)); }
  function insertControlFlowPlaneWithAudit(sourceNodeId?: string | null) { topologyActions.insertControlFlowPlane(sourceNodeId); appendControlFlowAudit(buildControlFlowPlaneInsertAuditPayload(sourceNodeId)); }
  function setControlFlowEdgeWithAudit(mode: "outgoing" | "incoming", nodeId: string, portId: string, target: string) { topologyActions.setControlFlowEdge(mode, nodeId, portId, target); appendControlFlowAudit(buildControlFlowEdgeAuditPayload({ graph: selectedGraph, mode, nodeId, portId, target })); }
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
    if (!selectedGraph || !selectedWorkflow) return;
    const exportedPackage = buildExportedWorkflowPackage({
      workflow: selectedWorkflow,
      graph: cloneWorkflowGraph(selectedGraph)!,
      inputArtifactTexts: draftInputTexts,
      templateChainPreferences: readWorkflowTemplateChainPreferences(),
    });
    downloadJsonArtifact(`${slugifyWorkflowAssetName(selectedGraph.id)}.workflow-package.json`, exportedPackage);
    const warningCount = Object.values(exportedPackage.workflow.input_artifact_contract_warnings ?? {}).reduce(
      (total, lines) => total + lines.length,
      0,
    );
    setImportMessage(warningCount > 0 ? `Workflow package exported with ${warningCount} contract warning(s).` : "Workflow package exported.");
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
    setImportMessage(buildWorkflowDraftContractWarningMessage(inputArtifactWarningCount));
    onRunWorkflowDraft(selectedWorkflow.id, selectedGraph, parsedDraftInputs.inputArtifacts);
  }
  function promoteCurrentDraft() {
    if (!selectedWorkflow || !selectedGraph) return;
    saveStoredLocalWorkflow(
      buildPromotedWorkflowParams({
        workflow: selectedWorkflow,
        graph: selectedGraph,
        inputArtifactTexts: draftInputTexts,
        importedPackage,
      }),
    );
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
  function duplicateCurrentLocalWorkflow() { if (!selectedWorkflow?.local) return; duplicateStoredLocalWorkflow(selectedWorkflow.local.storage_id); onRefreshWorkflowCatalog(); setImportMessage(labels.localWorkflowDuplicatedLabel); }
  function deleteCurrentLocalWorkflow() { if (!selectedWorkflow?.local) return; removeStoredWorkflowSnapshotsByWorkflowId(selectedWorkflow.id); removeStoredLocalWorkflow(selectedWorkflow.local.storage_id); onRefreshWorkflowCatalog(); setImportMessage(labels.localWorkflowDeletedLabel); }
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
    setImportedPackage(null);
    setSelectedDatasetValueId(nextGraph?.dataset_contract?.values?.[0]?.id ?? null);
    resetBuilderFocus();
    setImportMessage(labels.draftLoadedLabel);
  }
  function deleteSavedDraft(draftId: string) { if (!selectedWorkflow) return; removeStoredWorkflowDraft(draftId); setSavedDrafts(listStoredWorkflowDrafts(selectedWorkflow.id)); setImportMessage(labels.draftDeletedLabel); }
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
    setImportedPackage(null);
    setSelectedDatasetValueId(nextGraph.dataset_contract?.values?.[0]?.id ?? null);
    setRecentFixSummary(snapshot.summary.filter((entry) => !entry.startsWith("contract warnings:")).map((entry, index) => ({ id: `${snapshot.id}:${index}`, title: entry, detail: "从快照恢复的历史修复摘要。", nodeIds: [], edgeIds: [], portKeys: [], artifactKeys: [] })));
    setImportMessage((() => { const parse = (summary?: string[]) => Number(summary?.find((entry) => entry.startsWith("contract warnings:"))?.split(":")[1]?.trim() ?? ""); const nextCount = parse(snapshot.summary), currentCount = parse(savedSnapshots[0]?.summary); const health = Number.isFinite(nextCount) && Number.isFinite(currentCount) && snapshot.id !== savedSnapshots[0]?.id ? nextCount > currentCount ? "restoring to a dirtier contract state" : nextCount < currentCount ? "restoring to a cleaner contract state" : "restoring to an equivalent contract state" : null; return Number.isFinite(nextCount) ? `${snapshot.reason} (${health ? `${health}; ` : ""}contract warnings: ${nextCount})` : snapshot.reason; })());
    resetBuilderFocus();
  }
  function deleteSnapshot(snapshotId: string) { if (!selectedWorkflow) return; removeStoredWorkflowSnapshot(snapshotId); setSavedSnapshots(listStoredWorkflowSnapshots(selectedWorkflow.id)); }
  function exportDraftDatasetContract() { if (!selectedDatasetContract) return; downloadJsonArtifact(`${slugifyWorkflowAssetName(selectedDatasetContract.id)}.workflow-dataset.json`, selectedDatasetContract); }
  function exportPackageInstallReport(maintenanceHistory: Array<{ at: string; kind: "scan" | "repair"; lines: string[] }>) { if (!selectedWorkflow) return; downloadJsonArtifact(`${slugifyWorkflowAssetName(selectedWorkflow.id)}.workflow-package-install-report.json`, buildWorkflowPackageInstallReport({ workflow: selectedWorkflow, importedPackage, integrityReport, recentRunStatus, protocolAgents, frontendRuntimeMode, maintenanceHistory })); setImportMessage(labels.packageInstallRulesReportExportedLabel); }
  function scanPackageResiduals() { const receipt = packageResiduals.length > 0 ? packageResiduals.map((entry) => entry.message) : [labels.packageInstallRulesResidualsCleanLabel]; if (selectedWorkflow) appendWorkflowActivityLogEntry({ workflowId: selectedWorkflow.id, kind: "package_residual_scanned", message: "Scanned workflow package residuals.", count: packageResiduals.length, detail: receipt[0] }); setImportMessage(receipt[0]); return receipt; }
  function locatePackageResidual(residualId: string) { const residual = packageResiduals.find((entry) => entry.id === residualId); if (!residual) return; if (residual.locate === "snapshot") return queueMicrotask(() => builderRootRef.current?.querySelector<HTMLElement>('[data-workflow-snapshot-card="card"]')?.scrollIntoView({ block: "nearest", behavior: "smooth" })); if (residual.locate === "local") return queueMicrotask(() => builderRootRef.current?.querySelector<HTMLElement>('[data-workflow-local-card="card"]')?.scrollIntoView({ block: "nearest", behavior: "smooth" })); queueMicrotask(() => builderRootRef.current?.querySelector<HTMLElement>('[data-workflow-package-card="card"], [data-workflow-package-policy-card="card"]')?.scrollIntoView({ block: "nearest", behavior: "smooth" })); }
  function locateImportDiagnostic(diagnostic: WorkflowPackageImportDiagnostic) { locateWorkflowPackageImportDiagnostic(builderRootRef.current, diagnostic, setSelectedDatasetValueId); }
  function applyResidualRepair(kind: (typeof packageResiduals)[number]["kind"]) { if (!selectedWorkflow) return [] as string[]; if (kind === "orphan_snapshots") { removeStoredWorkflowSnapshotsByWorkflowId(selectedWorkflow.id); setSavedSnapshots(listStoredWorkflowSnapshots(selectedWorkflow.id)); return ["Removed orphan workflow snapshots for the current workflow."]; } if (kind === "summary_only_snapshots") { removeStoredWorkflowSummaryOnlySnapshots(selectedWorkflow.id); setSavedSnapshots(listStoredWorkflowSnapshots(selectedWorkflow.id)); return ["Removed summary-only snapshots that could not be restored."]; } if (kind === "package_override") { const nextGraph = cloneWorkflowGraph(selectedWorkflow.graph ?? null); setDraftGraph(nextGraph); setDraftInputTexts(selectedWorkflow.local?.input_artifact_texts ?? buildWorkflowInputArtifactTexts(nextGraph?.entry_inputs ?? [], builtInWorkflowSampleInputArtifacts(selectedWorkflow.local?.source_workflow_id ?? selectedWorkflow.id))); setImportedPackage(null); setSelectedDatasetValueId(nextGraph?.dataset_contract?.values?.[0]?.id ?? null); resetBuilderFocus(); return ["Discarded the draft package override and restored the mounted workflow state."]; } return []; }
  function repairPackageResidual(residualId: string) { const residual = packageResiduals.find((entry) => entry.id === residualId); if (!residual?.auto_fixable) { setImportMessage(labels.packageInstallRulesRepairUnavailableLabel); return []; } const receipt = applyResidualRepair(residual.kind); if (receipt.length > 0) { if (selectedWorkflow) appendWorkflowActivityLogEntry({ workflowId: selectedWorkflow.id, kind: "package_residual_repaired", message: "Applied workflow package residual repair.", detail: residual.message, count: 1 }); setImportMessage(labels.packageInstallRulesRepairedLabel); } return receipt; }
  async function importWorkflowGraphFile(file: File) {
    try {
      const json = await readJsonFile(file);
      const importedPayload = parseImportedWorkflowPayload(json);
      if (!importedPayload) return void (setImportDiagnostics([]), setImportMessage(labels.importInvalidGraphLabel));
      if (importedPayload.error) return void (setImportDiagnostics(importedPayload.diagnostics ?? []), setImportMessage(`${labels.importInvalidGraphLabel} ${importedPayload.error}`));
      const { graph, importedPackage: nextImportedPackage, inputArtifactTexts, templateChainPreferences } = importedPayload;
      const imported = normalizeImportedWorkflowGraph(graph, operatorDescriptors ?? []);
      const nextGraph = cloneWorkflowGraph(imported.graph);
      if (nextGraph) {
        nextGraph.entry_inputs = nextGraph.entry_inputs ?? selectedEntryInputs;
        nextGraph.output_artifacts = nextGraph.output_artifacts ?? selectedOutputArtifacts;
      }
      setDraftGraph(nextGraph);
      setDraftInputTexts(
        inputArtifactTexts ??
          buildWorkflowInputArtifactTexts(
            nextGraph?.entry_inputs ?? [],
            selectedWorkflow ? builtInWorkflowSampleInputArtifacts(selectedWorkflow.id) : null,
          ),
      );
      if (templateChainPreferences) {
        writeWorkflowTemplateChainPreferences(templateChainPreferences);
      }
      setImportDiagnostics([...(importedPayload.diagnostics ?? []), ...imported.diagnostics]);
      setImportedPackage(nextImportedPackage);
      setSelectedDatasetValueId(nextGraph?.dataset_contract?.values?.[0]?.id ?? null);
      if (selectedWorkflow) appendWorkflowActivityLogEntry({ workflowId: selectedWorkflow.id, kind: "workflow_imported", message: "Imported workflow graph into the builder.", count: imported.diagnostics.length, detail: buildWorkflowActivityCountSummary(countWorkflowBridgeNormalizationAdjustments(nextGraph), "bridge auto-fixes"), context: imported.diagnostics[0] ? buildWorkflowAuditContextFromImportDiagnostic(imported.diagnostics[0]) : undefined });
      flashHighlightedEdges(imported.autoReconnectEdgeIds);
      setImportMessage(buildImportedWorkflowContractHealthMessage({ importSuccessLabel: labels.importSuccessLabel, currentWarnings: inputArtifactWarnings, importedWarnings: nextImportedPackage?.workflow.input_artifact_contract_warnings, hasImportedPackage: Boolean(nextImportedPackage) }));
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
      <WorkbenchWorkflowFocusStrip activeTarget={activeFocusTarget} feedback={policyFeedback} labels={labels} />
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
      <WorkbenchWorkflowPackageManifestCard importedPackage={importedPackage} labels={labels} recentRunStatus={recentRunStatus} workflow={selectedWorkflow} />
      <WorkbenchWorkflowInputArtifactsCard entryInputs={selectedEntryInputs} inputTexts={draftInputTexts} invalidKeys={parsedDraftInputs.invalidKeys} labels={labels} onChangeInputText={updateDraftInputText} selectedNodes={selectedNodes} />
      <WorkbenchWorkflowControlFlowPlaneCard labels={labels} operatorDescriptors={operatorDescriptors} selectedEdges={selectedEdges} selectedNodes={selectedNodes} validationIssues={validationIssues} invalidInputCount={parsedDraftInputs.invalidKeys.length} traceFocusBranchNodeId={traceFocusBranchNodeId} traceFocusBranchOutputId={traceFocusBranchOutputId} traceFocusBranchToken={traceFocusBranchToken} onAddConditionNode={() => addControlFlowNode("condition")} onAddMergeNode={() => addControlFlowNode("merge")} onAddNode={topologyActions.addNode} onSyncNodeTemplate={topologyActions.syncNodeTemplate} onInsertControlFlowPlane={insertControlFlowPlaneWithAudit} onSetControlFlowEdge={setControlFlowEdgeWithAudit} />
      <WorkbenchWorkflowTopologyCard currentHeatPlaneModel={currentHeatPlaneModel} currentPlaneModel={currentPlaneModel} currentStudyKind={currentStudyKind} focusedEdgeId={focusedEdgeId} focusedNodeId={focusedNodeId} highlightedNodeIds={highlightedNodeIds} highlightedPortKeys={highlightedPortKeys} labels={labels} operatorDescriptors={operatorDescriptors} onAddEdge={topologyActions.addEdge} onAddConnectedNode={topologyActions.addConnectedNode} onInsertTemplateChain={topologyActions.insertTemplateChain} onAddNode={topologyActions.addNode} onAddNodePort={topologyActions.addNodePort} onRemoveEdge={topologyActions.removeEdge} onRemoveNode={topologyActions.removeNode} onRemoveNodePort={topologyActions.removeNodePort} onSyncNodeTemplate={topologyActions.syncNodeTemplate} onUpdateEdge={topologyActions.updateEdge} onUpdateNode={topologyActions.updateNode} onUpdateNodePort={topologyActions.updateNodePort} highlightedEdgeIds={highlightedEdgeIds} selectedEdges={selectedEdges} selectedNodes={selectedNodes} />
      {showDeferredPanels ? <>
        <WorkbenchWorkflowDiagnosticsPlane auditFocusHint={{ nodeId: focusedNodeId, edgeId: focusedEdgeId, branchNodeId: traceFocusBranchNodeId, branchOutputId: traceFocusBranchOutputId, datasetValueId: focusedDatasetValueId, ...parseWorkflowArtifactFocusKey(focusedArtifactKey) }} frontendRuntimeMode={frontendRuntimeMode} importedPackage={importedPackage} integrityReport={integrityReport} labels={labels} onApplyAllValidationFixes={applyAllValidationFixes} onApplyValidationFix={applyValidationFix} onExportPackageInstallReport={exportPackageInstallReport} onLocateAuditTarget={locateAuditTarget} onLocateIntegrityIssue={locateIntegrityIssue} onLocatePackageResidual={locatePackageResidual} onLocateImportDiagnostic={locateImportDiagnostic} onLocateValidationIssue={locateValidationIssue} onReplayAuditEntry={replayAuditEntry} onRepairPackageResidual={repairPackageResidual} onScanPackageResiduals={scanPackageResiduals} importDiagnostics={importDiagnostics} packageResiduals={packageResiduals} protocolAgents={protocolAgents} recentFixSummary={recentFixSummary} snapshotCount={savedSnapshots.length} validationIssues={validationIssues} workflow={selectedWorkflow} />
        {selectedWorkflow.local ? <WorkbenchWorkflowLocalMetadataCard labels={labels} onSave={saveCurrentLocalWorkflowMetadata} workflow={selectedWorkflow} /> : null}
        <WorkbenchWorkflowSnapshotCard labels={labels} onDeleteSnapshot={deleteSnapshot} onRestoreSnapshot={restoreSnapshot} snapshots={savedSnapshots} />
        <WorkbenchWorkflowGraphSummaryCard focusedEdgeId={focusedEdgeId} focusedNodeId={focusedNodeId} highlightedEdgeIds={highlightedEdgeIds} highlightedNodeIds={highlightedNodeIds} labels={labels} selectedEdges={selectedEdges} selectedEntryInputsCount={selectedEntryInputs.length} selectedNodes={selectedNodes} selectedOutputArtifactsCount={selectedOutputArtifacts.length} />
        <WorkbenchWorkflowDatasetCard addDatasetAxis={addDatasetAxis} addDatasetValue={addDatasetValue} labels={labels} removeDatasetAxis={removeDatasetAxis} removeSelectedDatasetValue={removeSelectedDatasetValue} selectedDatasetContract={selectedDatasetContract} selectedDatasetValue={selectedDatasetValue} selectedDatasetValueId={selectedDatasetValueId} selectedDatasetValues={selectedDatasetValues} selectedEdges={selectedEdges} focusedDatasetValueId={focusedDatasetValueId} highlightDatasetEditor={highlightDatasetEditor} selectedNodes={selectedNodes} setSelectedDatasetValueId={setSelectedDatasetValueId} updateDatasetAxis={updateDatasetAxis} updateDatasetValue={updateDatasetValue} updateEdgeDatasetValue={updateEdgeDatasetValue} updateNodePortDatasetValue={updateNodePortDatasetValue} />
        <WorkbenchWorkflowArtifactCard addLabel={labels.artifactAddEntryLabel} artifacts={selectedEntryInputs} highlightedArtifactKeys={highlightedArtifactKeys} labels={labels} mode="entry" onAddArtifact={() => addArtifact("entry_inputs")} onRemoveArtifact={(index) => removeArtifact("entry_inputs", index)} onUpdateArtifact={(index, updater) => updateArtifact("entry_inputs", index, updater)} focusedArtifactKey={focusedArtifactKey} selectedNodes={selectedNodes} title={labels.entryInputsTitle} />
        <WorkbenchWorkflowArtifactCard addLabel={labels.artifactAddOutputLabel} artifacts={selectedOutputArtifacts} highlightedArtifactKeys={highlightedArtifactKeys} labels={labels} mode="output" onAddArtifact={() => addArtifact("output_artifacts")} onRemoveArtifact={(index) => removeArtifact("output_artifacts", index)} onUpdateArtifact={(index, updater) => updateArtifact("output_artifacts", index, updater)} focusedArtifactKey={focusedArtifactKey} selectedNodes={selectedNodes} title={labels.outputArtifactsTitle} />
      </> : null}
    </section>
  );
}
