import type {
  AssetStoreEntry,
  AssetStoreEntryKind,
  AssetStorePayload,
} from "@/lib/api";
import {
  normalizeStoreEntrySelection,
  resolveStoreCatalogEntry,
  StoreEntrySelectionError,
  type WorkbenchStoreEntrySelection,
} from "./store-catalog";

export type { WorkbenchStoreEntrySelection } from "./store-catalog";

export type WorkbenchStoreCatalogQuery = {
  kind?: AssetStoreEntryKind;
  q?: string;
  source_id?: string;
};

export type WorkbenchStoreBackendTransport = {
  fetchCatalog: (query?: WorkbenchStoreCatalogQuery) => Promise<AssetStorePayload>;
};

export type WorkbenchStoreBackendService = {
  fetchCatalog: (query?: WorkbenchStoreCatalogQuery) => Promise<AssetStorePayload>;
  fetchEntry: (
    kind: AssetStoreEntryKind, entryId: string, selection?: WorkbenchStoreEntrySelection,
  ) => Promise<AssetStoreEntry>;
};

export function createWorkbenchStoreBackendService(
  transport: WorkbenchStoreBackendTransport,
): WorkbenchStoreBackendService {
  return {
    fetchCatalog: transport.fetchCatalog,
    async fetchEntry(kind, entryId, requestedSelection) {
      const selection = normalizeStoreEntrySelection(requestedSelection);
      if (!["operator", "workflow_template", "frontend_dsl_template"].includes(kind) ||
          typeof entryId !== "string" || !entryId.trim() || entryId.trim().length > 256) {
        throw new StoreEntrySelectionError("store_entry_invalid_selection", "Invalid Store kind or entryId.");
      }
      const id = entryId.trim();
      // The legacy detail endpoint chooses the first kind/id match across sources.
      const payload = await transport.fetchCatalog({ kind, q: id, source_id: selection.sourceId });
      if (!Array.isArray(payload?.entries) || payload.summary?.entry_count !== payload.entries.length) {
        throw new StoreEntrySelectionError("store_catalog_invalid", "Exact selection requires a complete catalog response.");
      }
      return resolveStoreCatalogEntry(payload.entries, kind, id, selection);
    },
  };
}
