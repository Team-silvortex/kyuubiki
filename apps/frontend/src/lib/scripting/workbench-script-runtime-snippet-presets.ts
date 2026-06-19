"use client";

import type {
  WorkbenchScriptSnippetParameters,
  WorkbenchScriptSnippetPresetRecord,
} from "./workbench-script-runtime-types";
import { assertSnippetPresetIsSafe } from "./workbench-script-preset-security";

const WORKBENCH_SNIPPET_PRESETS_KEY = "kyuubiki-workbench-snippet-presets";

function safeReadWorkbenchSnippetPresetRecords(): WorkbenchScriptSnippetPresetRecord[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(WORKBENCH_SNIPPET_PRESETS_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    if (!Array.isArray(parsed)) return [];
    return parsed.flatMap((entry) => {
      if (!entry || typeof entry !== "object") return [];
      const candidate = entry as Partial<WorkbenchScriptSnippetPresetRecord>;
      if (
        typeof candidate.presetId !== "string" ||
        typeof candidate.projectId !== "string" ||
        typeof candidate.snippetId !== "string" ||
        typeof candidate.name !== "string" ||
        typeof candidate.updatedAt !== "string" ||
        !candidate.parameters ||
        typeof candidate.parameters !== "object" ||
        Array.isArray(candidate.parameters)
      ) {
        return [];
      }
      return [{
        presetId: candidate.presetId,
        projectId: candidate.projectId,
        snippetId: candidate.snippetId,
        name: candidate.name,
        parameters: candidate.parameters as WorkbenchScriptSnippetParameters,
        updatedAt: candidate.updatedAt,
      }];
    });
  } catch {
    return [];
  }
}

function writeWorkbenchSnippetPresetRecords(records: WorkbenchScriptSnippetPresetRecord[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(WORKBENCH_SNIPPET_PRESETS_KEY, JSON.stringify(records));
}

function normalizePresetName(name: string) {
  return name.trim().replace(/\s+/g, " ").slice(0, 72);
}

export function listWorkbenchSnippetPresets(projectId: string | null) {
  if (!projectId) return [];
  return safeReadWorkbenchSnippetPresetRecords()
    .filter((entry) => entry.projectId === projectId)
    .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

export function saveWorkbenchSnippetPreset(params: {
  projectId: string;
  snippetId: string;
  name: string;
  parameters: WorkbenchScriptSnippetParameters;
  presetId?: string;
}) {
  const projectId = params.projectId.trim();
  const snippetId = params.snippetId.trim();
  const name = normalizePresetName(params.name);
  if (!projectId) throw new Error("A project must be selected before saving a snippet preset.");
  if (!snippetId) throw new Error("Snippet preset must target a snippet id.");
  if (!name) throw new Error("Snippet preset name cannot be empty.");

  const records = safeReadWorkbenchSnippetPresetRecords();
  const now = new Date().toISOString();
  const presetId = params.presetId?.trim() || `snippet_preset_${Math.random().toString(36).slice(2, 10)}`;
  assertSnippetPresetIsSafe(params.parameters);
  const nextRecord: WorkbenchScriptSnippetPresetRecord = {
    presetId,
    projectId,
    snippetId,
    name,
    parameters: params.parameters,
    updatedAt: now,
  };
  writeWorkbenchSnippetPresetRecords([...records.filter((entry) => entry.presetId !== presetId), nextRecord]);
  return nextRecord;
}

export function deleteWorkbenchSnippetPreset(presetId: string) {
  const normalizedPresetId = presetId.trim();
  if (!normalizedPresetId) return;
  const records = safeReadWorkbenchSnippetPresetRecords();
  writeWorkbenchSnippetPresetRecords(records.filter((entry) => entry.presetId !== normalizedPresetId));
}

export function serializeWorkbenchSnippetPresetRecord(preset: WorkbenchScriptSnippetPresetRecord) {
  return JSON.stringify(preset, null, 2);
}

export function parseWorkbenchSnippetPresetRecord(value: unknown) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("Invalid snippet preset document.");
  }
  const candidate = value as Partial<WorkbenchScriptSnippetPresetRecord>;
  if (
    typeof candidate.projectId !== "string" ||
    typeof candidate.snippetId !== "string" ||
    typeof candidate.name !== "string" ||
    !candidate.parameters ||
    typeof candidate.parameters !== "object" ||
    Array.isArray(candidate.parameters)
  ) {
    throw new Error("Snippet preset document is missing required fields.");
  }
  return {
    presetId: typeof candidate.presetId === "string" && candidate.presetId.trim() ? candidate.presetId : "",
    projectId: candidate.projectId,
    snippetId: candidate.snippetId,
    name: candidate.name,
    parameters: candidate.parameters as WorkbenchScriptSnippetParameters,
    updatedAt: typeof candidate.updatedAt === "string" && candidate.updatedAt.trim() ? candidate.updatedAt : new Date().toISOString(),
  } satisfies WorkbenchScriptSnippetPresetRecord;
}
