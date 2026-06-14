"use client";

import type {
  WorkbenchMacroPresetRecord,
  WorkbenchRecordedMacroDraft,
  WorkbenchScriptActionLogEntry,
  WorkbenchScriptMacroStep,
  WorkbenchScriptSnapshot,
} from "./workbench-script-runtime-types";
import { serializeWorkbenchPythonLiteral } from "./workbench-script-python-format";

const MACRO_TEMPLATE_EXACT_RE = /^\{\{\s*(payload|state)\.([a-zA-Z0-9_]+)\s*\}\}$/;
const MACRO_TEMPLATE_INLINE_RE = /\{\{\s*(payload|state)\.([a-zA-Z0-9_]+)\s*\}\}/g;
const WORKBENCH_MACRO_PRESETS_KEY = "kyuubiki-workbench-macro-presets";

export function buildWorkbenchRecordedMacroDraft(
  actionLog: WorkbenchScriptActionLogEntry[],
  options: {
    includedEntryIds?: string[];
    id?: string;
    maxSteps?: number;
  } = {},
): WorkbenchRecordedMacroDraft | null {
  const allowedEntryIds = options.includedEntryIds ? new Set(options.includedEntryIds) : null;
  const steps = actionLog
    .filter(
      (entry) =>
        (!allowedEntryIds || allowedEntryIds.has(entry.id)) &&
        entry.action !== "macro/run" &&
        ((entry.source === "manual" && entry.status === "completed") || entry.status === "started"),
    )
    .slice(0, options.maxSteps ?? 12)
    .reverse()
    .flatMap((entry) => {
      const payload = entry.payload;
      return [{ action: entry.action, ...(payload ? { payload } : {}) }];
    });

  if (steps.length === 0) {
    return null;
  }

  return {
    id: options.id ?? "macro/draft-from-log",
    steps,
  };
}

export function buildWorkbenchRecordedMacroDraftFromEntries(
  actionLog: WorkbenchScriptActionLogEntry[],
  options: {
    includedEntryIds?: string[];
    id?: string;
    maxSteps?: number;
    startEntryId?: string;
  } = {},
): WorkbenchRecordedMacroDraft | null {
  const startIndex = options.startEntryId ? actionLog.findIndex((entry) => entry.id === options.startEntryId) : -1;
  const timelineSlice = startIndex >= 0 ? actionLog.slice(0, startIndex + 1) : actionLog;

  return buildWorkbenchRecordedMacroDraft(timelineSlice, {
    includedEntryIds: options.includedEntryIds,
    id: options.id ?? "macro/draft-from-selection",
    maxSteps: options.maxSteps,
  });
}

export function isWorkbenchMacroStep(value: unknown): value is WorkbenchScriptMacroStep {
  if (!value || typeof value !== "object") return false;
  const candidate = value as { action?: unknown; payload?: unknown };
  if (typeof candidate.action !== "string" || candidate.action.trim().length === 0) return false;
  if (candidate.payload === undefined) return true;
  return Boolean(candidate.payload && typeof candidate.payload === "object" && !Array.isArray(candidate.payload));
}

export function parseWorkbenchRecordedMacroDraft(value: unknown): WorkbenchRecordedMacroDraft {
  if (!value || typeof value !== "object") {
    throw new Error("Invalid macro document.");
  }

  const candidate = value as { id?: unknown; steps?: unknown };
  const id = typeof candidate.id === "string" && candidate.id.trim() ? candidate.id : "macro/imported";
  const steps = Array.isArray(candidate.steps) ? candidate.steps : null;

  if (!steps || steps.length === 0) {
    throw new Error("Macro document does not contain any steps.");
  }

  const normalizedSteps = steps.map((step) => {
    if (!isWorkbenchMacroStep(step)) {
      throw new Error("Macro document contains an invalid step.");
    }
    return {
      action: step.action,
      ...(step.payload ? { payload: step.payload } : {}),
    };
  });

  return {
    id,
    steps: normalizedSteps,
  };
}

export function parseWorkbenchMacroImportDocument(value: unknown): {
  draft: WorkbenchRecordedMacroDraft;
  source: "macro" | "headless-workflow";
} {
  if (!value || typeof value !== "object") {
    throw new Error("Invalid macro document.");
  }

  const candidate = value as {
    schema_version?: unknown;
    workflow?: unknown;
  };

  if (candidate.schema_version === "kyuubiki.headless-workflow/v1") {
    return {
      draft: parseWorkbenchRecordedMacroDraft(candidate.workflow),
      source: "headless-workflow",
    };
  }

  return {
    draft: parseWorkbenchRecordedMacroDraft(value),
    source: "macro",
  };
}

export function serializeWorkbenchRecordedMacroDraft(macro: WorkbenchRecordedMacroDraft): string {
  return JSON.stringify(macro, null, 2);
}

export function serializeWorkbenchMacroPythonSnippet(macro: WorkbenchRecordedMacroDraft): string {
  return `recorded_macro = ${serializeWorkbenchPythonLiteral(macro)}\n\nawait ky.run_macro_definition(recorded_macro)\n`;
}

function safeReadWorkbenchMacroPresetRecords(): WorkbenchMacroPresetRecord[] {
  if (typeof window === "undefined") return [];

  try {
    const raw = window.localStorage.getItem(WORKBENCH_MACRO_PRESETS_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    if (!Array.isArray(parsed)) return [];

    return parsed.flatMap((entry) => {
      if (!entry || typeof entry !== "object") return [];
      const candidate = entry as Partial<WorkbenchMacroPresetRecord>;
      if (
        typeof candidate.presetId !== "string" ||
        typeof candidate.projectId !== "string" ||
        typeof candidate.name !== "string" ||
        typeof candidate.updatedAt !== "string"
      ) {
        return [];
      }

      try {
        return [
          {
            presetId: candidate.presetId,
            projectId: candidate.projectId,
            name: candidate.name,
            macro: parseWorkbenchRecordedMacroDraft(candidate.macro),
            updatedAt: candidate.updatedAt,
          },
        ];
      } catch {
        return [];
      }
    });
  } catch {
    return [];
  }
}

function writeWorkbenchMacroPresetRecords(records: WorkbenchMacroPresetRecord[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(WORKBENCH_MACRO_PRESETS_KEY, JSON.stringify(records));
}

function normalizeWorkbenchMacroPresetName(name: string) {
  return name.trim().replace(/\s+/g, " ").slice(0, 64);
}

export function listWorkbenchMacroPresets(projectId: string | null): WorkbenchMacroPresetRecord[] {
  if (!projectId) return [];

  return safeReadWorkbenchMacroPresetRecords()
    .filter((entry) => entry.projectId === projectId)
    .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

export function saveWorkbenchMacroPreset(params: {
  projectId: string;
  name: string;
  macro: WorkbenchRecordedMacroDraft;
  presetId?: string;
}): WorkbenchMacroPresetRecord {
  const projectId = params.projectId.trim();
  const name = normalizeWorkbenchMacroPresetName(params.name);

  if (!projectId) {
    throw new Error("A project must be selected before saving a macro preset.");
  }

  if (!name) {
    throw new Error("Macro preset name cannot be empty.");
  }

  const records = safeReadWorkbenchMacroPresetRecords();
  const now = new Date().toISOString();
  const presetId = params.presetId?.trim() || `preset_${Math.random().toString(36).slice(2, 10)}`;
  const nextRecord: WorkbenchMacroPresetRecord = {
    presetId,
    projectId,
    name,
    macro: parseWorkbenchRecordedMacroDraft(params.macro),
    updatedAt: now,
  };

  const nextRecords = [...records.filter((entry) => entry.presetId !== presetId), nextRecord];
  writeWorkbenchMacroPresetRecords(nextRecords);
  return nextRecord;
}

export function deleteWorkbenchMacroPreset(presetId: string) {
  const normalizedPresetId = presetId.trim();
  if (!normalizedPresetId) return;
  const records = safeReadWorkbenchMacroPresetRecords();
  writeWorkbenchMacroPresetRecords(records.filter((entry) => entry.presetId !== normalizedPresetId));
}

function resolveWorkbenchMacroTemplateString(
  value: string,
  payload: Record<string, unknown>,
  snapshot: WorkbenchScriptSnapshot,
): unknown {
  const exact = value.match(MACRO_TEMPLATE_EXACT_RE);

  if (exact) {
    const [, source, key] = exact;
    return source === "payload"
      ? payload[key]
      : snapshot[key as keyof WorkbenchScriptSnapshot];
  }

  return value.replaceAll(MACRO_TEMPLATE_INLINE_RE, (_full, source: string, key: string) => {
    const resolved =
      source === "payload"
        ? payload[key]
        : snapshot[key as keyof WorkbenchScriptSnapshot];
    return resolved === undefined || resolved === null ? "" : String(resolved);
  });
}

export function resolveWorkbenchMacroPayloadTemplates(
  value: unknown,
  payload: Record<string, unknown>,
  snapshot: WorkbenchScriptSnapshot,
): unknown {
  if (typeof value === "string") {
    return resolveWorkbenchMacroTemplateString(value, payload, snapshot);
  }

  if (Array.isArray(value)) {
    return value.map((entry) => resolveWorkbenchMacroPayloadTemplates(entry, payload, snapshot));
  }

  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, resolveWorkbenchMacroPayloadTemplates(entry, payload, snapshot)]),
    );
  }

  return value;
}
