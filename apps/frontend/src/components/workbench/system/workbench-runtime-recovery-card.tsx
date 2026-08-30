"use client";

import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import type { WorkbenchRuntimeRecoveryState } from "@/components/workbench/workbench-runtime-recovery";
import { WorkbenchSystemOverviewCard } from "@/components/workbench/system/workbench-system-overview-card";

type WorkbenchRuntimeRecoveryCardProps = {
  copy: WorkbenchCopy;
  recovery: WorkbenchRuntimeRecoveryState;
  onRetryAll: () => void;
  onRetryHealth: () => void;
  onRetryProjects: () => void;
  onRetrySecurityEvents: () => void;
  onRetryWorkflowCatalog: () => void;
};

export function WorkbenchRuntimeRecoveryCard({
  copy,
  recovery,
  onRetryAll,
  onRetryHealth,
  onRetryProjects,
  onRetrySecurityEvents,
  onRetryWorkflowCatalog,
}: WorkbenchRuntimeRecoveryCardProps) {
  const statusLabel =
    recovery.availability === "offline"
      ? copy.offline
      : recovery.availability === "degraded"
        ? copy.stabilityWatch
        : copy.ready;

  return (
    <WorkbenchSystemOverviewCard
      className="runtime-overview-card"
      status={statusLabel}
      title={copy.suggestedFixes}
      actions={
        <>
          <button onClick={onRetryAll} type="button">{copy.refresh}</button>
          <button onClick={onRetryHealth} type="button">{copy.clusterHealth}</button>
          <button onClick={onRetryProjects} type="button">{copy.projectLibrary}</button>
          <button onClick={onRetrySecurityEvents} type="button">{copy.audit}</button>
          <button onClick={onRetryWorkflowCatalog} type="button">{copy.workflowCatalogTitle}</button>
        </>
      }
    >
      {recovery.issues.length === 0 ? (
        <p className="card-copy">{copy.diagnosticsClear}</p>
      ) : (
        <div className="sidebar-list sidebar-list--metrics">
          {recovery.issues.slice(0, 3).map((issue) => (
            <div key={`${issue.channel}:${issue.lastFailureAt}`} style={{ display: "grid", gap: "0.35rem" }}>
              <div className="sidebar-list__row">
                <span>{issue.scopeLabel}</span>
                <strong>{issue.kind}</strong>
              </div>
              <p className="card-copy" style={{ margin: 0 }}>{issue.message}</p>
              <p className="card-copy" style={{ margin: 0 }}>{copy.suggestedFixes}: {issue.recoveryHint}</p>
              <p className="card-copy" style={{ margin: 0 }}>
                {copy.failureReason}: {new Date(issue.lastFailureAt).toLocaleString()}
                {typeof issue.statusCode === "number" ? ` · ${copy.status} ${issue.statusCode}` : ""}
              </p>
            </div>
          ))}
        </div>
      )}
    </WorkbenchSystemOverviewCard>
  );
}
