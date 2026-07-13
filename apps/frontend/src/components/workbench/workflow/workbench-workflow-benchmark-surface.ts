"use client";

import {
  DEEP_TRACE_PANEL_DELAY_MS,
  WORKFLOW_BUILDER_DEFERRED_PANEL_DELAY_MS,
  WORKFLOW_CATALOG_RENDER_LIMIT,
  WORKFLOW_RUN_RENDER_LIMIT,
} from "./workbench-workflow-render-budget.ts";

export const WORKBENCH_WORKFLOW_BENCHMARK_SURFACE_VERSION =
  "kyuubiki.workbench-workflow-benchmark-surface/v1";

export type WorkbenchWorkflowBenchmarkLane = {
  id: string;
  target: string;
  budgetMs: number;
  evidence: string[];
};

export type WorkbenchWorkflowBenchmarkSurface = {
  schemaVersion: typeof WORKBENCH_WORKFLOW_BENCHMARK_SURFACE_VERSION;
  owner: "workbench-shell";
  lanes: WorkbenchWorkflowBenchmarkLane[];
  renderLimits: {
    catalogEntries: number;
    runRecords: number;
  };
};

export function workbenchWorkflowBenchmarkSurface(): WorkbenchWorkflowBenchmarkSurface {
  return {
    schemaVersion: WORKBENCH_WORKFLOW_BENCHMARK_SURFACE_VERSION,
    owner: "workbench-shell",
    lanes: [
      {
        id: "workflow_surface_switch",
        target: "workflow tab surface ready measurement",
        budgetMs: WORKFLOW_BUILDER_DEFERRED_PANEL_DELAY_MS,
        evidence: ["markWorkflowSurfaceIntent", "measureWorkflowSurfaceReady"],
      },
      {
        id: "workflow_trace_card",
        target: "run trace card deferred render measurement",
        budgetMs: DEEP_TRACE_PANEL_DELAY_MS,
        evidence: ["measureWorkflowTraceCardReady"],
      },
      {
        id: "workflow_catalog_render",
        target: "catalog group render limit and pinned-entry budget",
        budgetMs: WORKFLOW_BUILDER_DEFERRED_PANEL_DELAY_MS,
        evidence: ["limitWorkflowCatalogGroups", "WORKFLOW_CATALOG_RENDER_LIMIT"],
      },
      {
        id: "workflow_run_history",
        target: "run history render cap for large workflow sessions",
        budgetMs: DEEP_TRACE_PANEL_DELAY_MS,
        evidence: ["WORKFLOW_RUN_RENDER_LIMIT"],
      },
    ],
    renderLimits: {
      catalogEntries: WORKFLOW_CATALOG_RENDER_LIMIT,
      runRecords: WORKFLOW_RUN_RENDER_LIMIT,
    },
  };
}
