"use client";

import { WORKBENCH_LOCAL_WORKFLOWS_KEY } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY } from "@/components/workbench/workflow/workbench-workflow-package-maintenance-log";
import {
  WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY,
  WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT,
  WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX,
} from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";

type WorkbenchSystemInstallLayoutCardProps = {
  title: string;
  hint: string;
  storageScopeLabel: string;
  localPathLabel: string;
  snapshotPathLabel: string;
  snapshotPayloadPathLabel: string;
  maintenancePathLabel: string;
  snapshotLimitLabel: string;
};

export function WorkbenchSystemInstallLayoutCard({
  title,
  hint,
  storageScopeLabel,
  localPathLabel,
  snapshotPathLabel,
  snapshotPayloadPathLabel,
  maintenancePathLabel,
  snapshotLimitLabel,
}: WorkbenchSystemInstallLayoutCardProps) {
  const rows = [
    [storageScopeLabel, "browser localStorage / per-user workspace profile"],
    [localPathLabel, WORKBENCH_LOCAL_WORKFLOWS_KEY],
    [snapshotPathLabel, WORKBENCH_WORKFLOW_SNAPSHOT_INDEX_KEY],
    [snapshotPayloadPathLabel, WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX],
    [maintenancePathLabel, WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY],
    [snapshotLimitLabel, String(WORKBENCH_WORKFLOW_SNAPSHOT_LIMIT)],
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
