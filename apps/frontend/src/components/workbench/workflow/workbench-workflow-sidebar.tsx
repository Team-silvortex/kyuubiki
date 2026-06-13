"use client";

import { useEffect, useMemo, useState } from "react";
import type {
  JobState,
  WorkflowCatalogEntry,
  WorkflowGraphDefinition,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import type { HeatPlaneStudyJobInput, PlaneStudyJobInput, StudyKind } from "@/components/workbench/workbench-types";

import { WorkbenchWorkflowBuilderCard } from "@/components/workbench/workflow/workbench-workflow-builder-card";
import {
  buildWorkflowContractHealthRunFeedbackMessage,
  elevateWorkflowContractHealthLabel,
  formatWorkflowContractHealthLabel,
  rankWorkflowContractHealth,
} from "@/components/workbench/workflow/workbench-workflow-contract-health";
import { removeStoredLocalWorkflow } from "@/components/workbench/workflow/workbench-workflow-local-storage";
import {
  markWorkflowSurfaceIntent,
  measureWorkflowSurfaceReady,
} from "@/components/workbench/workflow/workbench-workflow-perf";
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
  currentStudyKind: StudyKind;
  currentHeatPlaneModel: HeatPlaneStudyJobInput;
  currentPlaneModel: PlaneStudyJobInput;
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

function formatWorkflowTags(tags?: string[]) {
  const normalized = tags?.filter(Boolean) ?? [];
  return normalized.length > 0 ? normalized.join(", ") : null;
}

function scoreWorkflowRunComplexity(run: WorkflowRunRecord) {
  if (!run.traceSummary) {
    return (run.branchDecisions?.length ?? 0) * 3 +
      (run.skippedNodes?.length ?? 0) * 2 +
      (run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ?? 0);
  }
  return run.traceSummary.branchDecisionCount * 3 +
    run.traceSummary.skippedNodeRunCount * 2 +
    run.traceSummary.derivedArtifactCount +
    Math.min(run.traceSummary.progressEventCount, 6);
}

function describeWorkflowRunComplexity(run: WorkflowRunRecord) {
  const branches = run.traceSummary?.branchDecisionCount ?? run.branchDecisions?.length ?? 0;
  const derived =
    run.traceSummary?.derivedArtifactCount ??
    run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ??
    0;
  const skipped = run.traceSummary?.skippedNodeRunCount ?? run.skippedNodes?.length ?? 0;
  const progressEvents = run.traceSummary?.progressEventCount ?? 0;
  const score = scoreWorkflowRunComplexity(run);
  const tags: Array<{ label: string; tone: "watch" | "good" | "risk" }> = [];
  if (score >= 8) tags.push({ label: "complex", tone: "risk" });
  if (branches >= 2) tags.push({ label: "branch-heavy", tone: "watch" });
  if (derived >= 3) tags.push({ label: "lineage-heavy", tone: "good" });
  if (tags.length < 2 && progressEvents >= 4) tags.push({ label: "eventful", tone: "watch" });
  if (tags.length === 0 && skipped > 0) tags.push({ label: "skip-path", tone: "watch" });
  return tags.slice(0, 2);
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
  currentStudyKind,
  currentHeatPlaneModel,
  currentPlaneModel,
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
  const [catalogMessage, setCatalogMessage] = useState<string | null>(null);
  const [builderTraceFocus, setBuilderTraceFocus] = useState<{ nodeId: string; token: number } | null>(null);
  const [builderBranchFocus, setBuilderBranchFocus] = useState<{ nodeId: string; outputId: string; token: number } | null>(null);
  const [builderDatasetFocus, setBuilderDatasetFocus] = useState<{ nodeId: string; portId: string; token: number } | null>(null);
  const latestRunStatusByWorkflowId = useMemo(() => new Map(workflowRuns.map((run) => [run.workflowId, run.status] as const)), [workflowRuns]);
  const sortedWorkflowRuns = useMemo(
    () =>
      [...workflowRuns].sort((left, right) => {
        const complexityDelta = scoreWorkflowRunComplexity(right) - scoreWorkflowRunComplexity(left);
        if (complexityDelta !== 0) return complexityDelta;
        return right.jobId.localeCompare(left.jobId);
      }),
    [workflowRuns],
  );
  function deleteCatalogLocalWorkflow(workflow: WorkflowCatalogEntry) {
    if (!workflow.local) return;
    removeStoredLocalWorkflow(workflow.local.storage_id);
    onRefreshWorkflowCatalog();
  }
  function openSurfaceTab(tab: WorkflowSurfaceTab) {
    markWorkflowSurfaceIntent(tab);
    onSurfaceTabChange(tab);
  }
  function openRunNodeInBuilder(workflowId: string, nodeId: string) {
    onSelectWorkflow(workflowId);
    setBuilderTraceFocus({ nodeId, token: Date.now() });
    openSurfaceTab("builder");
  }
  function openRunBranchInBuilder(workflowId: string, nodeId: string, outputId: string) {
    onSelectWorkflow(workflowId); setBuilderTraceFocus({ nodeId, token: Date.now() }); setBuilderBranchFocus({ nodeId, outputId, token: Date.now() }); openSurfaceTab("builder");
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
    const dynamicHealth = (workflow: WorkflowCatalogEntry) =>
      elevateWorkflowContractHealthLabel(
        formatWorkflowContractHealthLabel(workflow.capability_tags ?? workflow.local?.tags),
        latestRunStatusByWorkflowId.get(workflow.id),
      );
    const filtered =
      catalogFilter === "local"
        ? workflowCatalogEntries.filter((workflow) => Boolean(workflow.local))
        : catalogFilter === "variants"
          ? workflowCatalogEntries.filter((workflow) => Boolean(workflow.local?.variant_of_workflow_id))
          : catalogFilter === "healthy"
            ? workflowCatalogEntries.filter((workflow) => {
                const health = dynamicHealth(workflow);
                return health === "clean" || health === "manageable";
              })
            : catalogFilter === "needs_review"
              ? workflowCatalogEntries.filter((workflow) => dynamicHealth(workflow) === "needs review")
              : workflowCatalogEntries;
    return [...filtered].sort((left, right) => {
      const leftRank = rankWorkflowContractHealth({ ...left, capability_tags: [`contract_health:${dynamicHealth(left) === "needs review" ? "review" : dynamicHealth(left) ?? "manageable"}`] });
      const rightRank = rankWorkflowContractHealth({ ...right, capability_tags: [`contract_health:${dynamicHealth(right) === "needs review" ? "review" : dynamicHealth(right) ?? "manageable"}`] });
      const healthDelta = leftRank - rightRank;
      if (healthDelta !== 0) return healthDelta;
      if (left.local?.promoted_at && right.local?.promoted_at) {
        return right.local.promoted_at.localeCompare(left.local.promoted_at);
      }
      if (left.local?.promoted_at) return -1;
      if (right.local?.promoted_at) return 1;
      return left.name.localeCompare(right.name);
    });
  }, [catalogFilter, latestRunStatusByWorkflowId, workflowCatalogEntries]);
  useEffect(() => {
    if (typeof window === "undefined") return;
    let disposed = false;
    const handle = window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        if (!disposed) measureWorkflowSurfaceReady(surfaceTab);
      });
    });
    return () => {
      disposed = true;
      window.cancelAnimationFrame(handle);
    };
  }, [surfaceTab, latestRun?.jobId, filteredWorkflowCatalogEntries.length]);

  return (
    <div className="sidebar-stack panel-scroll-window">
      <div className="panel-tabs panel-tabs--editor">
        <button
          className={`panel-tab${surfaceTab === "overview" ? " panel-tab--active" : ""}`}
          onClick={() => openSurfaceTab("overview")}
          type="button"
        >
          {labels.overviewPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "catalog" ? " panel-tab--active" : ""}`}
          onClick={() => openSurfaceTab("catalog")}
          type="button"
        >
          {labels.catalogPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "builder" ? " panel-tab--active" : ""}`}
          onClick={() => openSurfaceTab("builder")}
          type="button"
        >
          {labels.builderPageLabel}
        </button>
        <button
          className={`panel-tab${surfaceTab === "runs" ? " panel-tab--active" : ""}`}
          onClick={() => openSurfaceTab("runs")}
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
              <button onClick={() => openSurfaceTab("catalog")} type="button">
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
              <button onClick={() => openSurfaceTab("builder")} type="button">
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
              <button onClick={() => openSurfaceTab("runs")} type="button">
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
          {catalogMessage ? <p className="card-copy">{catalogMessage}</p> : null}
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
            <button onClick={() => setCatalogFilter("healthy")} type="button">
              {labels.catalogFilterHealthyLabel}
            </button>
            <button onClick={() => setCatalogFilter("needs_review")} type="button">
              {labels.catalogFilterNeedsReviewLabel}
            </button>
            <button onClick={onRefreshWorkflowCatalog} type="button">
              {labels.refreshLabel}
            </button>
          </div>
          {filteredWorkflowCatalogEntries.length === 0 ? <p className="card-copy">{labels.emptyCatalogLabel}</p> : null}
          <div className="runtime-overview-grid">
            {filteredWorkflowCatalogEntries.map((workflow) => {
              const localWorkflowTags = formatWorkflowTags(workflow.local?.tags);
              const contractHealth = elevateWorkflowContractHealthLabel(formatWorkflowContractHealthLabel(workflow.capability_tags ?? workflow.local?.tags), latestRunStatusByWorkflowId.get(workflow.id));
              return (
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
                      {workflow.local.imported_from_package_id ? (
                        <div className="sidebar-list__row">
                          <span>{labels.localWorkflowPackageIdLabel}</span>
                          <strong>{workflow.local.imported_from_package_id}</strong>
                        </div>
                      ) : null}
                      {workflow.local.imported_from_package_version ? (
                        <div className="sidebar-list__row">
                          <span>{labels.localWorkflowPackageVersionLabel}</span>
                          <strong>{workflow.local.imported_from_package_version}</strong>
                        </div>
                      ) : null}
                      {localWorkflowTags ? (
                        <div className="sidebar-list__row">
                          <span>{labels.localWorkflowTagsLabel}</span>
                          <strong>{localWorkflowTags}</strong>
                        </div>
                      ) : null}
                      {contractHealth ? (
                        <div className="sidebar-list__row">
                          <span>contract health</span>
                          <strong>{contractHealth}</strong>
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
                    <button onClick={() => { setCatalogMessage(buildWorkflowContractHealthRunFeedbackMessage(workflow.name, workflow.capability_tags ?? workflow.local?.tags, latestRunStatusByWorkflowId.get(workflow.id))); onRunWorkflowCatalog(workflow.id); }} type="button">
                      {labels.runLabel}
                    </button>
                    {workflow.local ? (
                      <button onClick={() => deleteCatalogLocalWorkflow(workflow)} type="button">
                        {labels.localWorkflowDeleteLabel}
                      </button>
                    ) : null}
                  </div>
                </section>
              );
            })}
          </div>
        </section>
      ) : null}

      {surfaceTab === "builder" ? (
        <WorkbenchWorkflowBuilderCard
          currentHeatPlaneModel={currentHeatPlaneModel}
          currentPlaneModel={currentPlaneModel}
          currentStudyKind={currentStudyKind}
          labels={labels}
          operatorDescriptors={workflowOperatorDescriptors}
          recentRunStatus={selectedWorkflow ? latestRunStatusByWorkflowId.get(selectedWorkflow.id) ?? null : null}
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
            {sortedWorkflowRuns.map((run) => (
              <section className="sidebar-card sidebar-card--compact runtime-overview-card" key={run.jobId}>
                <div className="card-head">
                  <h2>{run.workflowId}</h2>
                  <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap", justifyContent: "flex-end" }}>
                    {describeWorkflowRunComplexity(run).map((tag) => (
                      <span className={`status-pill status-pill--${tag.tone}`} key={`${run.jobId}:${tag.label}`}>{tag.label}</span>
                    ))}
                    <span className={`status-pill status-pill--${workflowStatusTone(run.status)}`}>{run.status}</span>
                  </div>
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
                    <strong>{run.traceSummary?.skippedNodeRunCount ?? run.skippedNodes?.length ?? 0}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>branches</span>
                    <strong>{run.traceSummary?.branchDecisionCount ?? run.branchDecisions?.length ?? 0}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>node runs</span>
                    <strong>
                      {run.traceSummary
                        ? `${run.traceSummary.completedNodeRunCount}/${run.traceSummary.skippedNodeRunCount}`
                        : run.nodeRuns?.length ?? 0}
                    </strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>events</span>
                    <strong>{run.traceSummary?.progressEventCount ?? 0}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>phase</span>
                    <strong>{run.traceSummary?.latestProgressLabel ?? "--"}</strong>
                  </div>
                  <div className="sidebar-list__row">
                    <span>lineage</span>
                    <strong>
                      {run.traceSummary
                        ? `${run.traceSummary.rootArtifactCount}/${run.traceSummary.derivedArtifactCount}`
                        : `${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) === 0).length ?? 0}/${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ?? 0}`}
                    </strong>
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
