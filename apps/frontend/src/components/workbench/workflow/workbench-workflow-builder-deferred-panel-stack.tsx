"use client";

import type {
  Dispatch,
  SetStateAction,
} from "react";
import type {
  ProtocolAgentDescriptor,
  WorkflowCatalogEntry,
  WorkflowCatalogEntryArtifact,
  WorkflowDatasetAxis,
  WorkflowDatasetContract,
  WorkflowDatasetValueInfo,
  WorkflowGraphEdge,
  WorkflowGraphNode,
} from "@/lib/api";
import { WorkbenchWorkflowArtifactCard } from "@/components/workbench/workflow/workbench-workflow-artifact-card";
import { WorkbenchWorkflowDatasetCard } from "@/components/workbench/workflow/workbench-workflow-dataset-card";
import { WorkbenchWorkflowDiagnosticsPlane } from "@/components/workbench/workflow/workbench-workflow-diagnostics-plane";
import { WorkbenchWorkflowGraphSummaryCard } from "@/components/workbench/workflow/workbench-workflow-graph-summary-card";
import { WorkbenchWorkflowLocalMetadataCard } from "@/components/workbench/workflow/workbench-workflow-local-metadata-card";
import { WorkbenchWorkflowSnapshotCard } from "@/components/workbench/workflow/workbench-workflow-snapshot-card";
import type { WorkflowIntegrityIssue, WorkflowIntegrityReport } from "@/components/workbench/workflow/workbench-workflow-integrity";
import type { WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";
import type { WorkflowPackageResidualRecord } from "@/components/workbench/workflow/workbench-workflow-package-install-report";
import type { WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import type { StoredWorkflowSnapshotSummary } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";
import type { WorkflowValidationFixSummaryEntry } from "@/components/workbench/workflow/workbench-workflow-validation-summary";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import type { WorkflowRunRecord, WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowAuditNavigationTarget } from "@/components/workbench/workflow/workbench-workflow-audit-targets";
import { parseWorkflowArtifactFocusKey } from "@/components/workbench/workflow/workbench-workflow-audit-targets";
import type { WorkflowBridgeRuntimeValidationIssue } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-validation";
import type { WorkbenchAuditTimelineEntry } from "@/lib/workbench/workbench-audit-timeline";

type AxisUpdater = (axis: WorkflowDatasetAxis) => WorkflowDatasetAxis;
type ArtifactUpdater = (artifact: WorkflowCatalogEntryArtifact) => WorkflowCatalogEntryArtifact;
type DatasetUpdater = (value: WorkflowDatasetValueInfo) => WorkflowDatasetValueInfo;

type WorkbenchWorkflowBuilderDeferredPanelStackProps = {
  labels: WorkflowSidebarLabels;
  workflow: WorkflowCatalogEntry;
  frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  importedPackage: WorkflowPackage | null;
  integrityReport: WorkflowIntegrityReport;
  latestRun?: WorkflowRunRecord | null;
  protocolAgents: ProtocolAgentDescriptor[];
  recentFixSummary: WorkflowValidationFixSummaryEntry[];
  snapshots: StoredWorkflowSnapshotSummary[];
  validationIssues: WorkflowGraphValidationIssue[];
  packageResiduals: WorkflowPackageResidualRecord[];
  importDiagnostics: WorkflowPackageImportDiagnostic[];
  selectedDatasetContract: WorkflowDatasetContract | null;
  selectedDatasetValue: WorkflowDatasetValueInfo | null;
  selectedDatasetValueId: string | null;
  selectedDatasetValues: WorkflowDatasetValueInfo[];
  selectedEdges: WorkflowGraphEdge[];
  selectedEntryInputs: WorkflowCatalogEntryArtifact[];
  selectedNodes: WorkflowGraphNode[];
  selectedOutputArtifacts: WorkflowCatalogEntryArtifact[];
  focusedArtifactKey: string | null;
  focusedDatasetValueId: string | null;
  focusedEdgeId: string | null;
  focusedNodeId: string | null;
  highlightDatasetEditor: boolean;
  highlightedArtifactKeys: string[];
  highlightedEdgeIds: string[];
  highlightedNodeIds: string[];
  traceFocusBranchNodeId?: string | null;
  traceFocusBranchOutputId?: string | null;
  addArtifact: (field: "entry_inputs" | "output_artifacts") => void;
  addDatasetAxis: () => void;
  addDatasetValue: () => void;
  removeArtifact: (field: "entry_inputs" | "output_artifacts", index: number) => void;
  removeDatasetAxis: (axisId: string) => void;
  removeSelectedDatasetValue: () => void;
  setSelectedDatasetValueId: Dispatch<SetStateAction<string | null>>;
  updateArtifact: (field: "entry_inputs" | "output_artifacts", index: number, updater: ArtifactUpdater) => void;
  updateDatasetAxis: (axisId: string, updater: AxisUpdater) => void;
  updateDatasetValue: (valueId: string, updater: DatasetUpdater) => void;
  updateEdgeDatasetValue: (edgeId: string, datasetValue: string) => void;
  updateNodePortDatasetValue: (nodeId: string, portId: string, direction: "inputs" | "outputs", datasetValue: string) => void;
  onApplyAllValidationFixes: () => void;
  onApplyValidationFix: (issueId: string) => void;
  onDeleteSnapshot: (snapshotId: string) => void;
  onExportPackageInstallReport: (history: Array<{ at: string; kind: "scan" | "repair"; lines: string[] }>) => void;
  onLocateAuditTarget: (target: WorkflowAuditNavigationTarget) => void;
  onLocateBridgeRuntimeIssue: (issue: WorkflowBridgeRuntimeValidationIssue) => void;
  onLocateImportDiagnostic: (diagnostic: WorkflowPackageImportDiagnostic) => void;
  onLocateIntegrityIssue: (issue: WorkflowIntegrityIssue) => void;
  onLocatePackageResidual: (residualId: string) => void;
  onLocateValidationIssue: (issueId: string) => void;
  onReplayAuditEntry: (entry: WorkbenchAuditTimelineEntry) => void;
  onRepairPackageResidual: (residualId: string) => string[];
  onRestoreSnapshot: (snapshotId: string) => void;
  onSaveLocalMetadata: (summary: string, notes: string) => void;
  onScanPackageResiduals: () => string[];
};

export function WorkbenchWorkflowBuilderDeferredPanelStack({
  labels,
  workflow,
  frontendRuntimeMode,
  importedPackage,
  integrityReport,
  latestRun,
  protocolAgents,
  recentFixSummary,
  snapshots,
  validationIssues,
  packageResiduals,
  importDiagnostics,
  selectedDatasetContract,
  selectedDatasetValue,
  selectedDatasetValueId,
  selectedDatasetValues,
  selectedEdges,
  selectedEntryInputs,
  selectedNodes,
  selectedOutputArtifacts,
  focusedArtifactKey,
  focusedDatasetValueId,
  focusedEdgeId,
  focusedNodeId,
  highlightDatasetEditor,
  highlightedArtifactKeys,
  highlightedEdgeIds,
  highlightedNodeIds,
  traceFocusBranchNodeId,
  traceFocusBranchOutputId,
  addArtifact,
  addDatasetAxis,
  addDatasetValue,
  removeArtifact,
  removeDatasetAxis,
  removeSelectedDatasetValue,
  setSelectedDatasetValueId,
  updateArtifact,
  updateDatasetAxis,
  updateDatasetValue,
  updateEdgeDatasetValue,
  updateNodePortDatasetValue,
  onApplyAllValidationFixes,
  onApplyValidationFix,
  onDeleteSnapshot,
  onExportPackageInstallReport,
  onLocateAuditTarget,
  onLocateBridgeRuntimeIssue,
  onLocateImportDiagnostic,
  onLocateIntegrityIssue,
  onLocatePackageResidual,
  onLocateValidationIssue,
  onReplayAuditEntry,
  onRepairPackageResidual,
  onRestoreSnapshot,
  onSaveLocalMetadata,
  onScanPackageResiduals,
}: WorkbenchWorkflowBuilderDeferredPanelStackProps) {
  return (
    <>
      <WorkbenchWorkflowDiagnosticsPlane auditFocusHint={{ nodeId: focusedNodeId, edgeId: focusedEdgeId, branchNodeId: traceFocusBranchNodeId, branchOutputId: traceFocusBranchOutputId, datasetValueId: focusedDatasetValueId, ...parseWorkflowArtifactFocusKey(focusedArtifactKey) }} frontendRuntimeMode={frontendRuntimeMode} importedPackage={importedPackage} integrityReport={integrityReport} labels={labels} latestRun={latestRun} onApplyAllValidationFixes={onApplyAllValidationFixes} onApplyValidationFix={onApplyValidationFix} onExportPackageInstallReport={onExportPackageInstallReport} onLocateAuditTarget={onLocateAuditTarget} onLocateBridgeRuntimeIssue={onLocateBridgeRuntimeIssue} onLocateIntegrityIssue={onLocateIntegrityIssue} onLocatePackageResidual={onLocatePackageResidual} onLocateImportDiagnostic={onLocateImportDiagnostic} onLocateValidationIssue={onLocateValidationIssue} onReplayAuditEntry={onReplayAuditEntry} onRepairPackageResidual={onRepairPackageResidual} onScanPackageResiduals={onScanPackageResiduals} importDiagnostics={importDiagnostics} packageResiduals={packageResiduals} protocolAgents={protocolAgents} recentFixSummary={recentFixSummary} snapshotCount={snapshots.length} validationIssues={validationIssues} workflow={workflow} />
      {workflow.local ? <WorkbenchWorkflowLocalMetadataCard labels={labels} onSave={onSaveLocalMetadata} workflow={workflow} /> : null}
      <WorkbenchWorkflowSnapshotCard labels={labels} onDeleteSnapshot={onDeleteSnapshot} onRestoreSnapshot={onRestoreSnapshot} snapshots={snapshots} />
      <WorkbenchWorkflowGraphSummaryCard focusedEdgeId={focusedEdgeId} focusedNodeId={focusedNodeId} highlightedEdgeIds={highlightedEdgeIds} highlightedNodeIds={highlightedNodeIds} labels={labels} selectedEdges={selectedEdges} selectedEntryInputsCount={selectedEntryInputs.length} selectedNodes={selectedNodes} selectedOutputArtifactsCount={selectedOutputArtifacts.length} />
      <WorkbenchWorkflowDatasetCard addDatasetAxis={addDatasetAxis} addDatasetValue={addDatasetValue} labels={labels} removeDatasetAxis={removeDatasetAxis} removeSelectedDatasetValue={removeSelectedDatasetValue} selectedDatasetContract={selectedDatasetContract} selectedDatasetValue={selectedDatasetValue} selectedDatasetValueId={selectedDatasetValueId} selectedDatasetValues={selectedDatasetValues} selectedEdges={selectedEdges} focusedDatasetValueId={focusedDatasetValueId} highlightDatasetEditor={highlightDatasetEditor} selectedNodes={selectedNodes} setSelectedDatasetValueId={setSelectedDatasetValueId} updateDatasetAxis={updateDatasetAxis} updateDatasetValue={updateDatasetValue} updateEdgeDatasetValue={updateEdgeDatasetValue} updateNodePortDatasetValue={updateNodePortDatasetValue} />
      <WorkbenchWorkflowArtifactCard addLabel={labels.artifactAddEntryLabel} artifacts={selectedEntryInputs} highlightedArtifactKeys={highlightedArtifactKeys} labels={labels} mode="entry" onAddArtifact={() => addArtifact("entry_inputs")} onRemoveArtifact={(index) => removeArtifact("entry_inputs", index)} onUpdateArtifact={(index, updater) => updateArtifact("entry_inputs", index, updater)} focusedArtifactKey={focusedArtifactKey} selectedNodes={selectedNodes} title={labels.entryInputsTitle} />
      <WorkbenchWorkflowArtifactCard addLabel={labels.artifactAddOutputLabel} artifacts={selectedOutputArtifacts} highlightedArtifactKeys={highlightedArtifactKeys} labels={labels} mode="output" onAddArtifact={() => addArtifact("output_artifacts")} onRemoveArtifact={(index) => removeArtifact("output_artifacts", index)} onUpdateArtifact={(index, updater) => updateArtifact("output_artifacts", index, updater)} focusedArtifactKey={focusedArtifactKey} selectedNodes={selectedNodes} title={labels.outputArtifactsTitle} />
    </>
  );
}
