import { requestJson } from "./core.ts";
import type {
  CentralDatabasePolicyPayload,
  CentralPublishPolicyPayload,
  CentralSessionPolicyPayload,
  CentralStoreCatalogPayload,
  CentralStoreEntryEnvelope,
  CentralStoreEntryKind,
} from "./central-store-types.ts";

type CentralStoreRequestJson = <T>(url: string, init?: RequestInit, timeoutMs?: number) => Promise<T>;

export type CentralStoreCatalogQuery = {
  kind?: CentralStoreEntryKind;
  q?: string;
  source_id?: string;
};

function appendQuery(url: string, query?: CentralStoreCatalogQuery) {
  if (!query) return url;
  const params = new URLSearchParams();

  for (const [key, value] of Object.entries(query)) {
    if (typeof value === "string" && value.trim()) {
      params.set(key, value);
    }
  }

  const search = params.toString();
  return search ? `${url}?${search}` : url;
}

export function createCentralStoreApiClient(request: CentralStoreRequestJson) {
  return {
    fetchCentralCatalog(query?: CentralStoreCatalogQuery) {
      return request<CentralStoreCatalogPayload>(appendQuery("/api/v1/central/catalog", query), {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchCentralStoreEntry(kind: CentralStoreEntryKind, entryId: string) {
      return request<CentralStoreEntryEnvelope>(
        `/api/v1/central/catalog/${kind}/${encodeURIComponent(entryId)}`,
        {
          method: "GET",
          cache: "no-store",
        },
      );
    },
    fetchCentralSessionPolicy() {
      return request<CentralSessionPolicyPayload>("/api/v1/central/session-policy", {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchCentralPublishPolicy() {
      return request<CentralPublishPolicyPayload>("/api/v1/central/publish-policy", {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchCentralDatabasePolicy() {
      return request<CentralDatabasePolicyPayload>("/api/v1/central/database-policy", {
        method: "GET",
        cache: "no-store",
      });
    },
  };
}

export type CentralStoreApiClient = ReturnType<typeof createCentralStoreApiClient>;

export const defaultCentralStoreApiClient = createCentralStoreApiClient(requestJson);

export function fetchCentralCatalog(query?: CentralStoreCatalogQuery) {
  return defaultCentralStoreApiClient.fetchCentralCatalog(query);
}

export function fetchCentralStoreEntry(kind: CentralStoreEntryKind, entryId: string) {
  return defaultCentralStoreApiClient.fetchCentralStoreEntry(kind, entryId);
}

export function fetchCentralSessionPolicy() {
  return defaultCentralStoreApiClient.fetchCentralSessionPolicy();
}

export function fetchCentralPublishPolicy() {
  return defaultCentralStoreApiClient.fetchCentralPublishPolicy();
}

export function fetchCentralDatabasePolicy() {
  return defaultCentralStoreApiClient.fetchCentralDatabasePolicy();
}
