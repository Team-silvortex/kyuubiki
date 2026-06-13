"use client";

import { useEffect } from "react";
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
  resolveWorkflowTraceBranchPredicateTone,
  resolveWorkflowTraceContractHealthTone,
  resolveWorkflowTraceContractWarningTone,
  resolveWorkflowTraceHeaderHealthLabel,
  resolveWorkflowTraceLineageSourceLabel,
  resolveWorkflowTraceLineageSourceTone,
  resolveWorkflowTraceNodeRunTone,
  type WorkflowTraceStatusTone,
} from "@/components/workbench/workflow/workbench-workflow-trace-status";
import type { WorkflowRunRecord, WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowCatalogEntry, WorkflowOperatorDescriptor } from "@/lib/api";

type WorkbenchWorkflowRunTraceCardProps = {
  labels: WorkflowSidebarLabels;
  run: WorkflowRunRecord;
  workflow?: WorkflowCatalogEntry | null;
  operatorDescriptors?: WorkflowOperatorDescriptor[];
  onSelectNode?: (nodeId: string) => void;
  onSelectBranch?: (nodeId: string, outputId: string) => void;
  onSelectLineage?: (entry: NonNullable<WorkflowRunRecord["artifactLineage"]>[number]) => void;
};

function renderStatusPill(label: string, tone: WorkflowTraceStatusTone) {
  return <span className={`status-pill status-pill--${tone}`}>{label}</span>;
}

function renderInlineList(values: string[] | undefined, empty = "--") {
  if (!values || values.length === 0) return <span className="card-copy">{empty}</span>;
  return (
    <div style={{ display: "flex", gap: "0.35rem", flexWrap: "wrap" }}>
      {values.slice(0, 4).map((value) => (
        <span className="status-pill status-pill--watch" key={value}>{value}</span>
      ))}
    </div>
  );
}

export function WorkbenchWorkflowRunTraceCard({
  labels,
  run,
  workflow,
  operatorDescriptors,
  onSelectNode,
  onSelectBranch,
  onSelectLineage,
}: WorkbenchWorkflowRunTraceCardProps) {
  const latestBranch = run.branchDecisions?.[run.branchDecisions.length - 1] ?? null;
  const latestSkipped = run.skippedNodes?.slice(0, 3) ?? [];
  const recentNodes = run.nodeRuns?.slice(-3).reverse() ?? [];
  const recentLineage = run.artifactLineage?.slice(-3).reverse() ?? [];
  const traceSummary = run.traceSummary;
  const contractWarnings = workflow
    ? collectWorkflowInputArtifactContractWarnings({
        entryInputs: workflow.entry_inputs,
        inputArtifactTexts: workflow.local?.input_artifact_texts,
      })
    : undefined;
  const contractWarningCount = countWorkflowContractWarnings(contractWarnings);
  const staticContractHealth = formatWorkflowContractHealthSummary(contractWarnings);
  const dynamicReviewState = formatWorkflowDynamicReviewState({
    warnings: contractWarnings,
    recentRunStatus: run.status,
  });
  const headerHealthLabel = resolveWorkflowTraceHeaderHealthLabel(
    staticContractHealth,
    dynamicReviewState,
  );
  useEffect(() => {
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
  }, [run.jobId, recentNodes.length, recentLineage.length, latestSkipped.length, latestBranch?.node_id]);
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
        <div className="sidebar-list__row"><span>skipped</span><strong>{run.skippedNodes?.length ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>node runs</span><strong>{traceSummary ? `${traceSummary.completedNodeRunCount}/${traceSummary.skippedNodeRunCount}` : run.nodeRuns?.length ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>branch decisions</span><strong>{traceSummary?.branchDecisionCount ?? run.branchDecisions?.length ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>progress events</span><strong>{traceSummary?.progressEventCount ?? 0}</strong></div>
        <div className="sidebar-list__row"><span>latest phase</span><strong>{traceSummary?.latestProgressLabel ?? "--"}</strong></div>
        <div className="sidebar-list__row"><span>lineage root/derived</span><strong>{traceSummary ? `${traceSummary.rootArtifactCount}/${traceSummary.derivedArtifactCount}` : `${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) === 0).length ?? 0}/${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ?? 0}`}</strong></div>
        <div className="sidebar-list__row"><span>static contract health</span><strong>{renderStatusPill(staticContractHealth, resolveWorkflowTraceContractHealthTone(staticContractHealth))}</strong></div>
        <div className="sidebar-list__row"><span>dynamic review state</span><strong>{renderStatusPill(dynamicReviewState, resolveWorkflowTraceContractHealthTone(dynamicReviewState))}</strong></div>
        <div className="sidebar-list__row"><span>contract warnings</span><strong>{renderStatusPill(String(contractWarningCount), resolveWorkflowTraceContractWarningTone(contractWarningCount))}</strong></div>
      </div>
      <div style={{ display: "grid", gap: "0.55rem", marginTop: "0.75rem" }}>
        <div>
          <p className="card-copy">latest branch</p>
          {latestBranch ? <button onClick={() => onSelectBranch?.(latestBranch.node_id, latestBranch.chosen_output)} style={{ all: "unset", cursor: onSelectBranch ? "pointer" : "default" }} type="button"><p className="card-copy" style={{ display: "flex", gap: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>{renderStatusPill(latestBranch.predicate_result ? "true" : "false", resolveWorkflowTraceBranchPredicateTone(latestBranch.predicate_result))}<span>{`${latestBranch.node_id} -> ${latestBranch.chosen_output}`}</span></p></button> : <p className="card-copy">--</p>}
        </div>
        <div>
          <p className="card-copy">skipped nodes</p>
          {renderInlineList(latestSkipped)}
        </div>
        <div style={{ display: "grid", gap: "0.35rem" }}>
          <p className="card-copy">recent artifact lineage</p>
          {recentLineage.length === 0 ? <span className="card-copy">--</span> : null}
          {recentLineage.map((entry) => (
            <div key={`${run.jobId}:${entry.artifact_key}`} style={{ display: "grid", gap: "0.2rem", padding: "0.45rem 0.55rem", borderRadius: "10px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.16))" }}>
              <button onClick={() => onSelectLineage?.(entry)} style={{ all: "unset", cursor: onSelectLineage ? "pointer" : "default", justifySelf: "start" }} type="button">
                <strong style={{ fontSize: "0.92rem" }}>{entry.artifact_key}</strong>
              </button>
              <button onClick={() => onSelectNode?.(entry.node_id)} style={{ all: "unset", cursor: onSelectNode ? "pointer" : "default" }} type="button">
                <span className="card-copy">{entry.node_id}.{entry.port_id}</span>
              </button>
              <span className="card-copy" style={{ display: "flex", gap: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>
                {renderStatusPill(resolveWorkflowTraceLineageSourceLabel(entry.source_artifacts), resolveWorkflowTraceLineageSourceTone(entry.source_artifacts))}
                <span>{(entry.source_artifacts?.length ?? 0) > 0 ? `from ${entry.source_artifacts?.slice(0, 2).join(", ")}` : "source input / root artifact"}</span>
              </span>
            </div>
          ))}
        </div>
        <div style={{ display: "grid", gap: "0.35rem" }}>
          <p className="card-copy">recent node activity</p>
          {recentNodes.length === 0 ? <span className="card-copy">--</span> : null}
          {recentNodes.map((entry) => (
            <div key={`${run.jobId}:${entry.node_id}:${entry.status}`} style={{ display: "grid", gap: "0.2rem", padding: "0.45rem 0.55rem", borderRadius: "10px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.16))" }}>
              <button onClick={() => onSelectNode?.(entry.node_id)} style={{ all: "unset", cursor: onSelectNode ? "pointer" : "default", justifySelf: "start" }} type="button">
                <strong style={{ fontSize: "0.92rem" }}>{entry.node_id}</strong>
              </button>
              <span className="card-copy" style={{ display: "flex", gap: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>
                {renderStatusPill(entry.status, resolveWorkflowTraceNodeRunTone(entry.status))}
                <span>{entry.kind}{entry.operator_id ? ` · ${entry.operator_id}` : ""}</span>
              </span>
              <span className="card-copy">in {entry.consumed_artifacts?.length ?? 0} / out {entry.produced_artifacts?.length ?? 0}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
