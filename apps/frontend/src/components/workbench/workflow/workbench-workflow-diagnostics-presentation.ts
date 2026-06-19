import type {
  WorkflowResolvedDiagnosticsReport,
} from "@/components/workbench/workflow/workbench-workflow-diagnostics-report-contract";
import type { WorkflowSummaryArtifactFieldValue } from "@/lib/api";

export function formatWorkflowDiagnosticsMetricValue(
  value: WorkflowSummaryArtifactFieldValue,
  digits = 3,
) {
  return typeof value === "number" ? value.toExponential(digits) : String(value);
}

export function summarizeWorkflowDiagnosticsReport(
  report: WorkflowResolvedDiagnosticsReport | null | undefined,
  options?: {
    maxHighlights?: number;
    maxFocusMetrics?: number;
    digits?: number;
  },
) {
  if (!report) return null;
  const maxHighlights = options?.maxHighlights ?? 2;
  const maxFocusMetrics = options?.maxFocusMetrics ?? 3;
  const digits = options?.digits ?? 3;
  const highlightPreview = report.payload.report_highlights
    .slice(0, maxHighlights)
    .map((entry) => `${entry.label}=${formatWorkflowDiagnosticsMetricValue(entry.value, digits)}`)
    .join(", ");
  if (highlightPreview) return highlightPreview;
  const focusPreview = Object.entries(report.payload.report_focus_metrics)
    .slice(0, maxFocusMetrics)
    .map(([key, value]) => `${key}=${formatWorkflowDiagnosticsMetricValue(value, digits)}`)
    .join(", ");
  return focusPreview || null;
}
