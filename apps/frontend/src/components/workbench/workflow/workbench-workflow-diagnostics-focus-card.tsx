"use client";

import type { ReactNode } from "react";
import type { WorkflowResolvedDiagnosticsReport } from "@/components/workbench/workflow/workbench-workflow-diagnostics-report-contract";
import {
  buildWorkflowDiagnosticsFocusCardSummary,
  formatWorkflowDiagnosticsMetricValue,
  resolveWorkflowDiagnosticsFocusContext,
} from "@/components/workbench/workflow/workbench-workflow-diagnostics-presentation";

type WorkbenchWorkflowDiagnosticsFocusCardProps = {
  artifactKey: string;
  attention?: boolean;
  id: string;
  label: string;
  mode: "diagnostics" | "peak";
  report: WorkflowResolvedDiagnosticsReport;
  tone: ReactNode;
  value: string | number | boolean | null;
};

export function WorkbenchWorkflowDiagnosticsFocusCard({
  artifactKey,
  attention = false,
  id,
  label,
  mode,
  report,
  tone,
  value,
}: WorkbenchWorkflowDiagnosticsFocusCardProps) {
  const summary = buildWorkflowDiagnosticsFocusCardSummary(
    id,
    resolveWorkflowDiagnosticsFocusContext(report, id),
  );
  const sections = summary.sections
    .map((section) => ({
      ...section,
      lines: section.lines.slice(0, mode === "peak" ? 2 : 1),
    }))
    .filter((section) => section.lines.length > 0)
    .slice(0, mode === "peak" ? 3 : 2);

  return (
    <div
      className={`workflow-trace-diagnostics__highlight workflow-trace-diagnostics__highlight--${summary.domain}${attention ? " workflow-trace-diagnostics__highlight--attention" : ""}`}
    >
      <span className="workflow-trace-diagnostics__highlight-head">
        {tone}
        <strong>{label}</strong>
        <span className={`workflow-trace-diagnostics__domain workflow-trace-diagnostics__domain--${summary.domain}`}>
          {summary.domainLabel}
        </span>
      </span>
      <span className="card-copy">{formatWorkflowDiagnosticsMetricValue(value)}</span>
      {sections.length > 0 ? (
        <div className="workflow-trace-diagnostics__focus-grid">
          {sections.map((section, sectionIndex) => (
            <details
              className="workflow-trace-diagnostics__focus-section"
              open={sectionIndex === 0}
              key={`${artifactKey}:context:${id}:${section.label}`}
            >
              <summary className="workflow-trace-diagnostics__focus-label">{section.label}</summary>
              <div className="workflow-trace-diagnostics__focus-body">
                {section.lines.map((line, index) => (
                  <span
                    className="card-copy"
                    key={`${artifactKey}:context:${id}:${section.label}:${index}`}
                  >
                    {line}
                  </span>
                ))}
              </div>
            </details>
          ))}
        </div>
      ) : null}
    </div>
  );
}
