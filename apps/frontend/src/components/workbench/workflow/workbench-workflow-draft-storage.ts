"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";
import { asWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-import";

const WORKBENCH_WORKFLOW_DRAFTS_KEY = "kyuubiki.workbench.workflowDrafts.v1";

export type StoredWorkflowDraft = {
  id: string;
  workflowId: string;
  name: string;
  savedAt: string;
  graph: WorkflowGraphDefinition;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readStoredDrafts(): StoredWorkflowDraft[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(WORKBENCH_WORKFLOW_DRAFTS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.flatMap((entry) => {
      if (!isRecord(entry)) return [];
      if (
        typeof entry.id !== "string" ||
        typeof entry.workflowId !== "string" ||
        typeof entry.name !== "string" ||
        typeof entry.savedAt !== "string"
      ) {
        return [];
      }
      const graph = asWorkflowGraphDefinition(entry.graph);
      if (!graph) return [];
      return [
        {
          id: entry.id,
          workflowId: entry.workflowId,
          name: entry.name,
          savedAt: entry.savedAt,
          graph,
        },
      ];
    });
  } catch {
    return [];
  }
}

function writeStoredDrafts(records: StoredWorkflowDraft[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(WORKBENCH_WORKFLOW_DRAFTS_KEY, JSON.stringify(records));
}

function buildDraftName(workflowName: string, graph: WorkflowGraphDefinition): string {
  const base = graph.name?.trim() || workflowName.trim() || graph.id;
  const stamp = new Date().toISOString().slice(0, 16).replace("T", " ");
  return `${base} (${stamp})`;
}

export function listStoredWorkflowDrafts(workflowId: string): StoredWorkflowDraft[] {
  return readStoredDrafts()
    .filter((entry) => entry.workflowId === workflowId)
    .sort((left, right) => right.savedAt.localeCompare(left.savedAt));
}

export function saveStoredWorkflowDraft(params: {
  workflowId: string;
  workflowName: string;
  graph: WorkflowGraphDefinition;
}): StoredWorkflowDraft {
  const nextRecord: StoredWorkflowDraft = {
    id: `draft_${Date.now()}`,
    workflowId: params.workflowId,
    name: buildDraftName(params.workflowName, params.graph),
    savedAt: new Date().toISOString(),
    graph: params.graph,
  };
  const next = [nextRecord, ...readStoredDrafts()].slice(0, 40);
  writeStoredDrafts(next);
  return nextRecord;
}

export function removeStoredWorkflowDraft(draftId: string) {
  writeStoredDrafts(readStoredDrafts().filter((entry) => entry.id !== draftId));
}
