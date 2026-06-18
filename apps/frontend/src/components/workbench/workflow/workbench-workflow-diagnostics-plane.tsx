"use client";

import { useEffect, useMemo, useRef, useState } from "react";
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

type WorkflowDiagnosticsFocusTarget =
  | "validation"
  | "validationAll"
  | "validationFixable"
  | "validationReview"
  | "integrity"
  | "bridge"
  | "bridgeAligned"
  | "bridgeDrift"
  | "bridgeMissingRuntime"
  | "packageResiduals"
  | "packageResidualsAll"
  | "packageResidualsAuto"
  | "packageResidualsManual"
  | "packageImport"
  | "packageImportAll"
  | "packageImportNode"
  | "packageImportDataset"
  | "packageImportPackage"
  | "activity";

function workflowDiagnosticsFocusRing(active: boolean) {
  return active
    ? {
        borderRadius: "0.9rem",
        boxShadow: "0 0 0 1px rgba(96, 165, 250, 0.55), 0 0 0 4px rgba(59, 130, 246, 0.12)",
      }
    : undefined;
}

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
  const [activeFocusTarget, setActiveFocusTarget] = useState<WorkflowDiagnosticsFocusTarget | null>(null);
  const validationRef = useRef<HTMLDivElement | null>(null);
  const integrityRef = useRef<HTMLDivElement | null>(null);
  const bridgeRef = useRef<HTMLDivElement | null>(null);
  const packageResidualsRef = useRef<HTMLDivElement | null>(null);
  const packageImportRef = useRef<HTMLDivElement | null>(null);
  const activityRef = useRef<HTMLDivElement | null>(null);
  const activityLogEntries = readWorkbenchAuditTimeline(workflow.id, 8, {
    frontendRuntimeMode,
    protocolAgents,
  });
  const bridgeRuntimeSummary = summarizeWorkflowBridgeRuntimeStatuses(workflow.graph ?? null, latestRun?.result ?? null);
  const controlFlowHistoryEntries = activityLogEntries.filter((entry) => entry.kind.startsWith("control_flow_"));
  const focusTargetRefMap = useMemo(
    () => ({
      validation: validationRef,
      validationAll: validationRef,
      validationFixable: validationRef,
      validationReview: validationRef,
      integrity: integrityRef,
      bridge: bridgeRef,
      bridgeAligned: bridgeRef,
      bridgeDrift: bridgeRef,
      bridgeMissingRuntime: bridgeRef,
      packageResiduals: packageResidualsRef,
      packageResidualsAll: packageResidualsRef,
      packageResidualsAuto: packageResidualsRef,
      packageResidualsManual: packageResidualsRef,
      packageImport: packageImportRef,
      packageImportAll: packageImportRef,
      packageImportNode: packageImportRef,
      packageImportDataset: packageImportRef,
      packageImportPackage: packageImportRef,
      activity: activityRef,
    }),
    [],
  );
  useEffect(() => {
    if (!activeFocusTarget) return;
    focusTargetRefMap[activeFocusTarget].current?.scrollIntoView({
      block: "nearest",
      behavior: "smooth",
    });
  }, [activeFocusTarget, focusTargetRefMap]);

  function focusDiagnosticsTarget(target: WorkflowDiagnosticsFocusTarget) {
    setActiveFocusTarget((current) => (current === target ? null : target));
  }
  const activeBridgeStatusFilter =
    activeFocusTarget === "bridgeAligned"
      ? "aligned"
      : activeFocusTarget === "bridgeDrift"
        ? "drift"
        : activeFocusTarget === "bridgeMissingRuntime"
          ? "missing-runtime"
          : null;
  const validationFixableCount = validationIssues.filter((issue) => issue.fix).length;
  const validationReviewCount = validationIssues.length - validationFixableCount;
  const packageResidualAutoCount = packageResiduals.filter((entry) => entry.auto_fixable).length;
  const packageResidualManualCount = packageResiduals.length - packageResidualAutoCount;
  const packageImportNodeCount = importDiagnostics.filter((entry) => entry.locate?.kind === "node").length;
  const packageImportDatasetCount = importDiagnostics.filter((entry) => entry.locate?.kind === "dataset").length;
  const packageImportPackageCount = importDiagnostics.filter((entry) => entry.locate?.kind === "package").length;
  const activeValidationFilter =
    activeFocusTarget === "validationFixable"
      ? "fixable"
      : activeFocusTarget === "validationReview"
        ? "review"
        : "all";
  const activePackageResidualFilter =
    activeFocusTarget === "packageResidualsAuto"
      ? "auto"
      : activeFocusTarget === "packageResidualsManual"
        ? "manual"
        : "all";
  const activePackageImportFilter =
    activeFocusTarget === "packageImportNode"
      ? "node"
      : activeFocusTarget === "packageImportDataset"
        ? "dataset"
        : activeFocusTarget === "packageImportPackage"
          ? "package"
          : "all";
  return (
    <section className="workflow-diagnostics-plane">
      <div className="workflow-diagnostics-plane__summary sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.validationTitle}</span>
          <strong>
            <span style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
              <button onClick={() => focusDiagnosticsTarget("validationAll")} type="button">
                {`${labels.validationSummaryAllLabel} ${validationIssues.length}`}
              </button>
              <button onClick={() => focusDiagnosticsTarget("validationFixable")} type="button">
                {`${labels.validationSummaryFixableLabel} ${validationFixableCount}`}
              </button>
              <button onClick={() => focusDiagnosticsTarget("validationReview")} type="button">
                {`${labels.validationSummaryReviewLabel} ${validationReviewCount}`}
              </button>
            </span>
          </strong>
        </div>
        <div className="sidebar-list__row">
          <span>Component integrity</span>
          <strong>
            <button onClick={() => focusDiagnosticsTarget("integrity")} type="button">
              {integrityReport.issues.length}
            </button>
          </strong>
        </div>
        <div className="sidebar-list__row">
          <span>Bridge runtime</span>
          <strong>
            {latestRun?.result ? (
              <span style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
                <button onClick={() => focusDiagnosticsTarget("bridgeAligned")} type="button">
                  {bridgeRuntimeSummary.aligned}
                </button>
                <button onClick={() => focusDiagnosticsTarget("bridgeDrift")} type="button">
                  {bridgeRuntimeSummary.drift}
                </button>
                <button onClick={() => focusDiagnosticsTarget("bridgeMissingRuntime")} type="button">
                  {bridgeRuntimeSummary["missing-runtime"]}
                </button>
              </span>
            ) : (
              <button onClick={() => focusDiagnosticsTarget("bridge")} type="button">
                --
              </button>
            )}
          </strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageInstallRulesResidualsLabel}</span>
          <strong>
            <span style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
              <button onClick={() => focusDiagnosticsTarget("packageResidualsAll")} type="button">
                {`${labels.validationSummaryAllLabel} ${packageResiduals.length}`}
              </button>
              <button onClick={() => focusDiagnosticsTarget("packageResidualsAuto")} type="button">
                {`${labels.packageInstallRulesAutoLabel} ${packageResidualAutoCount}`}
              </button>
              <button onClick={() => focusDiagnosticsTarget("packageResidualsManual")} type="button">
                {`${labels.packageInstallRulesManualLabel} ${packageResidualManualCount}`}
              </button>
            </span>
          </strong>
        </div>
        <div className="sidebar-list__row">
          <span>Package import diagnostics</span>
          <strong>
            <span style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
              <button onClick={() => focusDiagnosticsTarget("packageImportAll")} type="button">
                {`${labels.validationSummaryAllLabel} ${importDiagnostics.length}`}
              </button>
              <button onClick={() => focusDiagnosticsTarget("packageImportNode")} type="button">
                {`${labels.packageDiagnosticsNodeLabel} ${packageImportNodeCount}`}
              </button>
              <button onClick={() => focusDiagnosticsTarget("packageImportDataset")} type="button">
                {`${labels.packageDiagnosticsDatasetLabel} ${packageImportDatasetCount}`}
              </button>
              <button onClick={() => focusDiagnosticsTarget("packageImportPackage")} type="button">
                {`${labels.packageDiagnosticsPackageLabel} ${packageImportPackageCount}`}
              </button>
            </span>
          </strong>
        </div>
        <div className="sidebar-list__row">
          <span>Activity log</span>
          <strong>
            <button onClick={() => focusDiagnosticsTarget("activity")} type="button">
              {activityLogEntries.length}
            </button>
          </strong>
        </div>
      </div>
      <div className="workflow-diagnostics-plane__cards">
        <div
          ref={validationRef}
          style={workflowDiagnosticsFocusRing(
            activeFocusTarget === "validation" ||
              activeFocusTarget === "validationAll" ||
              activeFocusTarget === "validationFixable" ||
              activeFocusTarget === "validationReview",
          )}
        >
          <WorkbenchWorkflowValidationCard
            activeFilter={activeValidationFilter}
            labels={labels}
            onApplyAllValidationFixes={onApplyAllValidationFixes}
            onApplyValidationFix={onApplyValidationFix}
            onLocateValidationIssue={onLocateValidationIssue}
            recentFixSummary={recentFixSummary}
            validationIssues={validationIssues}
          />
        </div>
        <div
          ref={bridgeRef}
          style={workflowDiagnosticsFocusRing(
            activeFocusTarget === "bridge" ||
              activeFocusTarget === "bridgeAligned" ||
              activeFocusTarget === "bridgeDrift" ||
              activeFocusTarget === "bridgeMissingRuntime",
          )}
        >
          <WorkbenchWorkflowBridgeRuntimeCard
            activeStatusFilter={activeBridgeStatusFilter}
            graph={workflow.graph}
            onLocateIssue={onLocateBridgeRuntimeIssue}
            result={latestRun?.result ?? null}
          />
        </div>
        <div ref={integrityRef} style={workflowDiagnosticsFocusRing(activeFocusTarget === "integrity")}>
          <WorkbenchWorkflowIntegrityCard onLocateIssue={onLocateIntegrityIssue} report={integrityReport} />
        </div>
        <WorkbenchWorkflowControlFlowHistoryCard entries={controlFlowHistoryEntries} onLocateTarget={onLocateAuditTarget} onReplayEntry={onReplayAuditEntry} />
        <div
          ref={packageResidualsRef}
          style={workflowDiagnosticsFocusRing(
            activeFocusTarget === "packageResiduals" ||
              activeFocusTarget === "packageResidualsAll" ||
              activeFocusTarget === "packageResidualsAuto" ||
              activeFocusTarget === "packageResidualsManual",
          )}
        >
          <WorkbenchWorkflowPackageInstallCard
            activeFilter={activePackageResidualFilter}
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
        </div>
        <div
          ref={packageImportRef}
          style={workflowDiagnosticsFocusRing(
            activeFocusTarget === "packageImport" ||
              activeFocusTarget === "packageImportAll" ||
              activeFocusTarget === "packageImportNode" ||
              activeFocusTarget === "packageImportDataset" ||
              activeFocusTarget === "packageImportPackage",
          )}
        >
          <WorkbenchWorkflowPackageImportDiagnosticsCard
            activeFilter={activePackageImportFilter}
            diagnostics={importDiagnostics}
            labels={labels}
            onLocateDiagnostic={onLocateImportDiagnostic}
          />
        </div>
        <div ref={activityRef} style={workflowDiagnosticsFocusRing(activeFocusTarget === "activity")}>
          <WorkbenchWorkflowActivityLogCard auditFocusHint={auditFocusHint} entries={activityLogEntries} onLocateTarget={onLocateAuditTarget} protocolAgents={protocolAgents} workflowId={workflow.id} />
        </div>
      </div>
    </section>
  );
}
