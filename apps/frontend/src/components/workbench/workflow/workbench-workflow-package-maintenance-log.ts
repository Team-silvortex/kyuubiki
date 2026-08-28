"use client";

export const WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY =
  "kyuubiki.workbench.workflowPackageMaintenanceLog.v1";
export const WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_PER_WORKFLOW_LIMIT = 12;
export const WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_GLOBAL_LIMIT = 120;

export type WorkflowPackageMaintenanceLogEntry = {
  id: string;
  workflowId: string;
  at: string;
  kind: "scan" | "repair";
  lines: string[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

let maintenanceLogIdSequence = 0;

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function asStoredEntry(value: unknown): WorkflowPackageMaintenanceLogEntry | null {
  if (!isRecord(value)) return null;
  if (
    !isNonEmptyString(value.id) ||
    !isNonEmptyString(value.workflowId) ||
    !isNonEmptyString(value.at) ||
    !Number.isFinite(Date.parse(value.at)) ||
    (value.kind !== "scan" && value.kind !== "repair") ||
    !Array.isArray(value.lines)
  ) {
    return null;
  }
  const lines = value.lines
    .filter((entry): entry is string => typeof entry === "string" && entry.trim().length > 0)
    .slice(0, 64);
  return {
    id: value.id,
    workflowId: value.workflowId,
    at: new Date(Date.parse(value.at)).toISOString(),
    kind: value.kind,
    lines,
  };
}

function normalizeStoredEntries(values: unknown[]): WorkflowPackageMaintenanceLogEntry[] {
  const candidates = values
    .map(asStoredEntry)
    .filter((entry): entry is WorkflowPackageMaintenanceLogEntry => entry !== null)
    .sort((left, right) => right.at.localeCompare(left.at));
  const seenIds = new Set<string>();
  const perWorkflowCounts = new Map<string, number>();
  const entries: WorkflowPackageMaintenanceLogEntry[] = [];
  for (const entry of candidates) {
    if (seenIds.has(entry.id)) continue;
    const workflowCount = perWorkflowCounts.get(entry.workflowId) ?? 0;
    if (workflowCount >= WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_PER_WORKFLOW_LIMIT) continue;
    seenIds.add(entry.id);
    perWorkflowCounts.set(entry.workflowId, workflowCount + 1);
    entries.push(entry);
    if (entries.length === WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_GLOBAL_LIMIT) break;
  }
  return entries;
}

type MaintenanceLogReadResult = {
  entries: WorkflowPackageMaintenanceLogEntry[];
  readable: boolean;
};

function readStoredEntryState(): MaintenanceLogReadResult {
  if (typeof window === "undefined") return { entries: [], readable: false };
  try {
    const raw = window.localStorage.getItem(WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY);
    if (!raw) return { entries: [], readable: true };
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return { entries: [], readable: false };
    const entries = normalizeStoredEntries(parsed);
    if (raw !== JSON.stringify(entries)) writeStoredEntries(entries);
    return { entries, readable: true };
  } catch {
    return { entries: [], readable: false };
  }
}

function readStoredEntries(): WorkflowPackageMaintenanceLogEntry[] {
  return readStoredEntryState().entries;
}

function writeStoredEntries(entries: WorkflowPackageMaintenanceLogEntry[]): boolean {
  if (typeof window === "undefined") return false;
  try {
    window.localStorage.setItem(
      WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY,
      JSON.stringify(entries),
    );
    return true;
  } catch {
    return false;
  }
}

export function buildWorkflowPackageMaintenanceLogEntryId(
  kind: WorkflowPackageMaintenanceLogEntry["kind"],
): string {
  maintenanceLogIdSequence = (maintenanceLogIdSequence + 1) % Number.MAX_SAFE_INTEGER;
  return `${kind}:${Date.now()}:${maintenanceLogIdSequence.toString(36)}`;
}

export function listStoredWorkflowPackageMaintenanceHistory(
  workflowId: string,
): WorkflowPackageMaintenanceLogEntry[] {
  return readStoredEntries()
    .filter((entry) => entry.workflowId === workflowId)
    .sort((left, right) => right.at.localeCompare(left.at));
}

export function saveStoredWorkflowPackageMaintenanceHistory(
  workflowId: string,
  history: Array<Omit<WorkflowPackageMaintenanceLogEntry, "workflowId">>,
): boolean {
  if (!isNonEmptyString(workflowId)) return false;
  const stored = readStoredEntryState();
  if (!stored.readable) return false;
  const retained = stored.entries.filter((entry) => entry.workflowId !== workflowId);
  const nextEntries = history
    .slice(0, WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_PER_WORKFLOW_LIMIT)
    .map((entry) => asStoredEntry({ ...entry, workflowId }))
    .filter((entry): entry is WorkflowPackageMaintenanceLogEntry => entry !== null);
  return writeStoredEntries(normalizeStoredEntries([...nextEntries, ...retained]));
}
