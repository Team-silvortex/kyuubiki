import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";

export type WorkbenchAdminRecordSource = { isCurrent: () => boolean };

export function createWorkbenchAdminRecordSources(capture = () =>
  captureWorkbenchApiAuthority("/api/v1/jobs", "/api/v1/results")) {
  const sources = new WeakMap<object, WorkbenchAdminRecordSource>();
  let latest: WorkbenchAdminRecordSource | null = null;
  return {
    mark(records: readonly object[]) {
      if (!latest?.isCurrent()) latest = { isCurrent: capture() };
      if (!latest.isCurrent()) return;
      sources.set(records, latest);
    },
    sourceOf(records: readonly object[] | null | undefined) {
      return records ? sources.get(records) ?? null : null;
    },
    filter<T extends object>(records: T[], keep: (record: T) => boolean) {
      const filtered = records.filter(keep);
      const source = sources.get(records);
      if (source) sources.set(filtered, source);
      return filtered;
    },
    map<T extends object>(records: T[], update: (record: T) => T) {
      const mapped = records.map(update);
      const source = sources.get(records);
      if (source) sources.set(mapped, source);
      return mapped;
    },
  };
}

// One weak entry per collection, not per record. JSON and payload ownership are unchanged.
export const workbenchAdminRecordSources = createWorkbenchAdminRecordSources();

export function isWorkbenchAdminRecordSourceCurrent(
  recordSource: WorkbenchAdminRecordSource | null,
  draftSource: WorkbenchAdminRecordSource | null,
) {
  try {
    return Boolean(recordSource && recordSource === draftSource && recordSource.isCurrent());
  } catch {
    return false;
  }
}
