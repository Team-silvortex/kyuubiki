"use client";

export type WorkbenchSecurityAuditSource = "script" | "assistant";
export type WorkbenchSecurityAuditRisk = "sensitive" | "destructive";
export type WorkbenchSecurityAuditStatus = "prompted" | "cancelled" | "completed" | "failed";

export type WorkbenchSecurityAuditEntry = {
  id: string;
  at: string;
  action: string;
  source: WorkbenchSecurityAuditSource;
  risk: WorkbenchSecurityAuditRisk;
  status: WorkbenchSecurityAuditStatus;
  note: string;
};

const STORAGE_KEY = "kyuubiki-workbench-security-audit";

export function readSecurityAuditLog(): WorkbenchSecurityAuditEntry[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.sessionStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as WorkbenchSecurityAuditEntry[]) : [];
  } catch {
    return [];
  }
}

export function writeSecurityAuditLog(entries: WorkbenchSecurityAuditEntry[]) {
  if (typeof window === "undefined") return;
  try {
    window.sessionStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // Ignore storage write failures and keep the in-memory log usable.
  }
}

export function createSecurityAuditEntry(
  partial: Omit<WorkbenchSecurityAuditEntry, "id" | "at">,
): WorkbenchSecurityAuditEntry {
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    at: new Date().toISOString(),
    ...partial,
  };
}
