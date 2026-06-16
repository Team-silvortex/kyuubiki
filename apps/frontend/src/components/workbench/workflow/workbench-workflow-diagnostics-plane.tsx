"use client";

import type { ProtocolAgentDescriptor, WorkflowCatalogEntry } from "@/lib/api";
import { WorkbenchWorkflowActivityLogCard } from "@/components/workbench/workflow/workbench-workflow-activity-log-card";
import { WorkbenchWorkflowBridgeRuntimeCard } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-card";
import { summarizeWorkflowBridgeRuntimeStatuses, type WorkflowBridgeRuntimeValidationIssue } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-validation";
import { WorkbenchWorkflowControlFlowHistoryCard } from "@/components/workbench/workflow/workbench-workflow-control-flow-history-card";
import { WorkbenchWorkflowIntegrityCard } from "@/components/workbench/workflow/workbench-workflow-integrity-card";
import type { WorkflowIntegrityIssue, WorkflowIntegrityReport } from "@/components/workbench/workflow/workbench-workflow-integrity";
import { WorkbenchWorkflowPackageInstallCard } from "@/components/workbench/workflow/workbench-workflow-package-install-card";
import { WorkbenchWorkflowPackageImportDiagnosticsCard } from "@/components/workbench/workflow/workbench-workflow-package-import-diagnostics-card";
import type { WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";
import type { WorkflowPackageResidualRecord } from "@/components/workbench/workflow/workbench-workflow-package-install-report";
import type { WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import type { WorkflowValidationFixSummaryEntry } from "@/components/workbench/workflow/workbench-workflow-validation-summary";
import { WorkbenchWorkflowValidationCard } from "@/components/workbench/workflow/workbench-workflow-validation-card";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import type { WorkflowRunRecord, WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowAuditFocusHint, WorkflowAuditNavigationTarget } from "@/components/workbench/workflow/workbench-workflow-audit-targets";
import type { WorkbenchAuditTimelineEntry } from "@/lib/workbench/workbench-audit-timeline";
import { readWorkbenchAuditTimeline } from "@/lib/workbench/workbench-audit-timeline";

type WorkbenchWorkflowDiagnosticsPlaneProps = {
  labels: WorkflowSidebarLabels;
  workflow: WorkflowCatalogEntry;
  protocolAgents: ProtocolAgentDescriptor[];
  frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  importedPackage: WorkflowPackage | null;
  latestRun?: WorkflowRunRecord | null;
  validationIssues: WorkflowGraphValidationIssue[];
  recentFixSummary: WorkflowValidationFixSummaryEntry[];
  integrityReport: WorkflowIntegrityReport;
  packageResiduals: WorkflowPackageResidualRecord[];
  importDiagnostics: WorkflowPackageImportDiagnostic[];
  snapshotCount: number;
  onApplyAllValidationFixes: () => void;
  onApplyValidationFix: (issueId: string) => void;
  onLocateBridgeRuntimeIssue: (issue: WorkflowBridgeRuntimeValidationIssue) => void;
  onLocateValidationIssue: (issueId: string) => void;
  onLocateIntegrityIssue: (issue: WorkflowIntegrityIssue) => void;
  onExportPackageInstallReport: (history: Array<{ at: string; kind: "scan" | "repair"; lines: string[] }>) => void;
  onScanPackageResiduals: () => string[];
  onRepairPackageResidual: (residualId: string) => string[];
  onLocatePackageResidual: (residualId: string) => void;
  onLocateImportDiagnostic: (diagnostic: WorkflowPackageImportDiagnostic) => void;
  onLocateAuditTarget: (target: WorkflowAuditNavigationTarget) => void;
  onReplayAuditEntry: (entry: WorkbenchAuditTimelineEntry) => void;
  auditFocusHint?: WorkflowAuditFocusHint | null;
};

export function WorkbenchWorkflowDiagnosticsPlane({
  labels,
  workflow,
  protocolAgents,
  frontendRuntimeMode,
  importedPackage,
  latestRun,
  validationIssues,
  recentFixSummary,
  integrityReport,
  packageResiduals,
  importDiagnostics,
  snapshotCount,
  onApplyAllValidationFixes,
  onApplyValidationFix,
  onLocateBridgeRuntimeIssue,
  onLocateValidationIssue,
  onLocateIntegrityIssue,
  onExportPackageInstallReport,
  onScanPackageResiduals,
  onRepairPackageResidual,
  onLocatePackageResidual,
  onLocateImportDiagnostic,
  onLocateAuditTarget,
  onReplayAuditEntry,
  auditFocusHint,
}: WorkbenchWorkflowDiagnosticsPlaneProps) {
  const activityLogEntries = readWorkbenchAuditTimeline(workflow.id, 8, {
    frontendRuntimeMode,
    protocolAgents,
  });
  const bridgeRuntimeSummary = summarizeWorkflowBridgeRuntimeStatuses(workflow.graph ?? null, latestRun?.result ?? null);
  const controlFlowHistoryEntries = activityLogEntries.filter((entry) => entry.kind.startsWith("control_flow_"));
  return (
    <section className="workflow-diagnostics-plane">
      <div className="workflow-diagnostics-plane__summary sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.validationTitle}</span>
          <strong>{validationIssues.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>Component integrity</span>
          <strong>{integrityReport.issues.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>Bridge runtime</span>
          <strong>{latestRun?.result ? `${bridgeRuntimeSummary.aligned}/${bridgeRuntimeSummary.drift}/${bridgeRuntimeSummary["missing-runtime"]}` : "--"}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageInstallRulesResidualsLabel}</span>
          <strong>{packageResiduals.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>Package import diagnostics</span>
          <strong>{importDiagnostics.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>Activity log</span>
          <strong>{activityLogEntries.length}</strong>
        </div>
      </div>
      <div className="workflow-diagnostics-plane__cards">
        <WorkbenchWorkflowValidationCard
          labels={labels}
          onApplyAllValidationFixes={onApplyAllValidationFixes}
          onApplyValidationFix={onApplyValidationFix}
          onLocateValidationIssue={onLocateValidationIssue}
          recentFixSummary={recentFixSummary}
          validationIssues={validationIssues}
        />
        <WorkbenchWorkflowBridgeRuntimeCard
          graph={workflow.graph}
          onLocateIssue={onLocateBridgeRuntimeIssue}
          result={latestRun?.result ?? null}
        />
        <WorkbenchWorkflowIntegrityCard onLocateIssue={onLocateIntegrityIssue} report={integrityReport} />
        <WorkbenchWorkflowControlFlowHistoryCard entries={controlFlowHistoryEntries} onLocateTarget={onLocateAuditTarget} onReplayEntry={onReplayAuditEntry} />
        <WorkbenchWorkflowPackageInstallCard
          importedPackage={importedPackage}
          labels={labels}
          onExportReport={onExportPackageInstallReport}
          onLocateResidual={onLocatePackageResidual}
          onRepairResidual={onRepairPackageResidual}
          onScanResiduals={onScanPackageResiduals}
          residuals={packageResiduals}
          snapshotCount={snapshotCount}
          summaryOnlySnapshotCount={integrityReport.summaryOnlySnapshotCount}
          workflow={workflow}
        />
        <WorkbenchWorkflowPackageImportDiagnosticsCard diagnostics={importDiagnostics} onLocateDiagnostic={onLocateImportDiagnostic} />
        <WorkbenchWorkflowActivityLogCard auditFocusHint={auditFocusHint} entries={activityLogEntries} onLocateTarget={onLocateAuditTarget} protocolAgents={protocolAgents} workflowId={workflow.id} />
      </div>
    </section>
  );
}
