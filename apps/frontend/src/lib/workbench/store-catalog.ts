import type { AssetStoreEntry, AssetStoreEntryKind } from "@/lib/api";
import { manifestEntryFromAsset, type WorkspaceStoreManifestEntry } from "./store-manifest";

export type WorkbenchStoreEntrySelection = {
  sourceId?: string;
  version?: string;
  packageRef?: string;
  target?: string;
};

type StoreEntrySelectionErrorCode =
  | "store_entry_invalid_selection"
  | "store_catalog_invalid"
  | "store_entry_invalid"
  | "store_entry_not_found"
  | "store_entry_ambiguous";

export class StoreEntrySelectionError extends Error {
  readonly code: StoreEntrySelectionErrorCode;

  constructor(code: StoreEntrySelectionErrorCode, message: string) {
    super(`${code}: ${message}`);
    this.name = "StoreEntrySelectionError";
    this.code = code;
  }
}

const SELECTION_LIMITS = { sourceId: 256, version: 512, packageRef: 2048, target: 2048 } as const;

export function normalizeStoreEntrySelection(value: unknown = {}): WorkbenchStoreEntrySelection {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new StoreEntrySelectionError("store_entry_invalid_selection", "Expected Store identity selectors.");
  }
  const selection: WorkbenchStoreEntrySelection = {};
  for (const [key, field] of Object.entries(value)) {
    if (!Object.hasOwn(SELECTION_LIMITS, key)) {
      throw new StoreEntrySelectionError("store_entry_invalid_selection", `Unknown selector: ${key}`);
    }
    if (field === undefined) continue;
    const name = key as keyof WorkbenchStoreEntrySelection;
    if (typeof field !== "string" || !field.trim() || field.trim().length > SELECTION_LIMITS[name]) {
      throw new StoreEntrySelectionError("store_entry_invalid_selection", `Invalid selector: ${key}`);
    }
    selection[name] = field.trim();
  }
  return selection;
}

export function resolveStoreCatalogEntry(
  entries: AssetStoreEntry[], kind: AssetStoreEntryKind, entryId: string,
  selection: WorkbenchStoreEntrySelection,
): AssetStoreEntry {
  const candidates = entries.filter((entry) => entry?.kind === kind && entry?.id === entryId);
  const matches = candidates.filter((entry) => {
    const normalized = manifestEntryFromAsset(entry);
    const optionalFields = [entry.version, entry.package_ref, entry.install?.target];
    const normalizedFields = [normalized?.version, normalized?.package_ref, normalized?.target];
    if (!normalized || optionalFields.some((field, index) => field != null && normalizedFields[index] == null)) {
      throw new StoreEntrySelectionError("store_entry_invalid", "The matching catalog entry is not a valid project reference.");
    }
    return (selection.sourceId === undefined || normalized.source_id === selection.sourceId) &&
      (selection.version === undefined || normalized.version === selection.version) &&
      (selection.packageRef === undefined || normalized.package_ref === selection.packageRef) &&
      (selection.target === undefined || normalized.target === selection.target);
  });
  if (matches.length === 0) {
    throw new StoreEntrySelectionError("store_entry_not_found", "No entry matches the requested Store identity.");
  }
  if (matches.length !== 1) {
    throw new StoreEntrySelectionError("store_entry_ambiguous", "Specify sourceId, version, packageRef and target to select one entry.");
  }
  return matches[0];
}

export function storeEntryIdentity(entry: AssetStoreEntry) {
  return JSON.stringify([entry.source_id, entry.kind, entry.id, entry.version ?? null]);
}

export function storeProjectEntryState(entry: AssetStoreEntry, entries: WorkspaceStoreManifestEntry[]) {
  const candidate = manifestEntryFromAsset(entry);
  if (!candidate) return "invalid";
  const current = entries.find((item) => item.kind === candidate.kind && item.id === candidate.id);
  if (!current) return "available";
  return current.source_id === candidate.source_id &&
    (current.version ?? null) === candidate.version &&
    (current.package_ref ?? null) === candidate.package_ref &&
    (current.target ?? null) === candidate.target
    ? "staged" : "conflict";
}

export function selectStoreEntries(entries: AssetStoreEntry[], source: string, domain: string) {
  return entries.filter((entry) => (!source || entry.source_id === source) && (!domain || entry.domain === domain));
}
