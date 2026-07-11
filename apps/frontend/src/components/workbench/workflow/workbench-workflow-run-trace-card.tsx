"use client";

import { lazy, Suspense, useEffect, useLayoutEffect, useMemo, useState } from "react";
import {
  resolveJobStatusDetailLabel,
  resolveJobStatusDetailTone,
  resolveWorkflowRunStatusTone,
} from "@/lib/api";
import {
  downloadHtmlArtifact,
  slugifyWorkflowAssetName,
} from "@/components/workbench/workflow/workbench-workflow-builder-utils";
import {
  countWorkflowContractWarnings,
  formatWorkflowContractHealthSummary,
  formatWorkflowDynamicReviewState,
} from "@/components/workbench/workflow/workbench-workflow-contract-health";
import { collectWorkflowInputArtifactContractWarnings } from "@/components/workbench/workflow/workbench-workflow-fem-validation";
import { measureWorkflowTraceCardReady } from "@/components/workbench/workflow/workbench-workflow-perf";
import { buildWorkflowRunAuditReportHtml } from "@/components/workbench/workflow/workbench-workflow-run-trace-report";
import {
  resolveWorkflowTraceContractHealthTone,
  resolveWorkflowTraceContractWarningTone,
  resolveWorkflowTraceHeaderHealthLabel,
  resolveWorkflowTraceProgressStageTone,
  type WorkflowTraceStatusTone,
} from "@/components/workbench/workflow/workbench-workflow-trace-status";
import type { WorkflowRunRecord, WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowCatalogEntry, WorkflowOperatorDescriptor } from "@/lib/api";
import {
  DEEP_TRACE_PANEL_DELAY_MS,
  scheduleWorkflowDeferredRender,
} from "@/components/workbench/workflow/workbench-workflow-render-budget";

type WorkbenchWorkflowRunTraceCardProps = {
  labels: WorkflowSidebarLabels;
  run: WorkflowRunRecord;
  previousRun?: WorkflowRunRecord | null;
  workflow?: WorkflowCatalogEntry | null;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  onSelectNode?: (nodeId: string) => void;
  onSelectBranch?: (nodeId: string, outputId: string) => void;
  onSelectLineage?: (entry: NonNullable<WorkflowRunRecord["artifactLineage"]>[number]) => void;
};

const WorkbenchWorkflowRunTraceDeepPanels = lazy(() =>
  import("@/components/workbench/workflow/workbench-workflow-run-trace-deep-panels")
    .then((module) => ({ default: module.WorkbenchWorkflowRunTraceDeepPanels })),
);

function renderStatusPill(label: string, tone: WorkflowTraceStatusTone) {
  return <span className={`status-pill status-pill--${tone}`}>{label}</span>;
}

function TraceDeepPanelsFallback() {
  return (
    <div className="workflow-trace-panel-section">
      <div className="workflow-trace-panel-card">
        <strong>Preparing detailed trace panels</strong>
        <span className="card-copy">Bridge runtime, diagnostics, summary artifacts, lineage, and node lanes load after the first paint.</span>
      </div>
    </div>
  );
}

export function WorkbenchWorkflowRunTraceCard({
  labels,
  run,
  previousRun,
  workflow,
  operatorDescriptors,
  onSelectNode,
  onSelectBranch,
  onSelectLineage,
}: WorkbenchWorkflowRunTraceCardProps) {
  const [showDeepTracePanels, setShowDeepTracePanels] = useState(false);
  const traceSummary = run.traceSummary;
  const recentProgressEvents = traceSummary?.recentProgressEvents ?? [];
  const summaryArtifactCount = run.result ? Object.keys(run.result.artifacts ?? {}).length : 0;
  const contractWarnings = useMemo(() => workflow
    ? collectWorkflowInputArtifactContractWarnings({
        entryInputs: workflow.entry_inputs,
        inputArtifactTexts: workflow.local?.input_artifact_texts,
      })
    : undefined, [workflow]);
  const contractWarningCount = useMemo(() => countWorkflowContractWarnings(contractWarnings), [contractWarnings]);
  const staticContractHealth = useMemo(() => formatWorkflowContractHealthSummary(contractWarnings), [contractWarnings]);
  const dynamicReviewState = useMemo(() => formatWorkflowDynamicReviewState({
    warnings: contractWarnings,
    recentRunStatus: run.status,
  }), [contractWarnings, run.status]);
  const headerHealthLabel = resolveWorkflowTraceHeaderHealthLabel(
    staticContractHealth,
    dynamicReviewState,
  );
  useEffect(() => {
    setShowDeepTracePanels(false);
    return scheduleWorkflowDeferredRender(
      () => setShowDeepTracePanels(true),
      DEEP_TRACE_PANEL_DELAY_MS,
    );
  }, [run.jobId]);
  useLayoutEffect(() => {
    if (typeof window === "undefined" || typeof performance === "undefined") return;
    const startedAt = performance.now();
    let disposed = false;
    const handle = window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        if (!disposed) measureWorkflowTraceCardReady(startedAt);
      });
    });
    return () => {
      disposed = true;
      window.cancelAnimationFrame(handle);
    };
  }, [run.jobId]);
  function exportTraceReport() {
    const workflowSlug = slugifyWorkflowAssetName(run.workflowId);
    downloadHtmlArtifact(
      `${workflowSlug}-${run.jobId}-audit-report.html`,
      buildWorkflowRunAuditReportHtml({ run, workflow, operatorDescriptors }),
    );
  }
  return (
    <section className="sidebar-card sidebar-card--compact runtime-overview-card">
      <div className="card-head">
        <h2>{run.workflowId}</h2>
        <div style={{ display: "flex", gap: "0.45rem", alignItems: "center", flexWrap: "wrap" }}>
          <button onClick={exportTraceReport} type="button">export audit</button>
          {run.pollingState === "detached" ? <span className="status-pill status-pill--watch">detached</span> : null}
          {renderStatusPill(run.status, resolveWorkflowRunStatusTone(run.status, run.pollingState))}
          {resolveJobStatusDetailLabel(run.statusDetail) ? renderStatusPill(resolveJobStatusDetailLabel(run.statusDetail) ?? "--", resolveJobStatusDetailTone(run.statusDetail)) : null}
          <span className="status-pill status-pill--watch">trace</span>
          {renderStatusPill(
            headerHealthLabel,
            resolveWorkflowTraceContractHealthTone(dynamicReviewState),
          )}
        </div>
      </div>
      <div className="sidebar-list">
        <div className="sidebar-list__row"><span>{labels.progressLabel}</span><strong>{Math.round(run.progress * 100)}%</strong></div>
        <div className="sidebar-list__row"><span>{labels.currentNodeLabel}</span><strong>{run.currentNode ?? "--"}</strong></div>
        <div className="sidebar-list__row"><span>lifecycle</span><strong>{run.statusDetail?.lifecycle ?? "--"}</strong></div>
        <div className="sidebar-list__row"><span>skipped</span><strong>{run.skippedNodes?.length ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>node runs</span><strong>{traceSummary ? `${traceSummary.completedNodeRunCount}/${traceSummary.skippedNodeRunCount}` : run.nodeRuns?.length ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>branch decisions</span><strong>{traceSummary?.branchDecisionCount ?? run.branchDecisions?.length ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>progress events</span><strong>{traceSummary?.progressEventCount ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>result artifacts</span><strong>{summaryArtifactCount}</strong></div>
        <div className="sidebar-list__row"><span>latest phase</span><strong>{traceSummary?.latestProgressLabel ? renderStatusPill(traceSummary.latestProgressLabel, resolveWorkflowTraceProgressStageTone(recentProgressEvents[0]?.stage ?? run.status)) : "--"}</strong></div>
        <div className="sidebar-list__row"><span>lineage root/derived</span><strong>{traceSummary ? `${traceSummary.rootArtifactCount}/${traceSummary.derivedArtifactCount}` : `${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) === 0).length ?? 0}/${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ?? 0}`}</strong></div>
        <div className="sidebar-list__row"><span>static contract health</span><strong>{renderStatusPill(staticContractHealth, resolveWorkflowTraceContractHealthTone(staticContractHealth))}</strong></div>
        <div className="sidebar-list__row"><span>dynamic review state</span><strong>{renderStatusPill(dynamicReviewState, resolveWorkflowTraceContractHealthTone(dynamicReviewState))}</strong></div>
        <div className="sidebar-list__row"><span>contract warnings</span><strong>{renderStatusPill(String(contractWarningCount), resolveWorkflowTraceContractWarningTone(contractWarningCount))}</strong></div>
      </div>
      <div className="workflow-trace-panel-grid">
        {showDeepTracePanels ? (
          <Suspense fallback={<TraceDeepPanelsFallback />}>
            <WorkbenchWorkflowRunTraceDeepPanels
              onSelectBranch={onSelectBranch}
              onSelectLineage={onSelectLineage}
              onSelectNode={onSelectNode}
              previousRun={previousRun}
              run={run}
              workflow={workflow}
            />
          </Suspense>
        ) : (
          <TraceDeepPanelsFallback />
        )}
      </div>
    </section>
  );
}
