import type {
  AssetStoreEntry,
  AssetStoreEntryEnvelope,
  AssetStoreEntryKind,
  AssetStorePayload,
} from "@/lib/api";

export type WorkbenchStoreCatalogQuery = {
  kind?: AssetStoreEntryKind;
  q?: string;
  source_id?: string;
};

export type WorkbenchStoreBackendTransport = {
  fetchCatalog: (query?: WorkbenchStoreCatalogQuery) => Promise<AssetStorePayload>;
  fetchEntry: (
    kind: AssetStoreEntryKind,
    entryId: string,
  ) => Promise<AssetStoreEntryEnvelope>;
};

export type WorkbenchStoreBackendService = {
  fetchCatalog: (query?: WorkbenchStoreCatalogQuery) => Promise<AssetStorePayload>;
  fetchEntry: (kind: AssetStoreEntryKind, entryId: string) => Promise<AssetStoreEntry>;
};

export function createWorkbenchStoreBackendService(
  transport: WorkbenchStoreBackendTransport,
): WorkbenchStoreBackendService {
  return {
    fetchCatalog: transport.fetchCatalog,
    async fetchEntry(kind, entryId) {
      const payload = await transport.fetchEntry(kind, entryId);
      return payload.entry;
    },
  };
}
