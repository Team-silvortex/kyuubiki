"use client";

import type { ResultWindowState } from "./workbench-result-window-controller";

export type ViewportRenderMode = "axial" | "line" | "plane" | "space";
export type ViewportRenderStrategy = "auto" | "full" | "progressive" | "focus";

export type ViewportRenderDiagnostics = {
  mode: ViewportRenderMode;
  strategy: ViewportRenderStrategy;
  totalNodes: number;
  totalElements: number;
  visibleNodes: number;
  visibleElements: number;
  progressiveActive: boolean;
  progressiveBatchSize: number;
};

export type RenderFallbackMode = "direct" | "progressive" | "chunked" | "hybrid";

export function resolveRenderFallbackMode(
  diagnostics: ViewportRenderDiagnostics | null,
  resultWindow: ResultWindowState | null,
): RenderFallbackMode {
  const usesProgressive = diagnostics?.progressiveActive ?? false;
  const usesChunkedWindow = Boolean(resultWindow);

  if (usesProgressive && usesChunkedWindow) return "hybrid";
  if (usesChunkedWindow) return "chunked";
  if (usesProgressive) return "progressive";
  return "direct";
}

export function strategyInitialRenderBudget(length: number, strategy: ViewportRenderStrategy) {
  const base = initialBudget(length);
  if (strategy === "full") return length;
  if (strategy === "progressive") return Math.max(60, Math.round(base * 0.75));
  if (strategy === "focus") return Math.max(40, Math.round(base * 0.45));
  return base;
}

export function strategyRenderBatchSize(length: number, strategy: ViewportRenderStrategy) {
  const base = batchSize(length);
  if (strategy === "full") return Math.max(base, length);
  if (strategy === "progressive") return Math.max(30, Math.round(base * 0.7));
  if (strategy === "focus") return Math.max(20, Math.round(base * 0.4));
  return base;
}

export function strategyResultWindowLimit(limit: number, strategy: ViewportRenderStrategy, totalItems?: number) {
  const next =
    strategy === "full"
      ? Math.round(limit * 1.35)
      : strategy === "progressive"
        ? Math.round(limit * 0.8)
        : strategy === "focus"
          ? Math.round(limit * 0.55)
          : limit;
  const bounded = Math.max(120, Math.round(next / 60) * 60);
  return totalItems ? Math.min(totalItems, bounded) : bounded;
}

export function strategyHotspotLimit(limit: number, strategy: ViewportRenderStrategy) {
  if (strategy === "full") return Math.max(limit, 10);
  if (strategy === "focus") return Math.min(limit, 3);
  return limit;
}

type BuildViewportRenderDiagnosticsArgs = {
  studyKind: string;
  strategy: ViewportRenderStrategy;
  axialNodeCount: number;
  lineNodeCount: number;
  lineElementCount: number;
  lineVisibleNodeCount: number;
  lineVisibleElementCount: number;
  lineProgressiveActive: boolean;
  spaceNodeCount: number;
  spaceElementCount: number;
  spaceVisibleNodeCount: number;
  spaceVisibleElementCount: number;
  spaceProgressiveActive: boolean;
  planeNodeCount: number;
  planeElementCount: number;
  planeVisibleNodeCount: number;
  planeVisibleElementCount: number;
  planeProgressiveActive: boolean;
};

export function buildViewportRenderDiagnostics(args: BuildViewportRenderDiagnosticsArgs): ViewportRenderDiagnostics {
  if (args.studyKind === "axial_bar_1d") {
    return {
      mode: "axial",
      strategy: args.strategy,
      totalNodes: args.axialNodeCount,
      totalElements: Math.max(0, args.axialNodeCount - 1),
      visibleNodes: args.axialNodeCount,
      visibleElements: Math.max(0, args.axialNodeCount - 1),
      progressiveActive: false,
      progressiveBatchSize: 0,
    };
  }

  if (args.studyKind === "truss_3d" || args.studyKind === "thermal_truss_3d" || args.studyKind === "spring_3d") {
    return {
      mode: "space",
      strategy: args.strategy,
      totalNodes: args.spaceNodeCount,
      totalElements: args.spaceElementCount,
      visibleNodes: args.spaceVisibleNodeCount,
      visibleElements: args.spaceVisibleElementCount,
      progressiveActive: args.spaceProgressiveActive,
      progressiveBatchSize: Math.max(
        strategyRenderBatchSize(args.spaceNodeCount, args.strategy),
        strategyRenderBatchSize(args.spaceElementCount, args.strategy),
      ),
    };
  }

  if (args.studyKind.includes("plane_")) {
    return {
      mode: "plane",
      strategy: args.strategy,
      totalNodes: args.planeNodeCount,
      totalElements: args.planeElementCount,
      visibleNodes: args.planeVisibleNodeCount,
      visibleElements: args.planeVisibleElementCount,
      progressiveActive: args.planeProgressiveActive,
      progressiveBatchSize: Math.max(
        strategyRenderBatchSize(args.planeNodeCount, args.strategy),
        strategyRenderBatchSize(args.planeElementCount, args.strategy),
      ),
    };
  }

  return {
    mode: "line",
    strategy: args.strategy,
    totalNodes: args.lineNodeCount,
    totalElements: args.lineElementCount,
    visibleNodes: args.lineVisibleNodeCount,
    visibleElements: args.lineVisibleElementCount,
    progressiveActive: args.lineProgressiveActive,
    progressiveBatchSize: Math.max(
      strategyRenderBatchSize(args.lineNodeCount, args.strategy),
      strategyRenderBatchSize(args.lineElementCount, args.strategy),
    ),
  };
}

function initialBudget(length: number) {
  if (length >= 20_000) return 240;
  if (length >= 10_000) return 360;
  if (length >= 4_000) return 480;
  return length;
}

function batchSize(length: number) {
  if (length >= 20_000) return 240;
  if (length >= 10_000) return 360;
  if (length >= 4_000) return 480;
  if (length >= 1_500) return 240;
  return 160;
}
