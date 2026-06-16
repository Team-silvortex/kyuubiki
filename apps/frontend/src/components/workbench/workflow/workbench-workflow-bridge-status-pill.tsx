"use client";

import { getBridgeRunStatusTooltipProps, getBridgeStatusSummaryTooltipProps } from "@/components/workbench/workflow/workbench-workflow-bridge-status-tooltips";

export function WorkbenchWorkflowBridgeStatusPill({
  summary,
  tone,
  mode,
  tooltipProps,
}: {
  summary: string;
  tone: "good" | "watch" | "risk";
  mode: "summary" | "run";
  tooltipProps?: { "aria-label": string; title: string };
}) {
  const resolvedTooltipProps = tooltipProps ?? (mode === "run"
    ? getBridgeRunStatusTooltipProps()
    : getBridgeStatusSummaryTooltipProps());
  return <span {...resolvedTooltipProps} className={`status-pill status-pill--${tone}`}>{summary}</span>;
}
