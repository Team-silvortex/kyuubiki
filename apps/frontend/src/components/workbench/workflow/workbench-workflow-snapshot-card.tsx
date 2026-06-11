"use client";

import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { StoredWorkflowSnapshotSummary } from "@/components/workbench/workflow/workbench-workflow-snapshot-storage";

type WorkbenchWorkflowSnapshotCardProps = {
  labels: WorkflowSidebarLabels;
  snapshots: StoredWorkflowSnapshotSummary[];
  onRestoreSnapshot: (snapshotId: string) => void;
  onDeleteSnapshot: (snapshotId: string) => void;
};

export function WorkbenchWorkflowSnapshotCard({
  labels,
  snapshots,
  onRestoreSnapshot,
  onDeleteSnapshot,
}: WorkbenchWorkflowSnapshotCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact" data-workflow-snapshot-card="card">
      <div className="card-head">
        <h2>{labels.validationSnapshotsTitle}</h2>
        <span className={`status-pill status-pill--${snapshots.length > 0 ? "watch" : "good"}`}>{snapshots.length}</span>
      </div>
      {snapshots.length > 0 ? (
        <div className="sidebar-stack">
          {snapshots.map((snapshot) => (
            <div className="sidebar-card sidebar-card--compact" key={snapshot.id}>
              <div className="sidebar-list">
                <div className="sidebar-list__row">
                  <span>{snapshot.reason}</span>
                  <strong>{new Date(snapshot.createdAt).toLocaleString()}</strong>
                </div>
                {snapshot.payloadState === "summary_only" ? (
                  <div className="sidebar-list__row">
                    <span>{labels.validationSnapshotSummaryOnlyLabel}</span>
                    <strong>{labels.validationSnapshotRestoreUnavailableLabel}</strong>
                  </div>
                ) : null}
                {snapshot.summary.slice(0, 2).map((item) => (
                  <div className="sidebar-list__row" key={`${snapshot.id}:${item}`}>
                    <strong>{item}</strong>
                  </div>
                ))}
              </div>
              <div className="button-row">
                <button disabled={snapshot.payloadState === "summary_only"} onClick={() => onRestoreSnapshot(snapshot.id)} type="button">{labels.validationSnapshotRestoreLabel}</button>
                <button onClick={() => onDeleteSnapshot(snapshot.id)} type="button">{labels.validationSnapshotDeleteLabel}</button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="card-copy">{labels.validationSnapshotsEmptyLabel}</p>
      )}
    </section>
  );
}
