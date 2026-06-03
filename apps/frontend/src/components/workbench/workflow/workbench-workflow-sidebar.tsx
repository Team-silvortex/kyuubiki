"use client";

import type {
  JobState,
  WorkflowCatalogEntry,
} from "@/lib/api";

import { WorkbenchWorkflowBuilderCard } from "@/components/workbench/workflow/workbench-workflow-builder-card";
import type {
  WorkflowRunRecord,
  WorkflowSidebarLabels,
  WorkflowSurfaceTab,
} from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowSidebarProps = {
  surfaceTab: WorkflowSurfaceTab;
  onSurfaceTabChange: (tab: WorkflowSurfaceTab) => void;
  labels: WorkflowSidebarLabels;
  workflowCatalogEntries: WorkflowCatalogEntry[];
  workflowCatalogBusy: boolean;
  selectedWorkflowId: string | null;
  selectedWorkflow: WorkflowCatalogEntry | null;
  latestJob: JobState | null;
  latestWorkflowSummary: string | null;
  workflowRuns: WorkflowRunRecord[];
  onRefreshWorkflowCatalog: () => void;
  onSelectWorkflow: (workflowId: string) => void;
  onRunWorkflowCatalog: (workflowId: string) => void;
  onOpenWorkflowRun: (jobId: string) => void;
};

function workflowStatusTone(status: string): string {
  if (status === "completed") return "good";
  if (status === "failed" || status === "cancelled") return "risk";
  return "watch";
}

export function WorkbenchWorkflowSidebar({
  surfaceTab,
  onSurfaceTabChange,
  labels,
  workflowCatalogEntries,
  workflowCatalogBusy,
  selectedWorkflowId,
  selectedWorkflow,
  latestJob,
  latestWorkflowSummary,
  workflowRuns,
  onRefreshWorkflowCatalog,
  onSelectWorkflow,
  onRunWorkflowCatalog,
  onOpenWorkflowRun,
}: WorkbenchWorkflowSidebarProps) {
  const latestRun = workflowRuns[0] ?? null;

  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs panel-tabs--editor">
        <button
          className={`panel-tab${surfaceTab === "overview" ? " panel-tab--active" : ""}`}
          onClick={() => onSurfaceTabChange("overview")}
          type="button"
        >
          {labels.overviewPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "catalog" ? " panel-tab--active" : ""}`}
          onClick={() => onSurfaceTabChange("catalog")}
          type="button"
        >
          {labels.catalogPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "builder" ? " panel-tab--active" : ""}`}
          onClick={() => onSurfaceTabChange("builder")}
          type="button"
        >
          {labels.builderPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "runs" ? " panel-tab--active" : ""}`}
          onClick={() => onSurfaceTabChange("runs")}
          type="button"
        >
          {labels.runsPageLabel}
        </button>
      </div>

      {surfaceTab === "overview" ? (
        <div className="runtime-overview-grid">
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{labels.catalogPageLabel}</h2>
            </div>
            <p className="card-copy">{labels.catalogHint}</p>
            <div className="button-row">
              <button onClick={() => onSurfaceTabChange("catalog")} type="button">
                {labels.catalogPageLabel}
              </button>
            </div>
          </section>
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{labels.builderPageLabel}</h2>
            </div>
            <p className="card-copy">{labels.builderHint}</p>
            <div className="button-row">
              <button onClick={() => onSurfaceTabChange("builder")} type="button">
                {labels.builderPageLabel}
              </button>
            </div>
          </section>
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{labels.runsPageLabel}</h2>
            </div>
            <p className="card-copy">{labels.runsHint}</p>
            <div className="button-row">
              <button onClick={() => onSurfaceTabChange("runs")} type="button">
                {labels.runsPageLabel}
              </button>
            </div>
          </section>
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{labels.sectionTitle}</h2>
              <span className={`status-pill status-pill--${workflowCatalogBusy ? "watch" : "good"}`}>
                {workflowCatalogBusy ? labels.statusBusyLabel : labels.statusReadyLabel}
              </span>
            </div>
            <p className="card-copy">{labels.overviewHint}</p>
            {selectedWorkflow ? (
              <div className="sidebar-list">
                <div className="sidebar-list__row">
                  <span>{labels.builderPageLabel}</span>
                  <strong>{selectedWorkflow.name}</strong>
                </div>
                <div className="sidebar-list__row">
                  <span>{labels.runsPageLabel}</span>
                  <strong>{latestRun?.status ?? "--"}</strong>
                </div>
              </div>
            ) : null}
          </section>
        </div>
      ) : null}

      {surfaceTab === "catalog" ? (
        <section className="sidebar-card sidebar-card--compact">
          <div className="card-head">
            <h2>{labels.catalogTitle}</h2>
          </div>
          <p className="card-copy">{labels.catalogHint}</p>
          <div className="button-row">
            <button onClick={onRefreshWorkflowCatalog} type="button">
              {labels.refreshLabel}
            </button>
          </div>
          {workflowCatalogEntries.length === 0 ? <p className="card-copy">{labels.emptyCatalogLabel}</p> : null}
          <div className="runtime-overview-grid">
            {workflowCatalogEntries.map((workflow) => (
              <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={workflow.id}>
                <div className="card-head">
                  <h2>{workflow.name}</h2>
                  <span className={`status-pill status-pill--${selectedWorkflowId === workflow.id ? "good" : "watch"}`}>
                    {workflow.version}
                  </span>
                </div>
                <p className="card-copy">{workflow.summary}</p>
                <div className="button-row">
                  <button
                    onClick={() => {
                      onSelectWorkflow(workflow.id);
                      onSurfaceTabChange("builder");
                    }}
                    type="button"
                  >
                    {labels.selectForBuilderLabel}
                  </button>
                  <button onClick={() => onRunWorkflowCatalog(workflow.id)} type="button">
                    {labels.runLabel}
                  </button>
                </div>
              </section>
            ))}
          </div>
        </section>
      ) : null}

      {surfaceTab === "builder" ? (
        <WorkbenchWorkflowBuilderCard
          labels={labels}
          onRunWorkflowCatalog={onRunWorkflowCatalog}
          selectedWorkflow={selectedWorkflow}
        />
      ) : null}

      {surfaceTab === "runs" ? (
        <section className="sidebar-card sidebar-card--compact">
          <div className="card-head">
            <h2>{labels.runsPageLabel}</h2>
          </div>
          <p className="card-copy">{labels.runsHint}</p>
          {latestJob ? (
            <div className="sidebar-list">
              <div className="sidebar-list__row">
                <span>{labels.progressLabel}</span>
                <strong>{Math.round((latestJob.progress ?? 0) * 100)}%</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{labels.currentNodeLabel}</span>
                <strong>{latestRun?.currentNode ?? "--"}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{labels.latestSummaryLabel}</span>
                <strong>{latestWorkflowSummary ?? "--"}</strong>
              </div>
            </div>
          ) : null}
          {workflowRuns.length === 0 ? <p className="card-copy">{labels.emptyRunsLabel}</p> : null}
          <div className="runtime-overview-grid">
            {workflowRuns.map((run) => (
              <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={run.jobId}>
                <div className="card-head">
                  <h2>{run.workflowId}</h2>
                  <span className={`status-pill status-pill--${workflowStatusTone(run.status)}`}>{run.status}</span>
                </div>
                <div className="sidebar-list">
                  <div className="sidebar-list__row">
                    <span>{labels.progressLabel}</span>
                    <strong>{Math.round(run.progress * 100)}%</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{labels.currentNodeLabel}</span>
                    <strong>{run.currentNode ?? "--"}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>{labels.latestSummaryLabel}</span>
                    <strong>{run.summary ?? "--"}</strong>
                  </div>
                </div>
                <div className="button-row">
                  <button onClick={() => onOpenWorkflowRun(run.jobId)} type="button">
                    {labels.openRunLabel}
                  </button>
                </div>
              </section>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}
