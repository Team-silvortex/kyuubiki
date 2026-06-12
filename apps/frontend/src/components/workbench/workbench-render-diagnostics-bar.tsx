"use client";

import type { WorkbenchCopy } from "./workbench-copy";
import type { ResultWindowState } from "./workbench-result-window-controller";
import {
  resolveRenderFallbackMode,
  type ViewportRenderDiagnostics,
  type ViewportRenderMode,
  type ViewportRenderStrategy,
} from "./workbench-render-diagnostics";

type WorkbenchRenderDiagnosticsBarProps = {
  t: WorkbenchCopy;
  diagnostics: ViewportRenderDiagnostics | null;
  resultWindow: ResultWindowState | null;
  renderStrategy: ViewportRenderStrategy;
  onRenderStrategyChange: (strategy: ViewportRenderStrategy) => void;
};

function modeLabel(t: WorkbenchCopy, mode: ViewportRenderMode) {
  return mode === "plane"
    ? t.renderModePlane
    : mode === "space"
      ? t.renderModeSpace
      : mode === "axial"
        ? t.renderModeAxial
        : t.renderModeLine;
}

function fallbackLabel(t: WorkbenchCopy, fallback: ReturnType<typeof resolveRenderFallbackMode>) {
  return fallback === "hybrid"
    ? t.renderFallbackHybrid
    : fallback === "chunked"
      ? t.renderFallbackChunked
      : fallback === "progressive"
        ? t.renderFallbackProgressive
        : t.renderFallbackDirect;
}

function strategyLabel(t: WorkbenchCopy, strategy: ViewportRenderStrategy) {
  return strategy === "full"
    ? t.renderStrategyFull
    : strategy === "progressive"
      ? t.renderStrategyProgressive
      : strategy === "focus"
        ? t.renderStrategyFocus
        : t.renderStrategyAuto;
}

export function WorkbenchRenderDiagnosticsBar({
  t,
  diagnostics,
  resultWindow,
  renderStrategy,
  onRenderStrategyChange,
}: WorkbenchRenderDiagnosticsBarProps) {
  if (!diagnostics) return null;

  const fallback = resolveRenderFallbackMode(diagnostics, resultWindow);

  return (
    <div className="viewport-render-bar">
      <div className="viewport-render-bar__meta">
        <strong>{t.renderDiagnosticsTitle}</strong>
        <span>
          {t.renderViewportModeLabel}: {modeLabel(t, diagnostics.mode)}
        </span>
        <span>
          {t.renderStatusLabel}: {diagnostics.progressiveActive ? t.renderStatusProgressive : t.renderStatusStable}
        </span>
        <span>
          {t.renderFallbackModeLabel}: {fallbackLabel(t, fallback)}
        </span>
        <span>
          {t.renderStrategyLabel}: {strategyLabel(t, renderStrategy)}
        </span>
        <span>
          {t.renderVisibleLabel}: {diagnostics.visibleNodes}/{diagnostics.totalNodes} {t.nodes} · {diagnostics.visibleElements}/{diagnostics.totalElements} {t.totalElements}
        </span>
        {diagnostics.progressiveActive ? (
          <span>
            {t.renderBatchLabel}: {diagnostics.progressiveBatchSize}
          </span>
        ) : null}
        {resultWindow ? (
          <span>
            {t.chunkSize}: {resultWindow.limit}
          </span>
        ) : null}
      </div>
      <div className="button-row">
        {(["auto", "full", "progressive", "focus"] as ViewportRenderStrategy[]).map((strategy) => (
          <button
            key={strategy}
            className={`ghost-button ghost-button--compact${renderStrategy === strategy ? " ghost-button--active" : ""}`}
            onClick={() => onRenderStrategyChange(strategy)}
            type="button"
          >
            {strategyLabel(t, strategy)}
          </button>
        ))}
      </div>
    </div>
  );
}
