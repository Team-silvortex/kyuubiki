"use client";

import type { ProtocolAgentDescriptor, WorkflowCatalogEntry } from "@/lib/api";
import type { WorkflowIntegrityReport } from "@/components/workbench/workflow/workbench-workflow-integrity";
import { formatWorkflowContractHealthSummary, formatWorkflowDynamicReviewState } from "@/components/workbench/workflow/workbench-workflow-contract-health";
import { collectWorkflowInputArtifactContractWarnings } from "@/components/workbench/workflow/workbench-workflow-fem-validation";
import { buildWorkbenchInstallGovernanceDiagnostics, WORKBENCH_STANDARD_STORAGE_CONTRACT } from "@/components/workbench/system/workbench-system-storage-contract";
import type { WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import { readWorkbenchAuditTimeline } from "@/lib/workbench/workbench-audit-timeline";

export type WorkflowPackageResidualRecord = {
  id: string;
  kind: "orphan_snapshots" | "local_link_missing" | "package_override" | "summary_only_snapshots";
  severity: "warning";
  auto_fixable: boolean;
  locate: "snapshot" | "local" | "package";
  message: string;
};

export type WorkflowPackageRepairPlanStep = {
  residualId: string;
  action: string;
  scope: string;
  safe: boolean;
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
    safe_mode: string;
    downgrade_reason: string;
    standard_install: string;
    residual_policy: string;
    visibility: string;
    storage: string;
    storage_scope: string;
    local_workflow_key: string;
    snapshot_index_key: string;
    snapshot_payload_prefix: string;
    maintenance_log_key: string;
    cleanup: string;
    snapshots: string;
    cleanup_authority: string;
    retention_policy: string;
    ownership_model: string;
    format_contract: string;
    portability: string;
  };
  input_contract_warnings: Record<string, string[]>;
  contract_health: {
    static_health: string;
    dynamic_review_state: string;
    recent_run_status?: string;
  };
  integrity: WorkflowIntegrityReport;
  residuals: WorkflowPackageResidualRecord[];
  repair_plan: WorkflowPackageRepairPlanStep[];
  maintenance_history?: Array<{
    at: string;
    kind: "scan" | "repair";
    lines: string[];
  }>;
  audit_timeline?: Array<{
    at: string;
    source: string;
    kind: string;
    message: string;
    detail?: string;
    count?: number;
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
  recentRunStatus?: string | null;
  protocolAgents?: ProtocolAgentDescriptor[];
  frontendRuntimeMode?: "orchestrated_gui" | "direct_mesh_gui";
  maintenanceHistory?: Array<{
    at: string;
    kind: "scan" | "repair";
    lines: string[];
  }>;
}): WorkflowPackageInstallReport {
  const { workflow, importedPackage, integrityReport, maintenanceHistory } = params;
  const residuals = scanWorkflowPackageResiduals(params);
  const installGovernance = buildWorkbenchInstallGovernanceDiagnostics({
    residualCount: residuals.length,
    autoFixableResidualCount: residuals.filter((entry) => entry.auto_fixable).length,
    summaryOnlySnapshotCount: integrityReport.summaryOnlySnapshotCount,
  });
  const repairPlan = buildWorkflowPackageRepairPlan(residuals);
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
  const auditTimeline = readWorkbenchAuditTimeline(workflow.id, 24, params.frontendRuntimeMode ? {
    frontendRuntimeMode: params.frontendRuntimeMode,
    protocolAgents: params.protocolAgents ?? [],
  } : undefined)
    .slice(0, 16)
    .map((entry) => ({ at: entry.at, source: entry.source, kind: entry.kind, message: entry.title, detail: entry.detail, count: entry.count }));

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
      safe_mode: installGovernance.safeMode,
      downgrade_reason: installGovernance.downgradeReason,
      standard_install: installGovernance.standardInstallLabel,
      residual_policy: installGovernance.residualPolicyLabel,
      visibility: installGovernance.visibilityLabel,
      storage,
      storage_scope: WORKBENCH_STANDARD_STORAGE_CONTRACT.storageScope,
      local_workflow_key: WORKBENCH_STANDARD_STORAGE_CONTRACT.localWorkflowKey,
      snapshot_index_key: WORKBENCH_STANDARD_STORAGE_CONTRACT.snapshotIndexKey,
      snapshot_payload_prefix: WORKBENCH_STANDARD_STORAGE_CONTRACT.snapshotPayloadPrefix,
      maintenance_log_key: WORKBENCH_STANDARD_STORAGE_CONTRACT.maintenanceLogKey,
      cleanup,
      snapshots: snapshots.length > 0 ? snapshots : WORKBENCH_STANDARD_STORAGE_CONTRACT.snapshotRule,
      cleanup_authority: WORKBENCH_STANDARD_STORAGE_CONTRACT.cleanupAuthority,
      retention_policy: WORKBENCH_STANDARD_STORAGE_CONTRACT.retentionPolicy,
      ownership_model: WORKBENCH_STANDARD_STORAGE_CONTRACT.ownershipModel,
      format_contract: importedPackage
        ? WORKBENCH_STANDARD_STORAGE_CONTRACT.formatContract
        : "Built-in workflow graph + optional workflow package export",
      portability: WORKBENCH_STANDARD_STORAGE_CONTRACT.portability,
    },
    input_contract_warnings: inputContractWarnings,
    contract_health: {
      static_health: formatWorkflowContractHealthSummary(inputContractWarnings),
      dynamic_review_state: formatWorkflowDynamicReviewState({
        warnings: inputContractWarnings,
        recentRunStatus: params.recentRunStatus,
      }),
      recent_run_status: params.recentRunStatus ?? undefined,
    },
    integrity: integrityReport,
    residuals,
    repair_plan: repairPlan,
    maintenance_history: maintenanceHistory,
    audit_timeline: auditTimeline,
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

export function buildWorkflowPackageRepairPlan(
  residuals: WorkflowPackageResidualRecord[],
): WorkflowPackageRepairPlanStep[] {
  return residuals.flatMap((entry) => {
    if (entry.kind === "orphan_snapshots") {
      return [{
        residualId: entry.id,
        action: "Remove orphan workflow snapshots that no longer have a local workflow owner.",
        scope: "snapshot index + snapshot payload cache",
        safe: entry.auto_fixable,
      }];
    }
    if (entry.kind === "summary_only_snapshots") {
      return [{
        residualId: entry.id,
        action: "Remove summary-only snapshots that cannot restore a full workflow graph.",
        scope: "snapshot payload cache",
        safe: entry.auto_fixable,
      }];
    }
    if (entry.kind === "package_override") {
      return [{
        residualId: entry.id,
        action: "Discard the active draft override and restore the mounted workflow package linkage.",
        scope: "draft package session state",
        safe: entry.auto_fixable,
      }];
    }
    if (entry.kind === "local_link_missing") {
      return [{
        residualId: entry.id,
        action: "Rebuild or manually relink the missing local workflow record before cleanup.",
        scope: "local workflow library entry",
        safe: false,
      }];
    }
    return [];
  });
}
