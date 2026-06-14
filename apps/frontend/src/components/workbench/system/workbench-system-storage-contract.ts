export type WorkbenchStandardStorageContract = {
  storageScope: string;
  localWorkflowKey: string;
  snapshotIndexKey: string;
  snapshotPayloadPrefix: string;
  maintenanceLogKey: string;
  snapshotRule: string;
  cleanupAuthority: string;
  retentionPolicy: string;
  ownershipModel: string;
  formatContract: string;
  portability: string;
};

export type WorkbenchInstallGovernanceDiagnostics = {
  safeMode: "standard" | "watch" | "repair_only";
  standardInstallLabel: string;
  residualPolicyLabel: string;
  visibilityLabel: string;
  downgradeReason: string;
};

export const WORKBENCH_STANDARD_STORAGE_CONTRACT: WorkbenchStandardStorageContract = {
  storageScope: "browser localStorage / per-user workspace profile",
  localWorkflowKey: "kyuubiki.workbench.workflowLibrary.v1",
  snapshotIndexKey: "kyuubiki.workbench.workflowSnapshots.index.v1",
  snapshotPayloadPrefix: "kyuubiki.workbench.workflowSnapshots.payload.v1:",
  maintenanceLogKey: "kyuubiki.workbench.workflowPackageMaintenanceLog.v1",
  snapshotRule: "workflow snapshots are indexed separately from package metadata; limit=20",
  cleanupAuthority:
    "safe cache cleanup may be initiated by hub/workbench; destructive removal of retained workflow assets stays explicit",
  retentionPolicy:
    "snapshots and drafts are bounded caches; local workflow assets, presets, and settings persist until explicit delete or repair",
  ownershipModel:
    "hub is the system entrypoint, workbench owns workflow state, installer owns runtime deployment, agents execute but do not become the source of truth",
  formatContract: "kyuubiki.workflow-package v1 + workflow dataset contract JSON",
  portability:
    "package manifest, graph, and dataset contract remain JSON-exportable for cross-operator reuse and headless SDK flows.",
};

export function buildWorkbenchInstallGovernanceDiagnostics(input: {
  residualCount: number;
  autoFixableResidualCount: number;
  summaryOnlySnapshotCount?: number;
}) {
  const summaryOnlySnapshotCount = input.summaryOnlySnapshotCount ?? 0;
  const hasResiduals = input.residualCount > 0;
  const hasManualResiduals = input.residualCount > input.autoFixableResidualCount;
  const safeMode =
    hasManualResiduals || summaryOnlySnapshotCount > 0
      ? "repair_only"
      : hasResiduals
        ? "watch"
        : "standard";

  return {
    safeMode,
    standardInstallLabel:
      "single local storage authority + explicit retention + installer-owned runtime deployment",
    residualPolicyLabel:
      hasResiduals
        ? hasManualResiduals
          ? "residuals detected; auto cleanup is restricted to preview/explicit repair"
          : "residuals detected; only previewable safe repairs are allowed"
        : "no residual pressure; standard install layout is aligned",
    visibilityLabel:
      "all install keys, cleanup scope, and storage paths stay visible in read-only policy surfaces",
    downgradeReason:
      safeMode === "repair_only"
        ? summaryOnlySnapshotCount > 0
          ? "summary-only snapshots require explicit repair posture"
          : "manual residuals are present"
        : safeMode === "watch"
          ? "safe residual review is required before cleanup"
          : "aligned",
  } satisfies WorkbenchInstallGovernanceDiagnostics;
}
