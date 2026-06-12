"use client";

export const WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY =
  "kyuubiki.workbench.workflowPackageMaintenanceLog.v1";

export type WorkflowPackageMaintenanceLogEntry = {
  id: string;
  workflowId: string;
  at: string;
  kind: "scan" | "repair";
  lines: string[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readStoredEntries(): WorkflowPackageMaintenanceLogEntry[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.flatMap((entry) => {
      if (!isRecord(entry)) return [];
      if (
        typeof entry.id !== "string" ||
        typeof entry.workflowId !== "string" ||
        typeof entry.at !== "string" ||
        (entry.kind !== "scan" && entry.kind !== "repair") ||
        !Array.isArray(entry.lines)
      ) {
        return [];
      }
      const lines = entry.lines.filter((value): value is string => typeof value === "string");
      return [{ id: entry.id, workflowId: entry.workflowId, at: entry.at, kind: entry.kind, lines }];
    });
  } catch {
    return [];
  }
}

function writeStoredEntries(entries: WorkflowPackageMaintenanceLogEntry[]) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(
    WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY,
    JSON.stringify(entries),
  );
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
) {
  const retained = readStoredEntries().filter((entry) => entry.workflowId !== workflowId);
  const nextEntries = history.slice(0, 12).map((entry) => ({ ...entry, workflowId }));
  writeStoredEntries([...nextEntries, ...retained]);
}
