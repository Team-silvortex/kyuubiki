"use client";

import { useEffect, useMemo } from "react";
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
import { WorkbenchWorkflowBridgeRuntimeCard } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-card";
import { buildWorkflowRunAuditReportHtml } from "@/components/workbench/workflow/workbench-workflow-run-trace-report";
import { listWorkflowSummaryArtifacts } from "@/components/workbench/workflow/workbench-workflow-summary-contract";
import {
  resolveWorkflowTraceBranchPredicateTone,
  resolveWorkflowTraceContractHealthTone,
  resolveWorkflowTraceContractWarningTone,
  resolveWorkflowTraceHeaderHealthLabel,
  resolveWorkflowTraceLineageSourceLabel,
  resolveWorkflowTraceLineageSourceTone,
  resolveWorkflowTraceNodeRunTone,
  resolveWorkflowTraceProgressStageTone,
  type WorkflowTraceStatusTone,
} from "@/components/workbench/workflow/workbench-workflow-trace-status";
import type { WorkflowRunRecord, WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import type { WorkflowCatalogEntry, WorkflowOperatorDescriptor } from "@/lib/api";

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

function formatSummaryValue(value: string | number | boolean | null) {
  return typeof value === "number" ? value.toExponential(4) : String(value);
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
  const latestBranch = run.branchDecisions?.[run.branchDecisions.length - 1] ?? null;
  const latestSkipped = run.skippedNodes?.slice(0, 3) ?? [];
  const recentNodes = run.nodeRuns?.slice(-3).reverse() ?? [];
  const recentLineage = run.artifactLineage?.slice(-3).reverse() ?? [];
  const lineageByArtifactKey = useMemo(
    () =>
      new Map((run.artifactLineage ?? []).map((entry) => [entry.artifact_key, entry] as const)),
    [run.artifactLineage],
  );
  const recentSummaryArtifacts = useMemo(
    () => listWorkflowSummaryArtifacts(run.result ?? null).slice(0, 3),
    [run.result],
  );
  const previousSummaryArtifactsByKind = useMemo(
    () =>
      new Map(
        listWorkflowSummaryArtifacts(previousRun?.result ?? null).map((artifact) => [
          artifact.payload.summary_kind ?? artifact.artifactType,
          artifact,
        ] as const),
      ),
    [previousRun?.result],
  );
  const traceSummary = run.traceSummary;
  const recentProgressEvents = traceSummary?.recentProgressEvents ?? [];
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
  }, [run.jobId, recentNodes.length, recentLineage.length, recentSummaryArtifacts.length, latestSkipped.length, latestBranch?.node_id]);
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
        <div className="sidebar-list__row"><span>summary artifacts</span><strong>{recentSummaryArtifacts.length}</strong></div>
        <div className="sidebar-list__row"><span>latest phase</span><strong>{traceSummary?.latestProgressLabel ? renderStatusPill(traceSummary.latestProgressLabel, resolveWorkflowTraceProgressStageTone(recentProgressEvents[0]?.stage ?? run.status)) : "--"}</strong></div>
        <div className="sidebar-list__row"><span>lineage root/derived</span><strong>{traceSummary ? `${traceSummary.rootArtifactCount}/${traceSummary.derivedArtifactCount}` : `${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) === 0).length ?? 0}/${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ?? 0}`}</strong></div>
        <div className="sidebar-list__row"><span>static contract health</span><strong>{renderStatusPill(staticContractHealth, resolveWorkflowTraceContractHealthTone(staticContractHealth))}</strong></div>
        <div className="sidebar-list__row"><span>dynamic review state</span><strong>{renderStatusPill(dynamicReviewState, resolveWorkflowTraceContractHealthTone(dynamicReviewState))}</strong></div>
        <div className="sidebar-list__row"><span>contract warnings</span><strong>{renderStatusPill(String(contractWarningCount), resolveWorkflowTraceContractWarningTone(contractWarningCount))}</strong></div>
      </div>
      <div style={{ display: "grid", gap: "0.55rem", marginTop: "0.75rem" }}>
        <div>
          <details>
            <summary className="card-copy" style={{ cursor: recentProgressEvents.length > 0 ? "pointer" : "default" }}>
              {`recent phase timeline (${recentProgressEvents.length})`}
            </summary>
            <div style={{ display: "grid", gap: "0.35rem", marginTop: "0.55rem" }}>
              {recentProgressEvents.length === 0 ? <span className="card-copy">--</span> : null}
              {recentProgressEvents.map((event, index) => (
                <div key={`${run.jobId}:progress:${index}:${event.stage}`} style={{ display: "grid", gap: "0.2rem", padding: "0.45rem 0.55rem", borderRadius: "10px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.16))" }}>
                  <span className="card-copy" style={{ display: "flex", gap: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>
                    {renderStatusPill(event.stage, resolveWorkflowTraceProgressStageTone(event.stage))}
                    <span>{Math.round(event.progress * 100)}%</span>
                    {event.kind ? <span>{event.kind}</span> : null}
                  </span>
                  <span className="card-copy">{event.label ?? "--"}</span>
                  <span className="card-copy">{event.nodeId ? `node ${event.nodeId}` : event.emittedAt ?? "workflow event"}</span>
                </div>
              ))}
            </div>
          </details>
        </div>
        <div>
          <WorkbenchWorkflowBridgeRuntimeCard
            graph={workflow?.graph ?? null}
            onLocateIssue={(issue) => onSelectNode?.(issue.nodeId)}
            result={run.result ?? null}
          />
        </div>
        <div>
          <p className="card-copy">latest branch</p>
          {latestBranch ? <button onClick={() => onSelectBranch?.(latestBranch.node_id, latestBranch.chosen_output)} style={{ all: "unset", cursor: onSelectBranch ? "pointer" : "default" }} type="button"><p className="card-copy" style={{ display: "flex", gap: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>{renderStatusPill(latestBranch.predicate_result ? "true" : "false", resolveWorkflowTraceBranchPredicateTone(latestBranch.predicate_result))}<span>{`${latestBranch.node_id} -> ${latestBranch.chosen_output}`}</span></p></button> : <p className="card-copy">--</p>}
        </div>
        <div>
          <p className="card-copy">skipped nodes</p>
          {renderInlineList(latestSkipped)}
        </div>
        <div style={{ display: "grid", gap: "0.35rem" }}>
          <p className="card-copy">summary artifacts</p>
          {recentSummaryArtifacts.length === 0 ? <span className="card-copy">--</span> : null}
          {recentSummaryArtifacts.map((artifact) => {
            const lineageEntry = lineageByArtifactKey.get(artifact.artifactKey);
            const compareKey = artifact.payload.summary_kind ?? artifact.artifactType;
            const previousArtifact = previousSummaryArtifactsByKind.get(compareKey);
            return (
            <div key={`${run.jobId}:${artifact.artifactKey}`} style={{ display: "grid", gap: "0.2rem", padding: "0.45rem 0.55rem", borderRadius: "10px", border: "1px solid var(--line)", background: "linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.16))" }}>
              <button onClick={() => lineageEntry ? onSelectLineage?.(lineageEntry) : onSelectNode?.(artifact.nodeId ?? "")} style={{ all: "unset", cursor: onSelectLineage || onSelectNode ? "pointer" : "default", justifySelf: "start" }} type="button">
                <strong style={{ fontSize: "0.92rem" }}>{artifact.payload.summary_kind ?? artifact.artifactType}</strong>
              </button>
              <button onClick={() => lineageEntry ? onSelectLineage?.(lineageEntry) : undefined} style={{ all: "unset", cursor: onSelectLineage && lineageEntry ? "pointer" : "default", justifySelf: "start" }} type="button">
                <span className="card-copy">{artifact.artifactKey}</span>
              </button>
              <span className="card-copy" style={{ display: "flex", gap: "0.35rem", alignItems: "center", flexWrap: "wrap" }}>
                {renderStatusPill(String(Object.keys(artifact.payload.fields).length), "watch")}
                <button onClick={() => artifact.nodeId ? onSelectNode?.(artifact.nodeId) : undefined} style={{ all: "unset", cursor: onSelectNode && artifact.nodeId ? "pointer" : "default" }} type="button">
                  <span>{artifact.payload.source_operator_id ?? "unknown operator"}</span>
                </button>
              </span>
              <span className="card-copy">
                {Object.entries(artifact.payload.fields)
                  .slice(0, 2)
                  .map(([key, value]) => `${key}=${typeof value === "number" ? value.toExponential(2) : String(value)}`)
                  .join(", ") || "--"}
              </span>
              <span className="card-copy">
                {previousArtifact ? `vs previous ${previousRun?.jobId ?? "--"}` : "no previous summary match"}
              </span>
              <details>
                <summary className="card-copy" style={{ cursor: "pointer" }}>
                  contract details
                </summary>
                <div style={{ display: "grid", gap: "0.25rem", marginTop: "0.45rem" }}>
                  <span className="card-copy">
                    contract {artifact.payload.contract_version}
                  </span>
                  <span className="card-copy">
                    namespace {artifact.payload.field_namespace ?? "--"}
                  </span>
                  <div style={{ display: "grid", gap: "0.2rem" }}>
                    {Object.entries(artifact.payload.fields).map(([key, value]) => (
                      <span className="card-copy" key={`${artifact.artifactKey}:field:${key}`}>
                        {`${key}: ${formatSummaryValue(value)}`}
                      </span>
                    ))}
                  </div>
                  <div style={{ display: "grid", gap: "0.2rem" }}>
                    {previousArtifact ? (
                      Object.entries(artifact.payload.fields).map(([key, value]) => {
                        const previousValue = previousArtifact.payload.fields[key];
                        if (typeof value === "number" && typeof previousValue === "number") {
                          const delta = value - previousValue;
                          const ratio = previousValue !== 0 ? value / previousValue : null;
                          return (
                            <span className="card-copy" key={`${artifact.artifactKey}:delta:${key}`}>
                              {`${key} delta: ${delta.toExponential(4)}${ratio !== null ? ` (${ratio.toFixed(3)}x)` : ""}`}
                            </span>
                          );
                        }
                        return (
                          <span className="card-copy" key={`${artifact.artifactKey}:delta:${key}`}>
                            {`${key} previous: ${previousValue === undefined ? "--" : formatSummaryValue(previousValue)}`}
                          </span>
                        );
                      })
                    ) : (
                      <span className="card-copy">previous run comparison --</span>
                    )}
                  </div>
                  <div style={{ display: "grid", gap: "0.2rem" }}>
                    {Object.entries(artifact.payload.metadata ?? {}).length === 0 ? (
                      <span className="card-copy">metadata --</span>
                    ) : (
                      Object.entries(artifact.payload.metadata ?? {}).map(([key, value]) => (
                        <span className="card-copy" key={`${artifact.artifactKey}:meta:${key}`}>
                          {`${key}: ${String(value)}`}
                        </span>
                      ))
                    )}
                  </div>
                </div>
              </details>
            </div>
          )})}
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
