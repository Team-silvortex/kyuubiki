"use client";

import { useEffect, useMemo } from "react";
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
import { WorkbenchWorkflowBridgeRuntimeCard } from "@/components/workbench/workflow/workbench-workflow-bridge-runtime-card";
import { WorkbenchWorkflowDiagnosticsFocusCard } from "@/components/workbench/workflow/workbench-workflow-diagnostics-focus-card";
import { buildWorkflowRunAuditReportHtml } from "@/components/workbench/workflow/workbench-workflow-run-trace-report";
import { listWorkflowDiagnosticsReports } from "@/components/workbench/workflow/workbench-workflow-diagnostics-report-contract";
import {
  formatWorkflowDiagnosticsMetricValue,
  orderWorkflowDiagnosticsFocusMetrics,
  orderWorkflowDiagnosticsHighlights,
  resolveWorkflowDiagnosticsFocusContext,
  resolveWorkflowDiagnosticsReportMode,
  resolveWorkflowDiagnosticsReportTitle,
  summarizeWorkflowDiagnosticsFocusContext,
  summarizeWorkflowDiagnosticsReport,
} from "@/components/workbench/workflow/workbench-workflow-diagnostics-presentation";
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

function resolveDiagnosticsGuardTone(status: string | undefined): WorkflowTraceStatusTone {
  if (status === "block") return "risk";
  if (status === "warn") return "watch";
  if (status === "pass") return "good";
  return "watch";
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
  const diagnosticsReport = useMemo(
    () => listWorkflowDiagnosticsReports(run.result ?? null)[0] ?? null,
    [run.result],
  );
  const diagnosticsPreview = useMemo(
    () => summarizeWorkflowDiagnosticsReport(diagnosticsReport),
    [diagnosticsReport],
  );
  const diagnosticsTitle = useMemo(
    () => resolveWorkflowDiagnosticsReportTitle(diagnosticsReport),
    [diagnosticsReport],
  );
  const diagnosticsMode = useMemo(
    () => resolveWorkflowDiagnosticsReportMode(diagnosticsReport),
    [diagnosticsReport],
  );
  const orderedDiagnosticsHighlights = useMemo(
    () => orderWorkflowDiagnosticsHighlights(diagnosticsReport),
    [diagnosticsReport],
  );
  const orderedDiagnosticsFocusMetrics = useMemo(
    () => orderWorkflowDiagnosticsFocusMetrics(diagnosticsReport),
    [diagnosticsReport],
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
        <div className="sidebar-list__row"><span>summary artifacts</span><strong>{recentSummaryArtifacts.length}</strong></div>
        <div className="sidebar-list__row"><span>latest phase</span><strong>{traceSummary?.latestProgressLabel ? renderStatusPill(traceSummary.latestProgressLabel, resolveWorkflowTraceProgressStageTone(recentProgressEvents[0]?.stage ?? run.status)) : "--"}</strong></div>
        <div className="sidebar-list__row"><span>lineage root/derived</span><strong>{traceSummary ? `${traceSummary.rootArtifactCount}/${traceSummary.derivedArtifactCount}` : `${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) === 0).length ?? 0}/${run.artifactLineage?.filter((entry) => (entry.source_artifacts?.length ?? 0) > 0).length ?? 0}`}</strong></div>
        <div className="sidebar-list__row"><span>static contract health</span><strong>{renderStatusPill(staticContractHealth, resolveWorkflowTraceContractHealthTone(staticContractHealth))}</strong></div>
        <div className="sidebar-list__row"><span>dynamic review state</span><strong>{renderStatusPill(dynamicReviewState, resolveWorkflowTraceContractHealthTone(dynamicReviewState))}</strong></div>
        <div className="sidebar-list__row"><span>contract warnings</span><strong>{renderStatusPill(String(contractWarningCount), resolveWorkflowTraceContractWarningTone(contractWarningCount))}</strong></div>
      </div>
      <div className="workflow-trace-panel-grid">
        <div className="workflow-trace-panel-section">
          <details className="workflow-trace-panel-section__details">
            <summary className="card-copy" style={{ cursor: recentProgressEvents.length > 0 ? "pointer" : "default" }}>
              {`recent phase timeline (${recentProgressEvents.length})`}
            </summary>
            <div className="workflow-trace-panel-stack">
              {recentProgressEvents.length === 0 ? <span className="card-copy">--</span> : null}
              {recentProgressEvents.map((event, index) => (
                <div className="workflow-trace-panel-card" key={`${run.jobId}:progress:${index}:${event.stage}`}>
                  <span className="workflow-trace-panel-card__meta">
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
        <div className="workflow-trace-panel-section">
          <WorkbenchWorkflowBridgeRuntimeCard
            graph={workflow?.graph ?? null}
            onLocateIssue={(issue) => onSelectNode?.(issue.nodeId)}
            result={run.result ?? null}
          />
        </div>
        <div className="workflow-trace-panel-section">
          <div className="workflow-trace-diagnostics">
            <div className="workflow-trace-diagnostics__head">
              <p className="card-copy">{diagnosticsTitle.toLowerCase()}</p>
              {!diagnosticsReport ? <span className="card-copy">--</span> : null}
            </div>
            {diagnosticsReport ? (
              <div className="workflow-trace-diagnostics__stack">
                <div className="workflow-trace-diagnostics__status">
                  {renderStatusPill(
                    diagnosticsReport.payload.report_guard_status ?? "report",
                    resolveDiagnosticsGuardTone(diagnosticsReport.payload.report_guard_status),
                  )}
                  {diagnosticsReport.payload.report_guard_recommendation ? (
                    <span className="card-copy">{diagnosticsReport.payload.report_guard_recommendation}</span>
                  ) : null}
                </div>
                <div className="workflow-trace-diagnostics__preview">
                  <span className="card-copy">summary preview</span>
                  <strong>{diagnosticsPreview ?? "--"}</strong>
                </div>
                <div className="workflow-trace-diagnostics__meta">
                  <span className="status-pill status-pill--watch">
                    {`${orderedDiagnosticsHighlights.length} highlights`}
                  </span>
                  <span className="status-pill status-pill--watch">
                    {`${orderedDiagnosticsFocusMetrics.length} focus metrics`}
                  </span>
                </div>
                {orderedDiagnosticsHighlights.length > 0 ? (
                  <div className="workflow-trace-diagnostics__highlights">
                    {orderedDiagnosticsHighlights.slice(0, 5).map((highlight) => (
                      <WorkbenchWorkflowDiagnosticsFocusCard
                        artifactKey={diagnosticsReport.artifactKey}
                        attention={highlight.attention}
                        id={highlight.id}
                        key={`${diagnosticsReport.artifactKey}:highlight:${highlight.id}`}
                        label={highlight.label}
                        mode={diagnosticsMode}
                        report={diagnosticsReport}
                        tone={renderStatusPill(
                          diagnosticsMode === "peak"
                            ? highlight.attention
                              ? "peak"
                              : "review"
                            : highlight.attention
                              ? "attention"
                              : "info",
                          highlight.attention ? "risk" : "watch",
                        )}
                        value={highlight.value}
                      />
                    ))}
                  </div>
                ) : null}
                {orderedDiagnosticsFocusMetrics.length > 0 ? (
                  <details className="workflow-trace-diagnostics__focus">
                    <summary className="card-copy" style={{ cursor: "pointer" }}>
                      focus metrics
                    </summary>
                    <div className="workflow-trace-diagnostics__focus-grid">
                      {orderedDiagnosticsFocusMetrics
                        .slice(0, 8)
                        .flatMap(([key, value]) => {
                          const rows = [
                            <span className="card-copy" key={`${diagnosticsReport.artifactKey}:focus:${key}`}>
                              {`${key}: ${formatWorkflowDiagnosticsMetricValue(value)}`}
                            </span>,
                          ];
                          const contextEntries = summarizeWorkflowDiagnosticsFocusContext(
                            resolveWorkflowDiagnosticsFocusContext(diagnosticsReport, key),
                          ).slice(0, 3);
                          return rows.concat(
                            contextEntries.map((line, index) => (
                              <span
                                className="card-copy"
                                key={`${diagnosticsReport.artifactKey}:focus:${key}:context:${index}`}
                              >
                                {line}
                              </span>
                            )),
                          );
                        })}
                    </div>
                  </details>
                ) : null}
              </div>
            ) : null}
          </div>
        </div>
        <div className="workflow-trace-panel-section">
          <div className="workflow-trace-panel-section__head">
            <p className="card-copy">control flow</p>
            <span className="status-pill status-pill--watch">
              {`${run.branchDecisions?.length ?? 0} branches · ${run.skippedNodes?.length ?? 0} skipped`}
            </span>
          </div>
          <div className="workflow-trace-panel-grid workflow-trace-panel-grid--control">
            <div className="workflow-trace-panel-card">
              <span className="card-copy">latest branch</span>
              {latestBranch ? (
                <button
                  className="workflow-trace-panel-card__button"
                  onClick={() => onSelectBranch?.(latestBranch.node_id, latestBranch.chosen_output)}
                  style={{ cursor: onSelectBranch ? "pointer" : "default" }}
                  type="button"
                >
                  <span className="workflow-trace-panel-card__meta">
                    {renderStatusPill(
                      latestBranch.predicate_result ? "true" : "false",
                      resolveWorkflowTraceBranchPredicateTone(latestBranch.predicate_result),
                    )}
                    <span>{`${latestBranch.node_id} -> ${latestBranch.chosen_output}`}</span>
                  </span>
                </button>
              ) : (
                <span className="card-copy">--</span>
              )}
            </div>
            <div className="workflow-trace-panel-card">
              <span className="card-copy">skipped nodes</span>
              {(latestSkipped?.length ?? 0) > 0 ? (
                <div className="workflow-trace-panel-card__meta">
                  {latestSkipped.map((value) => (
                    <span className="status-pill status-pill--watch" key={value}>
                      {value}
                    </span>
                  ))}
                </div>
              ) : (
                <span className="card-copy">--</span>
              )}
            </div>
          </div>
        </div>
        <div className="workflow-trace-panel-section">
          <div className="workflow-trace-panel-section__head">
            <p className="card-copy">summary artifacts</p>
            <span className="status-pill status-pill--watch">{recentSummaryArtifacts.length}</span>
          </div>
          {recentSummaryArtifacts.length === 0 ? <span className="card-copy">--</span> : null}
          <div className="workflow-trace-panel-stack">
          {recentSummaryArtifacts.map((artifact) => {
            const lineageEntry = lineageByArtifactKey.get(artifact.artifactKey);
            const compareKey = artifact.payload.summary_kind ?? artifact.artifactType;
            const previousArtifact = previousSummaryArtifactsByKind.get(compareKey);
            return (
            <div className="workflow-trace-panel-card workflow-trace-panel-card--summary" key={`${run.jobId}:${artifact.artifactKey}`}>
              <button className="workflow-trace-panel-card__button" onClick={() => lineageEntry ? onSelectLineage?.(lineageEntry) : onSelectNode?.(artifact.nodeId ?? "")} style={{ cursor: onSelectLineage || onSelectNode ? "pointer" : "default" }} type="button">
                <strong>{artifact.payload.summary_kind ?? artifact.artifactType}</strong>
              </button>
              <button className="workflow-trace-panel-card__button" onClick={() => lineageEntry ? onSelectLineage?.(lineageEntry) : undefined} style={{ cursor: onSelectLineage && lineageEntry ? "pointer" : "default" }} type="button">
                <span className="card-copy">{artifact.artifactKey}</span>
              </button>
              <span className="workflow-trace-panel-card__meta">
                {renderStatusPill(String(Object.keys(artifact.payload.fields).length), "watch")}
                <button className="workflow-trace-panel-card__button" onClick={() => artifact.nodeId ? onSelectNode?.(artifact.nodeId) : undefined} style={{ cursor: onSelectNode && artifact.nodeId ? "pointer" : "default" }} type="button">
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
                <div className="workflow-trace-panel-card__details">
                  <span className="card-copy">
                    contract {artifact.payload.contract_version}
                  </span>
                  <span className="card-copy">
                    namespace {artifact.payload.field_namespace ?? "--"}
                  </span>
                  <div className="workflow-trace-panel-card__details-grid">
                    {Object.entries(artifact.payload.fields).map(([key, value]) => (
                      <span className="card-copy" key={`${artifact.artifactKey}:field:${key}`}>
                        {`${key}: ${formatSummaryValue(value)}`}
                      </span>
                    ))}
                  </div>
                  <div className="workflow-trace-panel-card__details-grid">
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
                  <div className="workflow-trace-panel-card__details-grid">
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
        </div>
        <div className="workflow-trace-panel-section">
          <div className="workflow-trace-panel-section__head">
            <p className="card-copy">trace lanes</p>
            <span className="status-pill status-pill--watch">
              {`${recentLineage.length} lineage · ${recentNodes.length} node events`}
            </span>
          </div>
          <div className="workflow-trace-lanes">
            <div className="workflow-trace-panel-section">
              <div className="workflow-trace-panel-section__head">
                <p className="card-copy">recent artifact lineage</p>
                <span className="status-pill status-pill--watch">{recentLineage.length}</span>
              </div>
              {recentLineage.length === 0 ? <span className="card-copy">--</span> : null}
              <div className="workflow-trace-panel-stack">
                {recentLineage.map((entry) => (
                  <div className="workflow-trace-panel-card" key={`${run.jobId}:${entry.artifact_key}`}>
                    <button className="workflow-trace-panel-card__button" onClick={() => onSelectLineage?.(entry)} style={{ cursor: onSelectLineage ? "pointer" : "default" }} type="button">
                      <strong>{entry.artifact_key}</strong>
                    </button>
                    <button className="workflow-trace-panel-card__button" onClick={() => onSelectNode?.(entry.node_id)} style={{ cursor: onSelectNode ? "pointer" : "default" }} type="button">
                      <span className="card-copy">{entry.node_id}.{entry.port_id}</span>
                    </button>
                    <span className="workflow-trace-panel-card__meta">
                      {renderStatusPill(resolveWorkflowTraceLineageSourceLabel(entry.source_artifacts), resolveWorkflowTraceLineageSourceTone(entry.source_artifacts))}
                      <span>{(entry.source_artifacts?.length ?? 0) > 0 ? `from ${entry.source_artifacts?.slice(0, 2).join(", ")}` : "source input / root artifact"}</span>
                    </span>
                  </div>
                ))}
              </div>
            </div>
            <div className="workflow-trace-panel-section">
              <div className="workflow-trace-panel-section__head">
                <p className="card-copy">recent node activity</p>
                <span className="status-pill status-pill--watch">{recentNodes.length}</span>
              </div>
              {recentNodes.length === 0 ? <span className="card-copy">--</span> : null}
              <div className="workflow-trace-panel-stack">
                {recentNodes.map((entry) => (
                  <div className="workflow-trace-panel-card" key={`${run.jobId}:${entry.node_id}:${entry.status}`}>
                    <button className="workflow-trace-panel-card__button" onClick={() => onSelectNode?.(entry.node_id)} style={{ cursor: onSelectNode ? "pointer" : "default" }} type="button">
                      <strong>{entry.node_id}</strong>
                    </button>
                    <span className="workflow-trace-panel-card__meta">
                      {renderStatusPill(entry.status, resolveWorkflowTraceNodeRunTone(entry.status))}
                      <span>{entry.kind}{entry.operator_id ? ` · ${entry.operator_id}` : ""}</span>
                    </span>
                    <span className="card-copy">in {entry.consumed_artifacts?.length ?? 0} / out {entry.produced_artifacts?.length ?? 0}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
