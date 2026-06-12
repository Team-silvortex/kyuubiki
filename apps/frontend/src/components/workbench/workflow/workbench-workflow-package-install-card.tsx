"use client";

import { useEffect, useState } from "react";
import type { WorkflowCatalogEntry } from "@/lib/api";
import {
  listStoredWorkflowPackageMaintenanceHistory,
  saveStoredWorkflowPackageMaintenanceHistory,
  type WorkflowPackageMaintenanceLogEntry,
  WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY,
} from "@/components/workbench/workflow/workbench-workflow-package-maintenance-log";
import { WORKBENCH_LOCAL_WORKFLOWS_KEY } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import type { WorkflowPackageResidualRecord } from "@/components/workbench/workflow/workbench-workflow-package-install-report";
import type { WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import { WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY, WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowPackageInstallCardProps = {
  labels: WorkflowSidebarLabels;
  workflow: WorkflowCatalogEntry;
  importedPackage: WorkflowPackage | null;
  snapshotCount: number;
  summaryOnlySnapshotCount: number;
  residuals: WorkflowPackageResidualRecord[];
  onExportReport: (history: Array<{ at: string; kind: "scan" | "repair"; lines: string[] }>) => void;
  onScanResiduals: () => string[];
  onRepairResidual: (residualId: string) => string[];
  onLocateResidual: (residualId: string) => void;
};

function buildInstallRows(
  labels: WorkflowSidebarLabels,
  workflow: WorkflowCatalogEntry,
  importedPackage: WorkflowPackage | null,
  snapshotCount: number,
  summaryOnlySnapshotCount: number,
) {
  const hasMountedPackage = Boolean(importedPackage || workflow.local?.imported_from_package_id);
  const mountState = importedPackage
    ? `${labels.packageManifestDraftLabel} / browser session`
    : workflow.local?.imported_from_package_id
      ? `${labels.packageManifestMountedLabel} / local workflow library`
      : workflow.local
        ? `${labels.localWorkflowBadgeLabel} / no package mount`
        : `${workflow.version} / built-in catalog`;
  const storageRule = importedPackage
    ? "Draft package metadata stays in browser memory until the draft is replaced, reloaded, or promoted."
    : workflow.local
      ? "Mounted package linkage is persisted in browser localStorage with the local workflow record."
      : "Built-in workflows resolve from the bundled catalog and do not create local package storage by default.";
  const cleanupRule = workflow.local
    ? "Deleting this local workflow also clears its workflow snapshots. Draft-only package mounts are discarded when another draft is loaded."
    : "No local cleanup is required until this workflow is promoted into the local library.";
  const snapshotRule =
    snapshotCount > 0
      ? `${snapshotCount} snapshot(s) indexed; ${summaryOnlySnapshotCount} summary-only due to payload size limits.`
      : "Snapshots are created per workflow and stored separately from package manifest metadata.";
  const formatRule = hasMountedPackage
    ? "kyuubiki.workflow-package v1 + workflow dataset contract JSON"
    : "Built-in workflow graph + optional workflow package export";
  const portabilityRule =
    "Package manifest, graph, and dataset contract remain JSON-exportable for cross-operator reuse and headless SDK flows.";

  return [
    [labels.packageInstallRulesMountStateLabel, mountState],
    [labels.packageInstallRulesStorageLabel, storageRule],
    [labels.packageInstallRulesStorageScopeLabel, "browser localStorage / per-user workspace profile"],
    [labels.packageInstallRulesLocalPathLabel, WORKBENCH_LOCAL_WORKFLOWS_KEY],
    [labels.packageInstallRulesSnapshotPathLabel, WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY],
    [labels.packageInstallRulesSnapshotPayloadPathLabel, WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX],
    [labels.packageInstallRulesMaintenancePathLabel, WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY],
    [labels.packageInstallRulesCleanupLabel, cleanupRule],
    [labels.packageInstallRulesSnapshotLabel, snapshotRule],
    [labels.packageInstallRulesFormatLabel, formatRule],
    [labels.packageInstallRulesPortabilityLabel, portabilityRule],
  ] as const;
}

export function WorkbenchWorkflowPackageInstallCard({
  labels,
  workflow,
  importedPackage,
  snapshotCount,
  summaryOnlySnapshotCount,
  residuals,
  onExportReport,
  onScanResiduals,
  onRepairResidual,
  onLocateResidual,
}: WorkbenchWorkflowPackageInstallCardProps) {
  const [previewResidualIds, setPreviewResidualIds] = useState<string[] | null>(null);
  const [history, setHistory] = useState<WorkflowPackageMaintenanceLogEntry[]>([]);
  const rows = buildInstallRows(
    labels,
    workflow,
    importedPackage,
    snapshotCount,
    summaryOnlySnapshotCount,
  );
  const autoFixableCount = residuals.filter((entry) => entry.auto_fixable).length;
  const previewResiduals = previewResidualIds ? residuals.filter((entry) => previewResidualIds.includes(entry.id)) : [];
  useEffect(() => {
    setHistory(listStoredWorkflowPackageMaintenanceHistory(workflow.id));
    setPreviewResidualIds(null);
  }, [workflow.id]);
  function appendHistory(kind: WorkflowPackageMaintenanceLogEntry["kind"], lines: string[]) {
    if (lines.length === 0) return;
    setHistory((current) => {
      const next = [
        { id: `${kind}:${Date.now()}`, at: new Date().toISOString(), kind, lines, workflowId: workflow.id },
        ...current,
      ].slice(0, 12);
      saveStoredWorkflowPackageMaintenanceHistory(
        workflow.id,
        next.map(({ id, at, kind: nextKind, lines: nextLines }) => ({
          id,
          at,
          kind: nextKind,
          lines: nextLines,
        })),
      );
      return next;
    });
  }

  return (
    <section className="sidebar-card sidebar-card--compact" data-workflow-package-policy-card="card">
      <div className="card-head">
        <h2>{labels.packageInstallRulesTitle}</h2>
        <span className="status-pill status-pill--watch">{labels.packageInstallRulesReadonlyLabel}</span>
      </div>
      <div className="sidebar-list">
        {rows.map(([label, value]) => (
          <div className="sidebar-list__row" key={label}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
        <div className="sidebar-list__row">
          <span>{labels.packageInstallRulesResidualsLabel}</span>
          <strong>{residuals.length === 0 ? labels.packageInstallRulesResidualsCleanLabel : `${residuals.length} warning(s)`}</strong>
        </div>
      </div>
      {residuals.length > 0 ? (
        <div style={{ display: "grid", gap: "0.35rem", marginTop: "0.75rem" }}>
          {residuals.map((entry) => (
            <div key={entry.id} style={{ display: "grid", gap: "0.35rem" }}>
              <div className="sidebar-list__row">
                <span>{entry.message}</span>
                <strong>
                  <span className={`status-pill status-pill--${entry.auto_fixable ? "good" : "watch"}`}>
                    {entry.auto_fixable ? labels.packageInstallRulesAutoLabel : labels.packageInstallRulesManualLabel}
                  </span>
                </strong>
              </div>
              {entry.auto_fixable ? (
                <div className="button-row">
                  <button onClick={() => setPreviewResidualIds([entry.id])} type="button">{labels.packageInstallRulesRepairItemLabel}</button>
                  <button onClick={() => onLocateResidual(entry.id)} type="button">{labels.validationLocateLabel}</button>
                </div>
              ) : (
                <div className="button-row">
                  <button onClick={() => onLocateResidual(entry.id)} type="button">{labels.validationLocateLabel}</button>
                </div>
              )}
            </div>
          ))}
        </div>
      ) : null}
      {previewResiduals.length > 0 ? (
        <div className="sidebar-card sidebar-card--compact" style={{ marginTop: "0.75rem" }}>
          <div className="card-head">
            <h2>{labels.packageInstallRulesPreviewTitle}</h2>
            <span className="status-pill status-pill--watch">{previewResiduals.length}</span>
          </div>
          <div style={{ display: "grid", gap: "0.35rem" }}>
            {previewResiduals.map((entry) => (
              <p className="card-copy" key={`preview:${entry.id}`}>{entry.message}</p>
            ))}
          </div>
          <div className="button-row button-row--adaptive">
            <button onClick={() => { const receipt = previewResiduals.flatMap((entry) => onRepairResidual(entry.id)); appendHistory("repair", receipt); setPreviewResidualIds(null); }} type="button">{labels.packageInstallRulesPreviewApplyLabel}</button>
            <button onClick={() => setPreviewResidualIds(null)} type="button">{labels.packageInstallRulesPreviewCancelLabel}</button>
          </div>
        </div>
      ) : null}
      <div className="sidebar-card sidebar-card--compact" style={{ marginTop: "0.75rem" }}>
        <div className="card-head">
          <h2>{labels.packageInstallRulesReceiptTitle}</h2>
          <span className={`status-pill status-pill--${history.length > 0 ? "good" : "watch"}`}>{history.length}</span>
        </div>
        {history.length > 0 ? (
          <div style={{ display: "grid", gap: "0.5rem" }}>
            {history.map((entry) => (
              <div key={entry.id} style={{ display: "grid", gap: "0.35rem" }}>
                <div className="sidebar-list__row">
                  <span>{new Date(entry.at).toLocaleString()}</span>
                  <strong>{entry.kind === "scan" ? labels.packageInstallRulesScanLabel : labels.packageInstallRulesRepairLabel}</strong>
                </div>
                {entry.lines.map((line, index) => (
                  <p className="card-copy" key={`${entry.id}:${index}`}>{line}</p>
                ))}
              </div>
            ))}
          </div>
        ) : (
          <p className="card-copy">{labels.packageInstallRulesHistoryEmptyLabel}</p>
        )}
      </div>
      <div className="button-row button-row--adaptive">
        <button onClick={() => appendHistory("scan", onScanResiduals())} type="button">{labels.packageInstallRulesScanLabel}</button>
        <button disabled={autoFixableCount === 0} onClick={() => setPreviewResidualIds(residuals.filter((entry) => entry.auto_fixable).map((entry) => entry.id))} type="button">{labels.packageInstallRulesRepairLabel}</button>
        <button onClick={() => onExportReport(history.map(({ at, kind, lines }) => ({ at, kind, lines })))} type="button">{labels.packageInstallRulesExportLabel}</button>
      </div>
    </section>
  );
}
