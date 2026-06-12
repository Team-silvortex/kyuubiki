"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import { WorkbenchWorkflowIntegrityCard } from "@/components/workbench/workflow/workbench-workflow-integrity-card";
import type { WorkflowIntegrityIssue, WorkflowIntegrityReport } from "@/components/workbench/workflow/workbench-workflow-integrity";
import { WorkbenchWorkflowPackageInstallCard } from "@/components/workbench/workflow/workbench-workflow-package-install-card";
import type { WorkflowPackageResidualRecord } from "@/components/workbench/workflow/workbench-workflow-package-install-report";
import type { WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import { WorkbenchWorkflowValidationCard } from "@/components/workbench/workflow/workbench-workflow-validation-card";
import type { WorkflowGraphValidationIssue } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowDiagnosticsPlaneProps = {
  labels: WorkflowSidebarLabels;
  workflow: WorkflowCatalogEntry;
  importedPackage: WorkflowPackage | null;
  validationIssues: WorkflowGraphValidationIssue[];
  recentFixSummary: string[];
  integrityReport: WorkflowIntegrityReport;
  packageResiduals: WorkflowPackageResidualRecord[];
  snapshotCount: number;
  onApplyAllValidationFixes: () => void;
  onApplyValidationFix: (issueId: string) => void;
  onLocateValidationIssue: (issueId: string) => void;
  onLocateIntegrityIssue: (issue: WorkflowIntegrityIssue) => void;
  onExportPackageInstallReport: (history: Array<{ at: string; kind: "scan" | "repair"; lines: string[] }>) => void;
  onScanPackageResiduals: () => string[];
  onRepairPackageResidual: (residualId: string) => string[];
  onLocatePackageResidual: (residualId: string) => void;
};

export function WorkbenchWorkflowDiagnosticsPlane({
  labels,
  workflow,
  importedPackage,
  validationIssues,
  recentFixSummary,
  integrityReport,
  packageResiduals,
  snapshotCount,
  onApplyAllValidationFixes,
  onApplyValidationFix,
  onLocateValidationIssue,
  onLocateIntegrityIssue,
  onExportPackageInstallReport,
  onScanPackageResiduals,
  onRepairPackageResidual,
  onLocatePackageResidual,
}: WorkbenchWorkflowDiagnosticsPlaneProps) {
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
          <span>{labels.packageInstallRulesResidualsLabel}</span>
          <strong>{packageResiduals.length}</strong>
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
        <WorkbenchWorkflowIntegrityCard onLocateIssue={onLocateIntegrityIssue} report={integrityReport} />
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
      </div>
    </section>
  );
}
