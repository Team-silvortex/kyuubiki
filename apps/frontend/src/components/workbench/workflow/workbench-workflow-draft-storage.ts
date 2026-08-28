"use client";

import type { WorkflowGraphDefinition } from "@/lib/api";
import { asWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-import";
import type { WorkflowTemplateChainPreferenceSnapshot } from "@/components/workbench/workflow/workbench-workflow-template-chain-storage";

export const WORKBENCH_WORKFLOW_DRAFTS_KEY = "kyuubiki.workbench.workflowDrafts.v1";
export const WORKBENCH_WORKFLOW_DRAFT_LIMIT = 40;

export type StoredWorkflowDraft = {
  id: string;
  workflowId: string;
  name: string;
  savedAt: string;
  graph: WorkflowGraphDefinition;
  inputArtifactTexts?: Record<string, string>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
};

let workflowDraftIdSequence = 0;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function normalizeTimestamp(value: unknown): string | null {
  if (!isNonEmptyString(value)) return null;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? new Date(timestamp).toISOString() : null;
}

function asStringRecord(value: unknown): Record<string, string> | undefined {
  if (!isRecord(value)) return undefined;
  return Object.fromEntries(
    Object.entries(value).filter(
      ([key, entryValue]) => typeof key === "string" && typeof entryValue === "string",
    ),
  ) as Record<string, string>;
}

function asTemplateChainPreferences(
  value: unknown,
): WorkflowTemplateChainPreferenceSnapshot | undefined {
  if (!isRecord(value)) return undefined;
  const favoriteChainIds = Array.isArray(value.favoriteChainIds)
    ? value.favoriteChainIds.filter(
        (entry): entry is string => typeof entry === "string",
      )
    : [];
  const favoriteChainAliases = asStringRecord(value.favoriteChainAliases) ?? {};
  if (favoriteChainIds.length === 0 && Object.keys(favoriteChainAliases).length === 0) {
    return undefined;
  }
  return { favoriteChainIds, favoriteChainAliases };
}

function stripLegacyDraftInputs(records: StoredWorkflowDraft[]) {
  const sanitized = records.map(({ inputArtifactTexts: _inputArtifactTexts, ...entry }) => entry);
  return sanitized as StoredWorkflowDraft[];
}

function asStoredWorkflowDraft(value: unknown): StoredWorkflowDraft | null {
  if (!isRecord(value)) return null;
  if (
    !isNonEmptyString(value.id) ||
    !isNonEmptyString(value.workflowId) ||
    !isNonEmptyString(value.name)
  ) {
    return null;
  }
  const savedAt = normalizeTimestamp(value.savedAt);
  const graph = asWorkflowGraphDefinition(value.graph);
  if (!savedAt || !graph) return null;
  return {
    id: value.id,
    workflowId: value.workflowId,
    name: value.name,
    savedAt,
    graph,
    templateChainPreferences: asTemplateChainPreferences(value.templateChainPreferences),
  };
}

function normalizeStoredWorkflowDrafts(values: unknown[]): StoredWorkflowDraft[] {
  const candidates = values
    .map(asStoredWorkflowDraft)
    .filter((entry): entry is StoredWorkflowDraft => entry !== null)
    .sort((left, right) => right.savedAt.localeCompare(left.savedAt));
  const seenIds = new Set<string>();
  const records: StoredWorkflowDraft[] = [];
  for (const entry of candidates) {
    if (seenIds.has(entry.id)) continue;
    seenIds.add(entry.id);
    records.push(entry);
    if (records.length === WORKBENCH_WORKFLOW_DRAFT_LIMIT) break;
  }
  return records;
}

function readStoredDrafts(): StoredWorkflowDraft[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(WORKBENCH_WORKFLOW_DRAFTS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    const records = normalizeStoredWorkflowDrafts(parsed);
    const normalized = JSON.stringify(stripLegacyDraftInputs(records));
    if (raw !== normalized) {
      // Normalization is best effort; recoverable drafts remain usable in memory.
      writeStoredDrafts(records);
    }
    return records;
  } catch {
    return [];
  }
}

function writeStoredDrafts(records: StoredWorkflowDraft[]): boolean {
  if (typeof window === "undefined") return false;
  try {
    window.localStorage.setItem(
      WORKBENCH_WORKFLOW_DRAFTS_KEY,
      JSON.stringify(stripLegacyDraftInputs(records)),
    );
    return true;
  } catch {
    return false;
  }
}

function buildDraftName(workflowName: string, graph: WorkflowGraphDefinition): string {
  const base = graph.name?.trim() || workflowName.trim() || graph.id;
  const stamp = new Date().toISOString().slice(0, 16).replace("T", " ");
  return `${base} (${stamp})`;
}

function buildDraftId() {
  workflowDraftIdSequence = (workflowDraftIdSequence + 1) % Number.MAX_SAFE_INTEGER;
  return `draft_${Date.now()}_${workflowDraftIdSequence.toString(36)}`;
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
  inputArtifactTexts?: Record<string, string>;
  templateChainPreferences?: WorkflowTemplateChainPreferenceSnapshot;
}): StoredWorkflowDraft | null {
  const nextRecord: StoredWorkflowDraft = {
    id: buildDraftId(),
    workflowId: params.workflowId,
    name: buildDraftName(params.workflowName, params.graph),
    savedAt: new Date().toISOString(),
    graph: structuredClone(params.graph),
    templateChainPreferences: params.templateChainPreferences
      ? structuredClone(params.templateChainPreferences)
      : undefined,
  };
  const next = [nextRecord, ...readStoredDrafts()].slice(0, WORKBENCH_WORKFLOW_DRAFT_LIMIT);
  return writeStoredDrafts(next) ? nextRecord : null;
}

export function removeStoredWorkflowDraft(draftId: string): boolean {
  const current = readStoredDrafts();
  if (!current.some((entry) => entry.id === draftId)) return false;
  return writeStoredDrafts(current.filter((entry) => entry.id !== draftId));
}
