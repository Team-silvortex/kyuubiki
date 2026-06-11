"use client";

import { useMemo, useState } from "react";
import type {
  JobState,
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "@/lib/api";

import { WorkbenchWorkflowBuilderCard } from "@/components/workbench/workflow/workbench-workflow-builder-card";
import { removeStoredLocalWorkflow } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import { WorkbenchWorkflowRunTraceCard } from "@/components/workbench/workflow/workbench-workflow-run-trace-card";
import type {
  WorkflowRunRecord,
  WorkflowCatalogFilter,
  WorkflowSidebarLabels,
  WorkflowSurfaceTab,
} from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowSidebarProps = {
  surfaceTab: WorkflowSurfaceTab;
  onSurfaceTabChange: (tab: WorkflowSurfaceTab) => void;
  labels: WorkflowSidebarLabels;
  workflowCatalogEntries: WorkflowCatalogEntry[];
  workflowOperatorDescriptors?: WorkflowOperatorDescriptor[];
  workflowCatalogBusy: boolean;
  selectedWorkflowId: string | null;
  selectedWorkflow: WorkflowCatalogEntry | null;
  latestJob: JobState | null;
  latestWorkflowSummary: string | null;
  workflowRuns: WorkflowRunRecord[];
  onRefreshWorkflowCatalog: () => void;
  onSelectWorkflow: (workflowId: string) => void;
  onRunWorkflowCatalog: (workflowId: string) => void;
  onRunWorkflowDraft: (
    workflowId: string,
    graph: WorkflowGraphDefinition,
    inputArtifacts: Record<string, unknown>,
  ) => void;
  onOpenWorkflowRun: (jobId: string) => void;
};

function workflowStatusTone(status: string): string {
  if (status === "completed") return "good";
  if (status === "failed" || status === "cancelled") return "risk";
  return "watch";
}

function formatPromotedAt(value?: string) {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function WorkbenchWorkflowSidebar({
  surfaceTab,
  onSurfaceTabChange,
  labels,
  workflowCatalogEntries,
  workflowOperatorDescriptors,
  workflowCatalogBusy,
  selectedWorkflowId,
  selectedWorkflow,
  latestJob,
  latestWorkflowSummary,
  workflowRuns,
  onRefreshWorkflowCatalog,
  onSelectWorkflow,
  onRunWorkflowCatalog,
  onRunWorkflowDraft,
  onOpenWorkflowRun,
}: WorkbenchWorkflowSidebarProps) {
  const latestRun = workflowRuns[0] ?? null;
  const latestRunWorkflow = latestRun
    ? workflowCatalogEntries.find((entry) => entry.id === latestRun.workflowId) ?? null
    : null;
  const [catalogFilter, setCatalogFilter] = useState<WorkflowCatalogFilter>("all");
  const [builderTraceFocus, setBuilderTraceFocus] = useState<{ nodeId: string; token: number } | null>(null);
  const [builderBranchFocus, setBuilderBranchFocus] = useState<{ nodeId: string; outputId: string; token: number } | null>(null);
  const [builderDatasetFocus, setBuilderDatasetFocus] = useState<{ nodeId: string; portId: string; token: number } | null>(null);
  function deleteCatalogLocalWorkflow(workflow: WorkflowCatalogEntry) {
    if (!workflow.local) return;
    removeStoredLocalWorkflow(workflow.local.storage_id);
    onRefreshWorkflowCatalog();
  }
  function openRunNodeInBuilder(workflowId: string, nodeId: string) {
    onSelectWorkflow(workflowId);
    setBuilderTraceFocus({ nodeId, token: Date.now() });
    onSurfaceTabChange("builder");
  }
  function openRunBranchInBuilder(workflowId: string, nodeId: string, outputId: string) {
    onSelectWorkflow(workflowId); setBuilderTraceFocus({ nodeId, token: Date.now() }); setBuilderBranchFocus({ nodeId, outputId, token: Date.now() }); onSurfaceTabChange("builder");
  }
  function openRunLineageInBuilder(run: WorkflowRunRecord, artifactKey: string, nodeId: string) {
    const producer = run.artifactLineage?.find((entry) => entry.artifact_key === artifactKey);
    if (producer) setBuilderDatasetFocus({ nodeId: producer.node_id, portId: producer.port_id, token: Date.now() });
    const branchSource = run.artifactLineage?.find((entry) => entry.artifact_key === artifactKey)?.source_artifacts?.find((source) => source.endsWith(".if_true") || source.endsWith(".if_false"));
    if (branchSource) {
      const lastDot = branchSource.lastIndexOf(".");
      if (lastDot > 0) return openRunBranchInBuilder(run.workflowId, branchSource.slice(0, lastDot), branchSource.slice(lastDot + 1));
    }
    openRunNodeInBuilder(run.workflowId, nodeId);
  }
  const filteredWorkflowCatalogEntries = useMemo(() => {
    if (catalogFilter === "local") return workflowCatalogEntries.filter((workflow) => Boolean(workflow.local));
    if (catalogFilter === "variants") {
      return workflowCatalogEntries.filter((workflow) => Boolean(workflow.local?.variant_of_workflow_id));
    }
    return workflowCatalogEntries;
  }, [catalogFilter, workflowCatalogEntries]);

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
            <div className="button-row button-row--adaptive">
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
          <div className="button-row button-row--adaptive">
            <button onClick={() => setCatalogFilter("all")} type="button">
              {labels.catalogFilterAllLabel}
            </button>
            <button onClick={() => setCatalogFilter("local")} type="button">
              {labels.catalogFilterLocalLabel}
            </button>
            <button onClick={() => setCatalogFilter("variants")} type="button">
              {labels.catalogFilterVariantsLabel}
            </button>
            <button onClick={onRefreshWorkflowCatalog} type="button">
              {labels.refreshLabel}
            </button>
          </div>
          {filteredWorkflowCatalogEntries.length === 0 ? <p className="card-copy">{labels.emptyCatalogLabel}</p> : null}
          <div className="runtime-overview-grid">
            {filteredWorkflowCatalogEntries.map((workflow) => (
              <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={workflow.id}>
                <div className="card-head">
                  <h2>{workflow.name}</h2>
                  <span className={`status-pill status-pill--${selectedWorkflowId === workflow.id ? "good" : "watch"}`}>
                    {workflow.local ? labels.localWorkflowBadgeLabel : workflow.version}
                  </span>
                </div>
                <p className="card-copy">{workflow.summary}</p>
                {workflow.local ? (
                  <div className="sidebar-list">
                    <div className="sidebar-list__row">
                      <span>{labels.localWorkflowSourceLabel}</span>
                      <strong>{workflow.local.source_workflow_name ?? workflow.local.source_workflow_id ?? "--"}</strong>
                    </div>
                    <div className="sidebar-list__row">
                      <span>{labels.localWorkflowPromotedAtLabel}</span>
                      <strong>{formatPromotedAt(workflow.local.promoted_at)}</strong>
                    </div>
                    {workflow.local.variant_of_workflow_name || workflow.local.variant_of_workflow_id ? (
                      <div className="sidebar-list__row">
                        <span>{labels.localWorkflowVariantOfLabel}</span>
                        <strong>{workflow.local.variant_of_workflow_name ?? workflow.local.variant_of_workflow_id}</strong>
                      </div>
                    ) : null}
                  </div>
                ) : null}
                {workflow.local?.notes ? <p className="card-copy">{workflow.local.notes}</p> : null}
                <div className="button-row button-row--adaptive">
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
                  {workflow.local ? (
                    <button onClick={() => deleteCatalogLocalWorkflow(workflow)} type="button">
                      {labels.localWorkflowDeleteLabel}
                    </button>
                  ) : null}
                </div>
              </section>
            ))}
          </div>
        </section>
      ) : null}

      {surfaceTab === "builder" ? (
        <WorkbenchWorkflowBuilderCard
          labels={labels}
          operatorDescriptors={workflowOperatorDescriptors}
          onRefreshWorkflowCatalog={onRefreshWorkflowCatalog}
          onRunWorkflowCatalog={onRunWorkflowCatalog}
          onRunWorkflowDraft={onRunWorkflowDraft}
          selectedWorkflow={selectedWorkflow}
          traceFocusNodeId={builderTraceFocus?.nodeId ?? null}
          traceFocusToken={builderTraceFocus?.token}
          traceFocusBranchNodeId={builderBranchFocus?.nodeId ?? null}
          traceFocusBranchOutputId={builderBranchFocus?.outputId ?? null}
          traceFocusBranchToken={builderBranchFocus?.token}
          traceFocusDatasetNodeId={builderDatasetFocus?.nodeId ?? null}
          traceFocusDatasetPortId={builderDatasetFocus?.portId ?? null}
          traceFocusDatasetToken={builderDatasetFocus?.token}
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
          {latestRun ? <WorkbenchWorkflowRunTraceCard labels={labels} onSelectBranch={(nodeId, outputId) => openRunBranchInBuilder(latestRun.workflowId, nodeId, outputId)} onSelectLineage={(entry) => openRunLineageInBuilder(latestRun, entry.artifact_key, entry.node_id)} onSelectNode={(nodeId) => openRunNodeInBuilder(latestRun.workflowId, nodeId)} operatorDescriptors={workflowOperatorDescriptors} run={latestRun} workflow={latestRunWorkflow} /> : null}
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
                  <div className="sidebar-list__row">
                    <span>skipped</span>
                    <strong>{run.skippedNodes?.length ?? 0}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>branches</span>
                    <strong>{run.branchDecisions?.length ?? 0}</strong>
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
