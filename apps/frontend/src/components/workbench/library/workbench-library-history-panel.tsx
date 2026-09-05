"use client";

import { HistoryWorkspaceCard } from "@/components/workbench/library/workbench-history-workspace-card";
import type { WorkbenchLibrarySidebarProps } from "@/components/workbench/library/workbench-library-sidebar-types";
import { VirtualList } from "@/components/ui/virtual-list";

type WorkbenchLibraryHistoryPanelProps = Pick<
  WorkbenchLibrarySidebarProps,
  "activeJobId" | "jobCount" | "jobRows" | "labels" | "onOpenHistoryJob"
> & {
  mode: "jobs" | "results";
};

export function WorkbenchLibraryHistoryPanel({
  activeJobId,
  jobCount,
  jobRows,
  labels,
  mode,
  onOpenHistoryJob,
}: WorkbenchLibraryHistoryPanelProps) {
  const resultRows = jobRows.filter((row) => row.hasResult === labels.yes);
  const latestRow = mode === "jobs" ? jobRows[0] ?? null : resultRows[0] ?? null;
  const rows = mode === "jobs" ? jobRows : resultRows;
  const title = mode === "jobs" ? labels.jobWorkspaceTitle : labels.resultWorkspaceTitle;
  const hint = mode === "jobs" ? labels.jobWorkspaceHint : labels.resultWorkspaceHint;
  const actionLabel = mode === "jobs" ? labels.openLatestJob : labels.openLatestResult;
  const panelLabel = mode === "jobs" ? labels.tabs.jobs : labels.tabs.results;
  const waitingJobsCount = jobRows.filter((row) => row.hasResult === labels.no).length;
  const metrics =
    mode === "jobs"
      ? [
          { label: labels.tabs.jobs, value: jobCount },
          { label: labels.waitingJobs, value: waitingJobsCount },
        ]
      : [
          { label: labels.readyResults, value: resultRows.length },
          { label: labels.tabs.jobs, value: jobCount },
        ];

  return (
    <>
      <HistoryWorkspaceCard
        title={title}
        hint={hint}
        actionLabel={actionLabel}
        actionDisabled={!latestRow}
        onAction={() => latestRow && onOpenHistoryJob(latestRow.id)}
        metrics={metrics}
      />
      <section className="sidebar-card">
        <div className="card-head">
          <h2>{panelLabel}</h2>
          <span>{rows.length}</span>
        </div>
        <VirtualList
          className="history-list"
          items={rows}
          itemHeight={112}
          maxHeight={360}
          emptyState={<p className="card-copy">{labels.historyEmpty}</p>}
          itemKey={(historyJob) => historyJob.id}
          renderItem={(historyJob) => (
            <button
              className={`history-item${activeJobId === historyJob.id ? " history-item--active" : ""}`}
              onClick={() => onOpenHistoryJob(historyJob.id)}
              type="button"
            >
              <strong>{historyJob.shortId}</strong>
              <span>{historyJob.status}</span>
              {historyJob.statusDetail ? <small>{historyJob.statusDetail}</small> : null}
              <small>
                {labels.updatedAt}: {historyJob.updatedAt}
              </small>
              <small>
                {labels.hasResult}: {historyJob.hasResult}
              </small>
            </button>
          )}
        />
      </section>
    </>
  );
}
