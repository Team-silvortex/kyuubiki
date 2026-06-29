import type { AssetStoreEntry, AssetStoreEntryKind } from "@/lib/api";

export const STORE_MANIFEST_STORAGE_KEY = "kyuubiki-workbench-store-manifests";
export const STORE_MANIFEST_SCHEMA_VERSION = "kyuubiki.workspace-store-manifest/v1";

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

export function manifestEntryKey(kind: AssetStoreEntryKind, id: string) {
  return `${kind}:${id}`;
}

export function blankWorkspaceStoreManifest(projectId: string | null): WorkspaceStoreManifest {
  return {
    schema_version: STORE_MANIFEST_SCHEMA_VERSION,
    project_id: projectId ?? "unassigned",
    updated_at: new Date(0).toISOString(),
    entries: [],
  };
}

export function readWorkspaceStoreManifest(projectId: string | null): WorkspaceStoreManifest {
  if (typeof window === "undefined" || !projectId) return blankWorkspaceStoreManifest(projectId);

  try {
    const raw = window.localStorage.getItem(STORE_MANIFEST_STORAGE_KEY);
    const manifests = raw ? (JSON.parse(raw) as Record<string, WorkspaceStoreManifest>) : {};
    return normalizeWorkspaceStoreManifest(manifests[projectId], projectId);
  } catch {
    return blankWorkspaceStoreManifest(projectId);
  }
}

export function persistWorkspaceStoreManifest(manifest: WorkspaceStoreManifest) {
  if (typeof window === "undefined" || manifest.project_id === "unassigned") return;

  try {
    const raw = window.localStorage.getItem(STORE_MANIFEST_STORAGE_KEY);
    const manifests = raw ? (JSON.parse(raw) as Record<string, WorkspaceStoreManifest>) : {};
    window.localStorage.setItem(
      STORE_MANIFEST_STORAGE_KEY,
      JSON.stringify({ ...manifests, [manifest.project_id]: manifest }),
    );
  } catch {
    // Best effort only; project bundle persistence is the durable interchange path.
  }
}

export function normalizeWorkspaceStoreManifest(
  value: unknown,
  projectId: string | null,
): WorkspaceStoreManifest {
  if (!value || typeof value !== "object") return blankWorkspaceStoreManifest(projectId);
  const candidate = value as Partial<WorkspaceStoreManifest>;
  return {
    schema_version: STORE_MANIFEST_SCHEMA_VERSION,
    project_id: projectId ?? candidate.project_id ?? "unassigned",
    updated_at: typeof candidate.updated_at === "string" ? candidate.updated_at : new Date().toISOString(),
    entries: Array.isArray(candidate.entries) ? candidate.entries.filter(isManifestEntry) : [],
  };
}

export function rewriteWorkspaceStoreManifestProject(
  manifest: WorkspaceStoreManifest | null | undefined,
  projectId: string,
): WorkspaceStoreManifest {
  const normalized = normalizeWorkspaceStoreManifest(manifest, projectId);
  return {
    ...normalized,
    project_id: projectId,
    updated_at: new Date().toISOString(),
  };
}

export function addManifestEntry(
  manifest: WorkspaceStoreManifest,
  entry: AssetStoreEntry,
): WorkspaceStoreManifest {
  const key = manifestEntryKey(entry.kind, entry.id);
  const entries = manifest.entries.filter((item) => manifestEntryKey(item.kind, item.id) !== key);
  return {
    ...manifest,
    updated_at: new Date().toISOString(),
    entries: [
      ...entries,
      {
        id: entry.id,
        kind: entry.kind,
        title: entry.title,
        version: entry.version,
        source_id: entry.source_id,
        package_ref: entry.package_ref,
        target: entry.install.target,
        installed_at: new Date().toISOString(),
      },
    ],
  };
}

export function removeManifestEntry(
  manifest: WorkspaceStoreManifest,
  entry: WorkspaceStoreManifestEntry,
): WorkspaceStoreManifest {
  const key = manifestEntryKey(entry.kind, entry.id);
  return {
    ...manifest,
    updated_at: new Date().toISOString(),
    entries: manifest.entries.filter((item) => manifestEntryKey(item.kind, item.id) !== key),
  };
}

function isManifestEntry(value: unknown): value is WorkspaceStoreManifestEntry {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<WorkspaceStoreManifestEntry>;
  return (
    typeof candidate.id === "string" &&
    typeof candidate.title === "string" &&
    typeof candidate.source_id === "string" &&
    (candidate.kind === "operator" ||
      candidate.kind === "workflow_template" ||
      candidate.kind === "frontend_dsl_template")
  );
}
