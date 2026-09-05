"use client";

import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import { buildWorkbenchInstallGovernanceDiagnostics, WORKBENCH_STANDARD_STORAGE_CONTRACT } from "@/components/workbench/system/workbench-system-storage-contract";
import { WorkbenchSystemOverviewCard } from "@/components/workbench/system/workbench-system-overview-card";

type WorkbenchSystemInstallLayoutCardProps = {
  copy: WorkbenchCopy;
  title: string;
  hint: string;
};

export function WorkbenchSystemInstallLayoutCard({
  copy,
  title,
  hint,
}: WorkbenchSystemInstallLayoutCardProps) {
  const diagnostics = buildWorkbenchInstallGovernanceDiagnostics({
    residualCount: 0,
    autoFixableResidualCount: 0,
  });
  const rows = [
    [copy.workflowPackageInstallRulesReadonlyLabel, diagnostics.safeMode],
    [copy.workflowPackageInstallRulesMountStateLabel, diagnostics.downgradeReason],
    [copy.workflowPackageInstallRulesPortabilityLabel, copy.settingsInstallPolicyUpdateValue],
    [copy.workflowPackageInstallRulesResidualsLabel, copy.workflowPackageInstallRulesResidualsCleanLabel],
    [copy.workflowPackageInstallRulesStorageScopeLabel, "browser.localStorage"],
    [copy.workflowPackageInstallRulesLocalPathLabel, WORKBENCH_STANDARD_STORAGE_CONTRACT.localWorkflowKey],
    [copy.workflowPackageInstallRulesSnapshotPathLabel, WORKBENCH_STANDARD_STORAGE_CONTRACT.snapshotIndexKey],
    [copy.workflowPackageInstallRulesSnapshotPayloadPathLabel, WORKBENCH_STANDARD_STORAGE_CONTRACT.snapshotPayloadPrefix],
    [copy.workflowPackageInstallRulesMaintenancePathLabel, WORKBENCH_STANDARD_STORAGE_CONTRACT.maintenanceLogKey],
    [copy.workflowPackageInstallRulesSnapshotLabel, "limit=20"],
    [copy.workflowPackageInstallRulesFormatLabel, WORKBENCH_STANDARD_STORAGE_CONTRACT.formatContract],
  ] as const;

  return (
    <WorkbenchSystemOverviewCard hint={hint} status={copy.workflowPackageInstallRulesReadonlyLabel} title={title}>
      <div className="sidebar-list">
        {rows.map(([label, value]) => (
          <div className="sidebar-list__row" key={label}>
            <span>{label}</span>
            <strong>{value}</strong>
          </div>
        ))}
      </div>
    </WorkbenchSystemOverviewCard>
  );
}
