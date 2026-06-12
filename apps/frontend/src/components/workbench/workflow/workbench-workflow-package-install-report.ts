"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import type { WorkflowIntegrityReport } from "@/components/workbench/workflow/workbench-workflow-integrity";
import { collectWorkflowInputArtifactContractWarnings } from "@/components/workbench/workflow/workbench-workflow-fem-validation";
import { WORKBENCH_LOCAL_WORKFLOWS_KEY } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY } from "@/components/workbench/workflow/workbench-workflow-package-maintenance-log";
import type { WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import { WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY, WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT, WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";

export type WorkflowPackageResidualRecord = {
  id: string;
  kind: "orphan_snapshots" | "local_link_missing" | "package_override" | "summary_only_snapshots";
  severity: "warning";
  auto_fixable: boolean;
  locate: "snapshot" | "local" | "package";
  message: string;
};

export type WorkflowPackageInstallReport = {
  generated_at: string;
  workflow: {
    id: string;
    name: string;
    version: string;
    local_storage_id?: string;
  };
  package_mount: {
    package_id?: string;
    package_version?: string;
    mount_state: "draft_session" | "local_library" | "local_without_package" | "built_in";
    tags: string[];
    domains: string[];
    capability_tags: string[];
  };
  install_rules: {
    storage: string;
    storage_scope: string;
    local_workflow_key: string;
    snapshot_index_key: string;
    snapshot_payload_prefix: string;
    maintenance_log_key: string;
    cleanup: string;
    snapshots: string;
    format_contract: string;
    portability: string;
  };
  input_contract_warnings: Record<string, string[]>;
  integrity: WorkflowIntegrityReport;
  residuals: WorkflowPackageResidualRecord[];
  maintenance_history?: Array<{
    at: string;
    kind: "scan" | "repair";
    lines: string[];
  }>;
};

export function scanWorkflowPackageResiduals(params: {
  workflow: WorkflowCatalogEntry;
  importedPackage: WorkflowPackage | null;
  integrityReport: WorkflowIntegrityReport;
}): WorkflowPackageResidualRecord[] {
  const { workflow, importedPackage, integrityReport } = params;
  const residuals: WorkflowPackageResidualRecord[] = [];
  if (!workflow.local && integrityReport.snapshotCount > 0) {
    residuals.push({
      id: `snapshot-residual:${workflow.id}`,
      kind: "orphan_snapshots",
      severity: "warning",
      auto_fixable: true,
      locate: "snapshot",
      message: `${integrityReport.snapshotCount} snapshot(s) exist without a local workflow record.`,
    });
  }
  if (workflow.local && !integrityReport.localWorkflowFound) {
    residuals.push({
      id: `local-link-residual:${workflow.id}`,
      kind: "local_link_missing",
      severity: "warning",
      auto_fixable: false,
      locate: "local",
      message: "Local workflow linkage is visible in the catalog but missing from local storage.",
    });
  }
  if (workflow.local?.imported_from_package_id && importedPackage?.package_id && workflow.local.imported_from_package_id !== importedPackage.package_id) {
    residuals.push({
      id: `package-override:${workflow.id}`,
      kind: "package_override",
      severity: "warning",
      auto_fixable: true,
      locate: "package",
      message: `Draft package "${importedPackage.package_id}" is overriding mounted package "${workflow.local.imported_from_package_id}".`,
    });
  }
  if (integrityReport.summaryOnlySnapshotCount > 0) {
    residuals.push({
      id: `summary-only:${workflow.id}`,
      kind: "summary_only_snapshots",
      severity: "warning",
      auto_fixable: true,
      locate: "snapshot",
      message: `${integrityReport.summaryOnlySnapshotCount} snapshot(s) are summary-only and cannot be fully restored.`,
    });
  }
  return residuals;
}

export function buildWorkflowPackageInstallReport(params: {
  workflow: WorkflowCatalogEntry;
  importedPackage: WorkflowPackage | null;
  integrityReport: WorkflowIntegrityReport;
  maintenanceHistory?: Array<{
    at: string;
    kind: "scan" | "repair";
    lines: string[];
  }>;
}): WorkflowPackageInstallReport {
  const { workflow, importedPackage, integrityReport, maintenanceHistory } = params;
  const mountState = importedPackage
    ? "draft_session"
    : workflow.local?.imported_from_package_id
      ? "local_library"
      : workflow.local
        ? "local_without_package"
        : "built_in";
  const storage = importedPackage
    ? "Draft package metadata remains in browser memory until draft replacement, reload, or promotion."
    : workflow.local
      ? "Mounted package linkage persists in browser localStorage beside the local workflow entry."
      : "Built-in workflows resolve from the bundled catalog without creating package storage.";
  const cleanup = workflow.local
    ? "Deleting the local workflow also removes its workflow snapshots."
    : "Cleanup is deferred until the workflow is promoted into the local library.";
  const snapshots =
    integrityReport.snapshotCount > 0
      ? `${integrityReport.snapshotCount} indexed; ${integrityReport.summaryOnlySnapshotCount} summary-only.`
      : "Snapshots are stored independently from package metadata.";
  const inputContractWarnings =
    importedPackage?.workflow.input_artifact_contract_warnings ??
    collectWorkflowInputArtifactContractWarnings({
      entryInputs: workflow.entry_inputs,
      inputArtifactTexts: workflow.local?.input_artifact_texts,
    });

  return {
    generated_at: new Date().toISOString(),
    workflow: {
      id: workflow.id,
      name: workflow.name,
      version: workflow.version,
      local_storage_id: workflow.local?.storage_id,
    },
    package_mount: {
      package_id: importedPackage?.package_id ?? workflow.local?.imported_from_package_id,
      package_version:
        importedPackage?.package_version ?? workflow.local?.imported_from_package_version,
      mount_state: mountState,
      tags: importedPackage?.tags ?? workflow.local?.tags ?? [],
      domains: importedPackage?.search_index.domains ?? workflow.domains ?? [],
      capability_tags:
        importedPackage?.search_index.capability_tags ?? workflow.capability_tags ?? [],
    },
    install_rules: {
      storage,
      storage_scope: "browser localStorage / per-user workspace profile",
      local_workflow_key: WORKBENCH_LOCAL_WORKFLOWS_KEY,
      snapshot_index_key: WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY,
      snapshot_payload_prefix: WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX,
      maintenance_log_key: WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY,
      cleanup,
      snapshots: `${snapshots} limit=${WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT}`,
      format_contract: importedPackage
        ? "kyuubiki.workflow-package v1 + workflow dataset contract JSON"
        : "Built-in workflow graph + optional workflow package export",
      portability:
        "Package manifest, graph, and dataset contract remain JSON-exportable for cross-operator reuse and headless SDK flows.",
    },
    input_contract_warnings: inputContractWarnings,
    integrity: integrityReport,
    residuals: scanWorkflowPackageResiduals(params),
    maintenance_history: maintenanceHistory,
  };
}

export function buildWorkflowPackageResidualRepairPreview(
  residuals: WorkflowPackageResidualRecord[],
) {
  if (residuals.length === 0) return "";
  const actions = residuals.flatMap((entry) => {
    if (!entry.auto_fixable) return [];
    if (entry.kind === "orphan_snapshots") return ["- remove orphan workflow snapshots for this workflow"];
    if (entry.kind === "summary_only_snapshots") return ["- remove summary-only snapshots that cannot be restored"];
    if (entry.kind === "package_override") return ["- discard the current draft package override and restore the mounted workflow state"];
    return [];
  });
  return actions.length === 0 ? "" : ["Apply these safe repairs?", ...actions].join("\n");
}
