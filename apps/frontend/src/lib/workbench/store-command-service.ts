import type { AssetStoreEntry, AssetStoreEntryKind } from "@/lib/api";
import {
  addManifestEntry,
  manifestEntryKey,
  persistWorkspaceStoreManifest,
  readWorkspaceStoreManifestResult,
  removeManifestEntry,
  type WorkspaceStoreManifest,
} from "@/lib/workbench/store-manifest";

export type WorkspaceStoreCommandErrorCode =
  | "project_required"
  | "manifest_unreadable"
  | "manifest_write_failed"
  | "invalid_asset"
  | "entry_missing";

export class WorkspaceStoreCommandError extends Error {
  readonly code: WorkspaceStoreCommandErrorCode;

  constructor(code: WorkspaceStoreCommandErrorCode, message: string) {
    super(message);
    this.name = "WorkspaceStoreCommandError";
    this.code = code;
  }
}

export function stageWorkspaceStoreEntry(
  projectId: string | null,
  entry: AssetStoreEntry,
): WorkspaceStoreManifest {
  const current = readWritableManifest(projectId);
  const nextManifest = addManifestEntry(current, entry);
  if (!nextManifest) {
    throw new WorkspaceStoreCommandError("invalid_asset", "Store asset entry is invalid.");
  }
  persistManifestOrThrow(nextManifest);
  return nextManifest;
}

export function removeWorkspaceStoreEntry(
  projectId: string | null,
  kind: AssetStoreEntryKind,
  entryId: string,
): WorkspaceStoreManifest {
  const current = readWritableManifest(projectId);
  const key = manifestEntryKey(kind, entryId);
  const entry = current.entries.find((candidate) => manifestEntryKey(candidate.kind, candidate.id) === key);
  if (!entry) {
    throw new WorkspaceStoreCommandError("entry_missing", `Store asset is not staged: ${key}`);
  }
  const nextManifest = removeManifestEntry(current, entry);
  persistManifestOrThrow(nextManifest);
  return nextManifest;
}

export function buildWorkspaceStoreManifestExport(projectId: string | null): {
  filename: string;
  contents: string;
  manifest: WorkspaceStoreManifest;
} {
  const manifest = readWritableManifest(projectId);
  const safeProjectId = manifest.project_id.replace(/[^A-Za-z0-9._-]+/gu, "-").replace(/^-+|-+$/gu, "");
  return {
    filename: `${safeProjectId || "kyuubiki-workspace"}.store-manifest.json`,
    contents: `${JSON.stringify(manifest, null, 2)}\n`,
    manifest,
  };
}

function readWritableManifest(projectId: string | null) {
  if (!projectId?.trim()) {
    throw new WorkspaceStoreCommandError("project_required", "A selected project is required.");
  }
  const result = readWorkspaceStoreManifestResult(projectId);
  if (!result.readable) {
    throw new WorkspaceStoreCommandError("manifest_unreadable", "Store manifest storage is unreadable.");
  }
  return result.manifest;
}

function persistManifestOrThrow(manifest: WorkspaceStoreManifest) {
  if (!persistWorkspaceStoreManifest(manifest)) {
    throw new WorkspaceStoreCommandError("manifest_write_failed", "Store manifest could not be persisted.");
  }
}
