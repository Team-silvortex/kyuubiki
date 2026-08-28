import type { AssetStoreEntry, AssetStoreEntryKind } from "@/lib/api";

export const STORE_MANIFEST_STORAGE_KEY = "kyuubiki-workbench-store-manifests";
export const STORE_MANIFEST_SCHEMA_VERSION = "kyuubiki.workspace-store-manifest/v1";
export const STORE_MANIFEST_CHANGED_EVENT = "kyuubiki:workspace-store-manifest-changed";
export const STORE_MANIFEST_ENTRY_LIMIT = 128;
export const STORE_MANIFEST_PROJECT_LIMIT = 64;

const UNASSIGNED_PROJECT_ID = "unassigned";
const EPOCH_TIMESTAMP = new Date(0).toISOString();
const ID_LENGTH_LIMIT = 256;
const LABEL_LENGTH_LIMIT = 512;
const REFERENCE_LENGTH_LIMIT = 2_048;

export type WorkspaceStoreManifestEntry = {
  id: string;
  kind: AssetStoreEntryKind;
  title: string;
  version?: string | null;
  source_id: string;
  package_ref?: string | null;
  target?: string | null;
  installed_at: string;
};

export type WorkspaceStoreManifest = {
  schema_version: string;
  project_id: string;
  updated_at: string;
  entries: WorkspaceStoreManifestEntry[];
};

export type WorkspaceStoreManifestReadResult = {
  manifest: WorkspaceStoreManifest;
  readable: boolean;
};

type StoredManifestCollectionReadResult = {
  manifests: Record<string, WorkspaceStoreManifest>;
  readable: boolean;
};

export function manifestEntryKey(kind: AssetStoreEntryKind, id: string) {
  return `${kind}:${id}`;
}

export function blankWorkspaceStoreManifest(projectId: string | null): WorkspaceStoreManifest {
  return {
    schema_version: STORE_MANIFEST_SCHEMA_VERSION,
    project_id: normalizeProjectId(projectId) ?? UNASSIGNED_PROJECT_ID,
    updated_at: EPOCH_TIMESTAMP,
    entries: [],
  };
}

export function readWorkspaceStoreManifestResult(
  projectId: string | null,
): WorkspaceStoreManifestReadResult {
  if (!projectId) {
    return { manifest: blankWorkspaceStoreManifest(projectId), readable: true };
  }
  const normalizedProjectId = normalizeProjectId(projectId);
  if (!normalizedProjectId) {
    return { manifest: blankWorkspaceStoreManifest(projectId), readable: false };
  }

  const collection = readStoredManifestCollection();
  return {
    manifest: collection.readable
      ? collection.manifests[normalizedProjectId] ?? blankWorkspaceStoreManifest(normalizedProjectId)
      : blankWorkspaceStoreManifest(projectId),
    readable: collection.readable,
  };
}

export function readWorkspaceStoreManifest(projectId: string | null): WorkspaceStoreManifest {
  return readWorkspaceStoreManifestResult(projectId).manifest;
}

export function manifestForSelectedProject(
  manifest: WorkspaceStoreManifest,
  projectId: string | null,
): WorkspaceStoreManifest {
  const expectedProjectId = normalizeProjectId(projectId) ?? UNASSIGNED_PROJECT_ID;
  return manifest.project_id === expectedProjectId
    ? manifest
    : blankWorkspaceStoreManifest(projectId);
}

export function persistWorkspaceStoreManifest(manifest: WorkspaceStoreManifest): boolean {
  const projectId = normalizeProjectId(manifest.project_id);
  if (typeof window === "undefined" || !projectId || projectId === UNASSIGNED_PROJECT_ID) {
    return false;
  }

  const collection = readStoredManifestCollection();
  if (!collection.readable) return false;

  const normalized = normalizeWorkspaceStoreManifest(manifest, projectId);
  const retained = Object.values({ ...collection.manifests, [projectId]: normalized })
    .sort((left, right) => Date.parse(right.updated_at) - Date.parse(left.updated_at))
    .slice(0, STORE_MANIFEST_PROJECT_LIMIT);
  const nextCollection = Object.fromEntries(retained.map((entry) => [entry.project_id, entry]));

  try {
    window.localStorage.setItem(STORE_MANIFEST_STORAGE_KEY, JSON.stringify(nextCollection));
    if (typeof window.dispatchEvent === "function" && typeof Event === "function") {
      window.dispatchEvent(new Event(STORE_MANIFEST_CHANGED_EVENT));
    }
    return true;
  } catch {
    return false;
  }
}

export function normalizeWorkspaceStoreManifest(
  value: unknown,
  projectId: string | null,
): WorkspaceStoreManifest {
  if (!isRecord(value)) return blankWorkspaceStoreManifest(projectId);

  const resolvedProjectId =
    normalizeProjectId(projectId) ??
    normalizeProjectId(value.project_id) ??
    UNASSIGNED_PROJECT_ID;
  const updatedAt = normalizeTimestamp(value.updated_at, EPOCH_TIMESTAMP);
  const sourceEntries = Array.isArray(value.entries) ? value.entries : [];
  const entries: WorkspaceStoreManifestEntry[] = [];
  const seen = new Set<string>();

  for (let index = sourceEntries.length - 1; index >= 0; index -= 1) {
    const entry = normalizeManifestEntry(sourceEntries[index], updatedAt);
    if (!entry) continue;
    const key = manifestEntryKey(entry.kind, entry.id);
    if (seen.has(key)) continue;
    seen.add(key);
    entries.push(entry);
    if (entries.length >= STORE_MANIFEST_ENTRY_LIMIT) break;
  }
  entries.reverse();

  return {
    schema_version: STORE_MANIFEST_SCHEMA_VERSION,
    project_id: resolvedProjectId,
    updated_at: updatedAt,
    entries,
  };
}

export function rewriteWorkspaceStoreManifestProject(
  manifest: WorkspaceStoreManifest | null | undefined,
  projectId: string,
): WorkspaceStoreManifest {
  const normalized = normalizeWorkspaceStoreManifest(manifest, projectId);
  return {
    ...normalized,
    project_id: normalizeProjectId(projectId) ?? UNASSIGNED_PROJECT_ID,
    updated_at: new Date().toISOString(),
  };
}

export function addManifestEntry(
  manifest: WorkspaceStoreManifest,
  entry: AssetStoreEntry,
): WorkspaceStoreManifest | null {
  const now = new Date().toISOString();
  const manifestEntry = normalizeManifestEntry(
    {
      id: entry.id,
      kind: entry.kind,
      title: entry.title,
      version: entry.version,
      source_id: entry.source_id,
      package_ref: entry.package_ref,
      target: entry.install?.target,
      installed_at: now,
    },
    now,
  );
  if (!manifestEntry) return null;

  const key = manifestEntryKey(manifestEntry.kind, manifestEntry.id);
  return normalizeWorkspaceStoreManifest(
    {
      ...manifest,
      updated_at: now,
      entries: [
        ...manifest.entries.filter((item) => manifestEntryKey(item.kind, item.id) !== key),
        manifestEntry,
      ],
    },
    manifest.project_id,
  );
}

export function removeManifestEntry(
  manifest: WorkspaceStoreManifest,
  entry: WorkspaceStoreManifestEntry,
): WorkspaceStoreManifest {
  const key = manifestEntryKey(entry.kind, entry.id);
  return normalizeWorkspaceStoreManifest(
    {
      ...manifest,
      updated_at: new Date().toISOString(),
      entries: manifest.entries.filter((item) => manifestEntryKey(item.kind, item.id) !== key),
    },
    manifest.project_id,
  );
}

function readStoredManifestCollection(): StoredManifestCollectionReadResult {
  if (typeof window === "undefined") return { manifests: {}, readable: false };

  try {
    const raw = window.localStorage.getItem(STORE_MANIFEST_STORAGE_KEY);
    if (raw === null) return { manifests: {}, readable: true };
    const parsed = JSON.parse(raw) as unknown;
    if (!isRecord(parsed)) return { manifests: {}, readable: false };

    const manifests: Record<string, WorkspaceStoreManifest> = Object.create(null);
    for (const [rawProjectId, value] of Object.entries(parsed)) {
      const projectId = normalizeProjectId(rawProjectId);
      if (
        !projectId ||
        projectId !== rawProjectId ||
        projectId === UNASSIGNED_PROJECT_ID ||
        !isStoredManifestShape(value)
      ) {
        return { manifests: {}, readable: false };
      }
      manifests[projectId] = normalizeWorkspaceStoreManifest(value, projectId);
    }
    return { manifests, readable: true };
  } catch {
    return { manifests: {}, readable: false };
  }
}

function isStoredManifestShape(value: unknown) {
  if (!isRecord(value)) return false;
  if (
    value.schema_version !== undefined &&
    value.schema_version !== STORE_MANIFEST_SCHEMA_VERSION
  ) {
    return false;
  }
  return value.entries === undefined || Array.isArray(value.entries);
}

function normalizeManifestEntry(
  value: unknown,
  fallbackInstalledAt: string,
): WorkspaceStoreManifestEntry | null {
  if (!isRecord(value) || !isAssetKind(value.kind)) return null;
  const id = normalizeText(value.id, ID_LENGTH_LIMIT);
  const title = normalizeText(value.title, LABEL_LENGTH_LIMIT);
  const sourceId = normalizeText(value.source_id, ID_LENGTH_LIMIT);
  if (!id || !title || !sourceId) return null;

  return {
    id,
    kind: value.kind,
    title,
    version: normalizeOptionalText(value.version, LABEL_LENGTH_LIMIT),
    source_id: sourceId,
    package_ref: normalizeOptionalText(value.package_ref, REFERENCE_LENGTH_LIMIT),
    target: normalizeOptionalText(value.target, REFERENCE_LENGTH_LIMIT),
    installed_at: normalizeTimestamp(value.installed_at, fallbackInstalledAt),
  };
}

function normalizeText(value: unknown, maxLength: number) {
  if (typeof value !== "string") return null;
  const normalized = value.trim();
  if (!normalized || normalized.length > maxLength) return null;
  return normalized;
}

function normalizeProjectId(value: unknown) {
  const projectId = normalizeText(value, ID_LENGTH_LIMIT);
  if (
    projectId === "__proto__" ||
    projectId === "prototype" ||
    projectId === "constructor"
  ) {
    return null;
  }
  return projectId;
}

function normalizeOptionalText(value: unknown, maxLength: number) {
  if (value === null || value === undefined) return null;
  return normalizeText(value, maxLength);
}

function normalizeTimestamp(value: unknown, fallback: string) {
  if (typeof value !== "string") return fallback;
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? new Date(timestamp).toISOString() : fallback;
}

function isAssetKind(value: unknown): value is AssetStoreEntryKind {
  return value === "operator" || value === "workflow_template" || value === "frontend_dsl_template";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
