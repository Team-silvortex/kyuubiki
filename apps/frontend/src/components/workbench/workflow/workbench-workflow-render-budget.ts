"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";

export const WORKFLOW_CATALOG_RENDER_LIMIT = 80;
export const WORKFLOW_RUN_RENDER_LIMIT = 40;
export const WORKFLOW_BUILDER_DEFERRED_PANEL_DELAY_MS = 260;
export const DEEP_TRACE_PANEL_DELAY_MS = 320;
export const WORKFLOW_DEFERRED_IDLE_TIMEOUT_MS = 180;
export const WORKFLOW_DEFERRED_EDIT_RETRY_MS = 120;

export type WorkflowCatalogRenderGroup = {
  key: string;
  label: string;
  entries: WorkflowCatalogEntry[];
};

export function limitWorkflowCatalogGroups(
  groups: WorkflowCatalogRenderGroup[],
  pinnedEntries: WorkflowCatalogEntry[],
) {
  return groups.reduce<WorkflowCatalogRenderGroup[]>((renderedGroups, group) => {
    const renderedCount = renderedGroups.reduce(
      (total, entry) => total + entry.entries.length,
      pinnedEntries.length,
    );
    if (renderedCount >= WORKFLOW_CATALOG_RENDER_LIMIT) return renderedGroups;
    renderedGroups.push({
      ...group,
      entries: group.entries.slice(0, WORKFLOW_CATALOG_RENDER_LIMIT - renderedCount),
    });
    return renderedGroups;
  }, []);
}

function isWorkflowInputEditingActive() {
  const activeElement = document.activeElement;
  if (!activeElement) return false;
  if (activeElement instanceof HTMLInputElement) return true;
  if (activeElement instanceof HTMLTextAreaElement) return true;
  if (activeElement instanceof HTMLSelectElement) return true;
  return activeElement instanceof HTMLElement && activeElement.isContentEditable;
}

export function scheduleWorkflowDeferredRender(
  callback: () => void,
  delayMs: number,
) {
  let frameHandle: number | null = null;
  let idleHandle: number | null = null;
  let editRetryHandle: number | null = null;
  let disposed = false;
  const runWhenSafe = () => {
    if (disposed) return;
    if (isWorkflowInputEditingActive()) {
      editRetryHandle = window.setTimeout(runWhenSafe, WORKFLOW_DEFERRED_EDIT_RETRY_MS);
      return;
    }
    if (typeof window.requestIdleCallback === "function") {
      idleHandle = window.requestIdleCallback(
        callback,
        { timeout: WORKFLOW_DEFERRED_IDLE_TIMEOUT_MS },
      );
      return;
    }
    frameHandle = window.requestAnimationFrame(() => callback());
  };
  const timeoutHandle = window.setTimeout(runWhenSafe, delayMs);

  return () => {
    disposed = true;
    window.clearTimeout(timeoutHandle);
    if (editRetryHandle !== null) window.clearTimeout(editRetryHandle);
    if (frameHandle !== null) window.cancelAnimationFrame(frameHandle);
    if (idleHandle !== null && typeof window.cancelIdleCallback === "function") {
      window.cancelIdleCallback(idleHandle);
    }
  };
}
