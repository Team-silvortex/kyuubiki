"use client";

export type WorkflowActivityLogEventKind =
  | "bridge_contract_normalized"
  | "control_flow_edge_updated"
  | "control_flow_node_added"
  | "control_flow_plane_inserted"
  | "workflow_imported"
  | "validation_fix_applied"
  | "package_residual_scanned"
  | "package_residual_repaired"
  | "snapshot_saved";

export type WorkflowActivityLogEntry = {
  id: string;
  at: string;
  workflowId: string;
  kind: WorkflowActivityLogEventKind;
  message: string;
  detail?: string;
  count?: number;
  context?: Record<string, unknown>;
};

const STORAGE_KEY = "kyuubiki-workflow-activity-log";
const ENTRY_LIMIT = 240;
const EVENT_KINDS = new Set<WorkflowActivityLogEventKind>([
  "bridge_contract_normalized",
  "control_flow_edge_updated",
  "control_flow_node_added",
  "control_flow_plane_inserted",
  "workflow_imported",
  "validation_fix_applied",
  "package_residual_scanned",
  "package_residual_repaired",
  "snapshot_saved",
]);
let activityLogSequence = 0;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asWorkflowActivityLogEntry(value: unknown): WorkflowActivityLogEntry | null {
  if (!isRecord(value)) return null;
  if (
    typeof value.id !== "string" ||
    typeof value.at !== "string" ||
    !Number.isFinite(Date.parse(value.at)) ||
    typeof value.workflowId !== "string" ||
    typeof value.kind !== "string" ||
    !EVENT_KINDS.has(value.kind as WorkflowActivityLogEventKind) ||
    typeof value.message !== "string" ||
    (value.detail !== undefined && typeof value.detail !== "string") ||
    (value.count !== undefined && (typeof value.count !== "number" || !Number.isFinite(value.count))) ||
    (value.context !== undefined && !isRecord(value.context))
  ) {
    return null;
  }
  return {
    id: value.id,
    at: value.at,
    workflowId: value.workflowId,
    kind: value.kind as WorkflowActivityLogEventKind,
    message: value.message,
    ...(typeof value.detail === "string" ? { detail: value.detail } : {}),
    ...(typeof value.count === "number" ? { count: value.count } : {}),
    ...(isRecord(value.context) ? { context: value.context } : {}),
  };
}

export function readWorkflowActivityLog(workflowId?: string | null) {
  if (typeof window === "undefined") return [] as WorkflowActivityLogEntry[];
  try {
    const raw = window.sessionStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const entries = JSON.parse(raw);
    if (!Array.isArray(entries)) return [];
    const normalized = entries.map(asWorkflowActivityLogEntry).filter(Boolean) as WorkflowActivityLogEntry[];
    const bounded = normalized.slice(0, ENTRY_LIMIT);
    return workflowId ? bounded.filter((entry) => entry.workflowId === workflowId) : bounded;
  } catch {
    return [];
  }
}

export function appendWorkflowActivityLogEntry(
  entry: Omit<WorkflowActivityLogEntry, "id" | "at">,
) {
  if (typeof window === "undefined") return null;
  activityLogSequence = (activityLogSequence + 1) % Number.MAX_SAFE_INTEGER;
  const nextEntry: WorkflowActivityLogEntry = {
    id: `${Date.now()}-${activityLogSequence.toString(36)}-${Math.random().toString(36).slice(2, 8)}`,
    at: new Date().toISOString(),
    ...entry,
  };
  try {
    const current = readWorkflowActivityLog();
    window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify([nextEntry, ...current].slice(0, ENTRY_LIMIT)));
  } catch {
    // Ignore storage failures to keep workflow actions non-blocking.
  }
  return nextEntry;
}

export function buildWorkflowActivityCountSummary(
  count: number,
  noun: string,
) {
  return `${noun}: ${count}`;
}
