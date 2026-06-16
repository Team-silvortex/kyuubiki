"use client";

import { WORKBENCH_FRONTEND_DSL_REPORT_PREFIX } from "@/lib/scripting/workbench-script-runtime";

export type WorkbenchScriptLayoutReportSummary = {
  anchors: string | null;
  activeSidebar: string | null;
  failureCode: string | null;
  failureReason: string | null;
  recoverySuggestion: string | null;
  runtimeTabCount: string | null;
  immersiveMode: string | null;
  overviewTabLabel: string | null;
  reportedAt: string | null;
  selectedTruss3dNodes: string | null;
  status: "passed" | "failed" | "unknown";
};

function buildRecoverySuggestion(code: string | null) {
  if (code === "selector_mismatch") return "Recheck layout anchors and selector bindings before rerunning the DSL.";
  if (code === "state_mismatch") return "Inspect the current workbench state snapshot and refresh the expected state contract.";
  if (code === "timeout") return "Increase the wait window or verify that the runtime job and message pipeline are still progressing.";
  if (code === "runtime_exception") return "Review the raw runtime error and retry after confirming the current panel context is valid.";
  return null;
}

const KEY_MAP = {
  active_sidebar: "activeSidebar",
  failure_code: "failureCode",
  failure_reason: "failureReason",
  runtime_tab_count: "runtimeTabCount",
  immersive_mode: "immersiveMode",
  overview_tab_label: "overviewTabLabel",
  reported_at: "reportedAt",
  selected_truss3d_nodes: "selectedTruss3dNodes",
} as const;

export function parseWorkbenchScriptLayoutReportSummary(
  output: string[],
): WorkbenchScriptLayoutReportSummary | null {
  const reportLines = output.filter((line) => line.startsWith(WORKBENCH_FRONTEND_DSL_REPORT_PREFIX));
  if (!reportLines.length) return null;

  const summary: WorkbenchScriptLayoutReportSummary = {
    anchors: null,
    activeSidebar: null,
    failureCode: null,
    failureReason: null,
    recoverySuggestion: null,
    runtimeTabCount: null,
    immersiveMode: null,
    overviewTabLabel: null,
    reportedAt: null,
    selectedTruss3dNodes: null,
    status: "unknown",
  };

  for (const line of reportLines.reverse()) {
    const payload = line.slice(WORKBENCH_FRONTEND_DSL_REPORT_PREFIX.length).trim();
    for (const token of payload.split(/\s+/)) {
      const [rawKey, ...rest] = token.split("=");
      const value = rest.join("=").trim();
      if (!rawKey || !value) continue;
      if (rawKey === "anchors") {
        summary.anchors = value;
        continue;
      }
      const mappedKey = KEY_MAP[rawKey as keyof typeof KEY_MAP];
      if (mappedKey) {
        summary[mappedKey] = mappedKey === "failureReason" ? decodeURIComponent(value) : value;
        continue;
      }
      if (rawKey === "status" && (value === "passed" || value === "failed" || value === "unknown")) {
        summary.status = value;
      }
    }
  }

  summary.recoverySuggestion = buildRecoverySuggestion(summary.failureCode);

  return summary;
}
