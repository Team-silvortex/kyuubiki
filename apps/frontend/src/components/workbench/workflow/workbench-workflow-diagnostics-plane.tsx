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
type WorkflowDiagnosticsPanel = "validation" | "bridge" | "integrity" | "package" | "import" | "activity";

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
  const [activePanel, setActivePanel] = useState<WorkflowDiagnosticsPanel>("validation");
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
    setActivePanel(
      target.startsWith("validation")
        ? "validation"
        : target.startsWith("bridge")
          ? "bridge"
          : target === "integrity"
            ? "integrity"
            : target.startsWith("packageResiduals")
              ? "package"
              : target.startsWith("packageImport")
                ? "import"
                : "activity",
    );
    setActiveFocusTarget((current) => (current === target ? null : target));
  }
  function openDiagnosticsPanel(panel: WorkflowDiagnosticsPanel) {
    const defaultTarget: Record<WorkflowDiagnosticsPanel, WorkflowDiagnosticsFocusTarget> = {
      validation: "validationAll",
      bridge: "bridge",
      integrity: "integrity",
      package: "packageResidualsAll",
      import: "packageImportAll",
      activity: "activity",
    };
    setActivePanel(panel);
    setActiveFocusTarget(defaultTarget[panel]);
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
  const bridgeRuntimeCount = latestRun?.result
    ? bridgeRuntimeSummary.aligned + bridgeRuntimeSummary.drift + bridgeRuntimeSummary["missing-runtime"]
    : "--";
  const diagnosticsPanels: Array<{ id: WorkflowDiagnosticsPanel; label: string; count: number | string }> = [
    { id: "validation", label: labels.validationTitle, count: validationIssues.length },
    { id: "integrity", label: "Component integrity", count: integrityReport.issues.length },
    { id: "bridge", label: "Bridge runtime", count: bridgeRuntimeCount },
    { id: "package", label: labels.packageInstallRulesResidualsLabel, count: packageResiduals.length },
    { id: "import", label: "Package import diagnostics", count: importDiagnostics.length },
    { id: "activity", label: "Activity log", count: activityLogEntries.length },
  ];
  return (
    <section className="workflow-diagnostics-plane" data-workflow-diagnostics-panel={activePanel}>
      <nav aria-label={labels.validationTitle} className="workflow-diagnostics-plane__tabs">
        {diagnosticsPanels.map((panel) => (
          <button aria-pressed={activePanel === panel.id} className={activePanel === panel.id ? "workflow-diagnostics-plane__tab workflow-diagnostics-plane__tab--active" : "workflow-diagnostics-plane__tab"} data-workflow-diagnostics-panel-target={panel.id} key={panel.id} onClick={() => openDiagnosticsPanel(panel.id)} title={panel.label} type="button">
            <span>{panel.label}</span><strong>{panel.count}</strong>
          </button>
        ))}
      </nav>
      {activePanel === "validation" ? (
        <div className="workflow-diagnostics-plane__filters" data-workflow-diagnostics-filter="validation">
          <button aria-pressed={activeValidationFilter === "all"} onClick={() => focusDiagnosticsTarget("validationAll")} type="button">{`${labels.validationSummaryAllLabel} ${validationIssues.length}`}</button>
          <button aria-pressed={activeValidationFilter === "fixable"} onClick={() => focusDiagnosticsTarget("validationFixable")} type="button">{`${labels.validationSummaryFixableLabel} ${validationFixableCount}`}</button>
          <button aria-pressed={activeValidationFilter === "review"} onClick={() => focusDiagnosticsTarget("validationReview")} type="button">{`${labels.validationSummaryReviewLabel} ${validationReviewCount}`}</button>
        </div>
      ) : null}
      {activePanel === "bridge" && latestRun?.result ? (
        <div className="workflow-diagnostics-plane__filters" data-workflow-diagnostics-filter="bridge">
          <button aria-pressed={activeBridgeStatusFilter === "aligned"} onClick={() => focusDiagnosticsTarget("bridgeAligned")} type="button">{bridgeRuntimeSummary.aligned}</button>
          <button aria-pressed={activeBridgeStatusFilter === "drift"} onClick={() => focusDiagnosticsTarget("bridgeDrift")} type="button">{bridgeRuntimeSummary.drift}</button>
          <button aria-pressed={activeBridgeStatusFilter === "missing-runtime"} onClick={() => focusDiagnosticsTarget("bridgeMissingRuntime")} type="button">{bridgeRuntimeSummary["missing-runtime"]}</button>
        </div>
      ) : null}
      {activePanel === "package" ? (
        <div className="workflow-diagnostics-plane__filters" data-workflow-diagnostics-filter="package">
          <button aria-pressed={activePackageResidualFilter === "all"} onClick={() => focusDiagnosticsTarget("packageResidualsAll")} type="button">{`${labels.validationSummaryAllLabel} ${packageResiduals.length}`}</button>
          <button aria-pressed={activePackageResidualFilter === "auto"} onClick={() => focusDiagnosticsTarget("packageResidualsAuto")} type="button">{`${labels.packageInstallRulesAutoLabel} ${packageResidualAutoCount}`}</button>
          <button aria-pressed={activePackageResidualFilter === "manual"} onClick={() => focusDiagnosticsTarget("packageResidualsManual")} type="button">{`${labels.packageInstallRulesManualLabel} ${packageResidualManualCount}`}</button>
        </div>
      ) : null}
      {activePanel === "import" ? (
        <div className="workflow-diagnostics-plane__filters" data-workflow-diagnostics-filter="import">
          <button aria-pressed={activePackageImportFilter === "all"} onClick={() => focusDiagnosticsTarget("packageImportAll")} type="button">{`${labels.validationSummaryAllLabel} ${importDiagnostics.length}`}</button>
          <button aria-pressed={activePackageImportFilter === "node"} onClick={() => focusDiagnosticsTarget("packageImportNode")} type="button">{`${labels.packageDiagnosticsNodeLabel} ${packageImportNodeCount}`}</button>
          <button aria-pressed={activePackageImportFilter === "dataset"} onClick={() => focusDiagnosticsTarget("packageImportDataset")} type="button">{`${labels.packageDiagnosticsDatasetLabel} ${packageImportDatasetCount}`}</button>
          <button aria-pressed={activePackageImportFilter === "package"} onClick={() => focusDiagnosticsTarget("packageImportPackage")} type="button">{`${labels.packageDiagnosticsPackageLabel} ${packageImportPackageCount}`}</button>
        </div>
      ) : null}
      <div className="workflow-diagnostics-plane__cards">
        {activePanel === "validation" ? <div
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
        </div> : null}
        {activePanel === "bridge" ? <div
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
        </div> : null}
        {activePanel === "integrity" ? <div ref={integrityRef} style={workflowDiagnosticsFocusRing(activeFocusTarget === "integrity")}>
          <WorkbenchWorkflowIntegrityCard onLocateIssue={onLocateIntegrityIssue} report={integrityReport} />
        </div> : null}
        {activePanel === "package" ? <div
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
        </div> : null}
        {activePanel === "import" ? <div
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
        </div> : null}
        {activePanel === "activity" ? (
          <>
            <WorkbenchWorkflowControlFlowHistoryCard entries={controlFlowHistoryEntries} onLocateTarget={onLocateAuditTarget} onReplayEntry={onReplayAuditEntry} />
            <div ref={activityRef} style={workflowDiagnosticsFocusRing(activeFocusTarget === "activity")}>
              <WorkbenchWorkflowActivityLogCard auditFocusHint={auditFocusHint} entries={activityLogEntries} onLocateTarget={onLocateAuditTarget} protocolAgents={protocolAgents} workflowId={workflow.id} />
            </div>
          </>
        ) : null}
      </div>
    </section>
  );
}
