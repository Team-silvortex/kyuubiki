"use client";

import { useMemo } from "react";
import type { WorkflowGraphJobResult } from "@/lib/api";
import { WorkbenchWorkflowDiagnosticsFocusCard } from "@/components/workbench/workflow/workbench-workflow-diagnostics-focus-card";
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
import type { WorkflowTraceStatusTone } from "@/components/workbench/workflow/workbench-workflow-trace-status";

type WorkbenchWorkflowTraceDiagnosticsPanelProps = {
  result?: WorkflowGraphJobResult | null;
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

export function WorkbenchWorkflowTraceDiagnosticsPanel({
  result,
}: WorkbenchWorkflowTraceDiagnosticsPanelProps) {
  const diagnosticsReport = useMemo(
    () => listWorkflowDiagnosticsReports(result ?? null)[0] ?? null,
    [result],
  );
  const diagnosticsPreview = useMemo(() => summarizeWorkflowDiagnosticsReport(diagnosticsReport), [diagnosticsReport]);
  const diagnosticsTitle = useMemo(() => resolveWorkflowDiagnosticsReportTitle(diagnosticsReport), [diagnosticsReport]);
  const diagnosticsMode = useMemo(() => resolveWorkflowDiagnosticsReportMode(diagnosticsReport), [diagnosticsReport]);
  const orderedDiagnosticsHighlights = useMemo(() => orderWorkflowDiagnosticsHighlights(diagnosticsReport), [diagnosticsReport]);
  const orderedDiagnosticsFocusMetrics = useMemo(() => orderWorkflowDiagnosticsFocusMetrics(diagnosticsReport), [diagnosticsReport]);

  return (
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
            <span className="status-pill status-pill--watch">{`${orderedDiagnosticsHighlights.length} highlights`}</span>
            <span className="status-pill status-pill--watch">{`${orderedDiagnosticsFocusMetrics.length} focus metrics`}</span>
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
                      ? highlight.attention ? "peak" : "review"
                      : highlight.attention ? "attention" : "info",
                    highlight.attention ? "risk" : "watch",
                  )}
                  value={highlight.value}
                />
              ))}
            </div>
          ) : null}
          {orderedDiagnosticsFocusMetrics.length > 0 ? (
            <details className="workflow-trace-diagnostics__focus">
              <summary className="card-copy" style={{ cursor: "pointer" }}>focus metrics</summary>
              <div className="workflow-trace-diagnostics__focus-grid">
                {orderedDiagnosticsFocusMetrics.slice(0, 8).flatMap(([key, value]) => {
                  const rows = [
                    <span className="card-copy" key={`${diagnosticsReport.artifactKey}:focus:${key}`}>
                      {`${key}: ${formatWorkflowDiagnosticsMetricValue(value)}`}
                    </span>,
                  ];
                  const contextEntries = summarizeWorkflowDiagnosticsFocusContext(
                    resolveWorkflowDiagnosticsFocusContext(diagnosticsReport, key),
                  ).slice(0, 3);
                  return rows.concat(contextEntries.map((line, index) => (
                    <span className="card-copy" key={`${diagnosticsReport.artifactKey}:focus:${key}:context:${index}`}>
                      {line}
                    </span>
                  )));
                })}
              </div>
            </details>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
