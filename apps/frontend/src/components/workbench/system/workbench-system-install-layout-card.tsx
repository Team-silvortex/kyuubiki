"use client";

import { buildWorkbenchInstallGovernanceDiagnostics, WORKBENCH_STANDARD_STORAGE_CONTRACT } from "@/components/workbench/system/workbench-system-storage-contract";

type WorkbenchSystemInstallLayoutCardProps = {
  title: string;
  hint: string;
};

export function WorkbenchSystemInstallLayoutCard({
  title,
  hint,
}: WorkbenchSystemInstallLayoutCardProps) {
  const diagnostics = buildWorkbenchInstallGovernanceDiagnostics({
    residualCount: 0,
    autoFixableResidualCount: 0,
  });
  const rows = [
    ["Safe mode", diagnostics.safeMode],
    ["Downgrade reason", diagnostics.downgradeReason],
    ["Standard install", diagnostics.standardInstallLabel],
    ["Residual policy", diagnostics.residualPolicyLabel],
    ["Visibility", diagnostics.visibilityLabel],
    ["Storage scope", WORKBENCH_STANDARD_STORAGE_CONTRACT.storageScope],
    ["Local workflow key", WORKBENCH_STANDARD_STORAGE_CONTRACT.localWorkflowKey],
    ["Snapshot index key", WORKBENCH_STANDARD_STORAGE_CONTRACT.snapshotIndexKey],
    ["Snapshot payload prefix", WORKBENCH_STANDARD_STORAGE_CONTRACT.snapshotPayloadPrefix],
    ["Maintenance log key", WORKBENCH_STANDARD_STORAGE_CONTRACT.maintenanceLogKey],
    ["Snapshot rule", WORKBENCH_STANDARD_STORAGE_CONTRACT.snapshotRule],
    ["Cleanup authority", WORKBENCH_STANDARD_STORAGE_CONTRACT.cleanupAuthority],
    ["Retention policy", WORKBENCH_STANDARD_STORAGE_CONTRACT.retentionPolicy],
    ["Ownership model", WORKBENCH_STANDARD_STORAGE_CONTRACT.ownershipModel],
  ] as const;

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span>local</span>
      </div>
      <p className="card-copy">{hint}</p>
      <div className="sidebar-list">
        {rows.map(([label, value]) => (
          <div className="sidebar-list__row" key={label}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
    </section>
  );
}
