import type {
  DatabaseExportPayload,
  ResultChunkKind,
  ResultChunkPayload,
  ResultListPayload,
  SecurityEventEnvelope,
  SecurityEventExportPayload,
  SecurityEventListPayload,
} from "./security-results-types.ts";
import { requestJson, requestText } from "./core.ts";

type SecurityResultsRequestJson = <T>(url: string, init?: RequestInit, timeoutMs?: number) => Promise<T>;
type SecurityResultsRequestText = (url: string, init?: RequestInit, timeoutMs?: number) => Promise<string>;

export type SecurityEventFilters = {
  source?: string;
  risk?: string;
  status?: string;
  action?: string;
  study_kind?: string;
  project_id?: string;
  model_version_id?: string;
  occurred_after?: string;
  occurred_before?: string;
  limit?: number;
};

export type SecurityEventInput = {
  event_id: string;
  event_type: string;
  source: string;
  action: string;
  risk: string;
  status: string;
  note?: string | null;
  context?: Record<string, unknown>;
  occurred_at: string;
};

function appendFilters(url: string, filters: SecurityEventFilters = {}) {
  const params = new URLSearchParams();
  Object.entries(filters).forEach(([key, value]) => {
    if (value !== undefined && value !== null && String(value).trim() !== "") {
      params.set(key, String(value));
    }
  });
  const suffix = params.size > 0 ? `?${params.toString()}` : "";
  return `${url}${suffix}`;
}

export function fetchDatabaseExport(): Promise<DatabaseExportPayload> {
  return defaultSecurityResultsApiClient.fetchDatabaseExport();
}

export function fetchSecurityEvents(filters: SecurityEventFilters = {}): Promise<SecurityEventListPayload> {
  return defaultSecurityResultsApiClient.fetchSecurityEvents(filters);
}

export function createSecurityEvent(input: SecurityEventInput): Promise<SecurityEventEnvelope> {
  return defaultSecurityResultsApiClient.createSecurityEvent(input);
}

export function exportSecurityEvents(filters: SecurityEventFilters = {}): Promise<SecurityEventExportPayload> {
  return defaultSecurityResultsApiClient.exportSecurityEvents(filters);
}

export function exportSecurityEventsCsv(filters: SecurityEventFilters = {}): Promise<string> {
  return defaultSecurityResultsApiClient.exportSecurityEventsCsv(filters);
}

export function fetchResults(): Promise<ResultListPayload> {
  return defaultSecurityResultsApiClient.fetchResults();
}

export function fetchResultChunk<TItem = Record<string, unknown>>(
  jobId: string,
  kind: ResultChunkKind,
  options: { offset?: number; limit?: number } = {},
): Promise<ResultChunkPayload<TItem>> {
  return defaultSecurityResultsApiClient.fetchResultChunk(jobId, kind, options);
}

export function fetchDirectMeshResultChunk<TItem = Record<string, unknown>>(
  jobId: string,
  kind: ResultChunkKind,
  options: { offset?: number; limit?: number } = {},
): Promise<ResultChunkPayload<TItem>> {
  return defaultSecurityResultsApiClient.fetchDirectMeshResultChunk(jobId, kind, options);
}

export function updateResultRecord(
  jobId: string,
  result: Record<string, unknown>,
): Promise<{ job_id: string; result: Record<string, unknown> }> {
  return defaultSecurityResultsApiClient.updateResultRecord(jobId, result);
}

export function deleteResultRecord(
  jobId: string,
): Promise<{ job_id: string; result: Record<string, unknown>; deleted: boolean }> {
  return defaultSecurityResultsApiClient.deleteResultRecord(jobId);
}

function appendChunkOptions(url: string, options: { offset?: number; limit?: number } = {}) {
  const params = new URLSearchParams();
  if (typeof options.offset === "number") params.set("offset", String(options.offset));
  if (typeof options.limit === "number") params.set("limit", String(options.limit));
  const suffix = params.size > 0 ? `?${params.toString()}` : "";
  return `${url}${suffix}`;
}

export function createSecurityResultsApiClient(input: {
  requestJson: SecurityResultsRequestJson;
  requestText: SecurityResultsRequestText;
}) {
  return {
    fetchDatabaseExport() {
      return input.requestJson<DatabaseExportPayload>("/api/v1/export/database", {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchSecurityEvents(filters: SecurityEventFilters = {}) {
      return input.requestJson<SecurityEventListPayload>(appendFilters("/api/v1/security-events", filters), {
        method: "GET",
        cache: "no-store",
      });
    },
    createSecurityEvent(eventInput: SecurityEventInput) {
      return input.requestJson<SecurityEventEnvelope>("/api/v1/security-events", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(eventInput),
      });
    },
    exportSecurityEvents(filters: SecurityEventFilters = {}) {
      return input.requestJson<SecurityEventExportPayload>(
        appendFilters("/api/v1/export/security-events", filters),
        { method: "GET", cache: "no-store" },
      );
    },
    exportSecurityEventsCsv(filters: SecurityEventFilters = {}) {
      return input.requestText(appendFilters("/api/v1/export/security-events.csv", filters), {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchResults() {
      return input.requestJson<ResultListPayload>("/api/v1/results", {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchResultChunk<TItem = Record<string, unknown>>(
      jobId: string,
      kind: ResultChunkKind,
      options: { offset?: number; limit?: number } = {},
    ) {
      return input.requestJson<ResultChunkPayload<TItem>>(
        appendChunkOptions(`/api/v1/results/${jobId}/chunks/${kind}`, options),
        { method: "GET", cache: "no-store" },
      );
    },
    fetchDirectMeshResultChunk<TItem = Record<string, unknown>>(
      jobId: string,
      kind: ResultChunkKind,
      options: { offset?: number; limit?: number } = {},
    ) {
      return input.requestJson<ResultChunkPayload<TItem>>(
        appendChunkOptions(`/api/direct-mesh/results/${jobId}/chunks/${kind}`, options),
        { method: "GET", cache: "no-store" },
      );
    },
    updateResultRecord(jobId: string, result: Record<string, unknown>) {
      return input.requestJson<{ job_id: string; result: Record<string, unknown> }>(
        `/api/v1/results/${jobId}`,
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ result }),
        },
      );
    },
    deleteResultRecord(jobId: string) {
      return input.requestJson<{ job_id: string; result: Record<string, unknown>; deleted: boolean }>(
        `/api/v1/results/${jobId}`,
        { method: "DELETE" },
      );
    },
  };
}

export type SecurityResultsApiClient = ReturnType<typeof createSecurityResultsApiClient>;

export const defaultSecurityResultsApiClient = createSecurityResultsApiClient({
  requestJson,
  requestText,
});
