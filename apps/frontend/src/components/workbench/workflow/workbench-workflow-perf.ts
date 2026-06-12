"use client";

import type {
  WorkflowRunRecord,
  WorkflowSurfaceTab,
} from "@/components/workbench/workflow/workbench-workflow-types";

type WorkflowPerfState = {
  surfaceIntent: Partial<Record<WorkflowSurfaceTab, number>>;
  surfaceMeasures: Partial<Record<WorkflowSurfaceTab, number>>;
  traceCardMs: number | null;
  updatedAt: string | null;
};

type WorkflowDebugBridge = {
  getState: () => {
    selectedWorkflowId: string | null;
    catalogWorkflowIds: string[];
    workflowRunCount: number;
  };
  setSurfaceTab: (tab: WorkflowSurfaceTab) => void;
  setSelectedWorkflowId: (workflowId: string | null) => void;
  replaceRuns: (runs: WorkflowRunRecord[]) => void;
};

declare global {
  interface Window {
    __kyuubikiPerf?: {
      workflow?: WorkflowPerfState;
    };
    __kyuubikiWorkflowDebug?: WorkflowDebugBridge;
  }
}

function roundDuration(value: number) {
  return Math.round(value * 1000) / 1000;
}

function ensureWorkflowPerfState(): WorkflowPerfState | null {
  if (typeof window === "undefined") return null;
  if (!window.__kyuubikiPerf) window.__kyuubikiPerf = {};
  if (!window.__kyuubikiPerf.workflow) {
    window.__kyuubikiPerf.workflow = {
      surfaceIntent: {},
      surfaceMeasures: {},
      traceCardMs: null,
      updatedAt: null,
    };
  }
  return window.__kyuubikiPerf.workflow;
}

export function markWorkflowSurfaceIntent(tab: WorkflowSurfaceTab) {
  const state = ensureWorkflowPerfState();
  if (!state || typeof performance === "undefined") return;
  state.surfaceIntent[tab] = performance.now();
}

export function measureWorkflowSurfaceReady(tab: WorkflowSurfaceTab) {
  const state = ensureWorkflowPerfState();
  if (!state || typeof performance === "undefined") return null;
  const startedAt = state.surfaceIntent[tab];
  const duration = startedAt ? roundDuration(performance.now() - startedAt) : 0;
  state.surfaceMeasures[tab] = duration;
  state.updatedAt = new Date().toISOString();
  return duration;
}

export function measureWorkflowTraceCardReady(startedAt: number) {
  const state = ensureWorkflowPerfState();
  if (!state || typeof performance === "undefined") return null;
  state.traceCardMs = roundDuration(performance.now() - startedAt);
  state.updatedAt = new Date().toISOString();
  return state.traceCardMs;
}

export function installWorkflowDebugBridge(bridge: WorkflowDebugBridge | null) {
  if (typeof window === "undefined") return;
  if (bridge) {
    window.__kyuubikiWorkflowDebug = bridge;
    return;
  }
  delete window.__kyuubikiWorkflowDebug;
}
