"use client";

export const RECENT_OPERATOR_STORAGE_KEY = "kyuubiki.workflow.recentOperators";
export const FAVORITE_OPERATOR_STORAGE_KEY = "kyuubiki.workflow.favoriteOperators";

const RECENT_OPERATOR_LIMIT = 12;
const FAVORITE_OPERATOR_LIMIT = 16;
const OPERATOR_ID_LIMIT = 256;

export type WorkflowOperatorPreferenceReadResult = {
  operatorIds: string[];
  readable: boolean;
};

function isValidOperatorId(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= OPERATOR_ID_LIMIT &&
    value.trim() === value &&
    !/[\u0000-\u001f\u007f]/u.test(value)
  );
}

function normalizeOperatorIds(value: unknown, limit: number) {
  if (!Array.isArray(value)) return [];
  const seen = new Set<string>();
  const operatorIds: string[] = [];
  for (const entry of value) {
    if (!isValidOperatorId(entry) || seen.has(entry)) continue;
    seen.add(entry);
    operatorIds.push(entry);
    if (operatorIds.length >= limit) break;
  }
  return operatorIds;
}

function readOperatorIds(key: string, limit: number): WorkflowOperatorPreferenceReadResult {
  if (typeof window === "undefined") return { operatorIds: [], readable: false };
  try {
    const raw = window.localStorage.getItem(key);
    if (raw === null) return { operatorIds: [], readable: true };
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return { operatorIds: [], readable: false };
    const operatorIds = normalizeOperatorIds(parsed, limit);
    const readable =
      parsed.length <= limit &&
      operatorIds.length === parsed.length &&
      operatorIds.every((operatorId, index) => operatorId === parsed[index]);
    return { operatorIds, readable };
  } catch {
    return { operatorIds: [], readable: false };
  }
}

function persistOperatorIds(key: string, operatorIds: string[], limit: number): boolean {
  if (typeof window === "undefined") return false;
  const normalized = normalizeOperatorIds(operatorIds, limit);
  if (
    normalized.length !== operatorIds.length ||
    normalized.some((operatorId, index) => operatorId !== operatorIds[index])
  ) {
    return false;
  }
  try {
    window.localStorage.setItem(key, JSON.stringify(operatorIds));
    return true;
  } catch {
    return false;
  }
}

export function readRecentWorkflowOperatorIds() {
  return readOperatorIds(RECENT_OPERATOR_STORAGE_KEY, RECENT_OPERATOR_LIMIT);
}

export function readFavoriteWorkflowOperatorIds() {
  return readOperatorIds(FAVORITE_OPERATOR_STORAGE_KEY, FAVORITE_OPERATOR_LIMIT);
}

export function persistRecentWorkflowOperatorIds(operatorIds: string[]) {
  return persistOperatorIds(RECENT_OPERATOR_STORAGE_KEY, operatorIds, RECENT_OPERATOR_LIMIT);
}

export function persistFavoriteWorkflowOperatorIds(operatorIds: string[]) {
  return persistOperatorIds(FAVORITE_OPERATOR_STORAGE_KEY, operatorIds, FAVORITE_OPERATOR_LIMIT);
}

export function prependRecentWorkflowOperatorId(current: string[], operatorId: string) {
  if (!isValidOperatorId(operatorId)) return current;
  return [operatorId, ...current.filter((value) => value !== operatorId)].slice(
    0,
    RECENT_OPERATOR_LIMIT,
  );
}

export function toggleFavoriteWorkflowOperatorId(current: string[], operatorId: string) {
  if (!isValidOperatorId(operatorId)) return current;
  return current.includes(operatorId)
    ? current.filter((value) => value !== operatorId)
    : [operatorId, ...current].slice(0, FAVORITE_OPERATOR_LIMIT);
}
