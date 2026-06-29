import type {
  DatabaseExportPayload,
  ResultChunkKind,
  ResultChunkPayload,
  ResultListPayload,
  SecurityEventEnvelope,
  SecurityEventExportPayload,
  SecurityEventListPayload,
} from "./security-results-types";
import { requestJson, requestText } from "./core";

export function fetchDatabaseExport(): Promise<DatabaseExportPayload> {
  return requestJson<DatabaseExportPayload>("/api/v1/export/database", {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchSecurityEvents(filters: {
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
} = {}): Promise<SecurityEventListPayload> {
  const params = new URLSearchParams();
  Object.entries(filters).forEach(([key, value]) => {
    if (value !== undefined && value !== null && String(value).trim() !== "") {
      params.set(key, String(value));
    }
  });
  const suffix = params.size > 0 ? `?${params.toString()}` : "";

  return requestJson<SecurityEventListPayload>(`/api/v1/security-events${suffix}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function createSecurityEvent(input: {
  event_id: string;
  event_type: string;
  source: string;
  action: string;
  risk: string;
  status: string;
  note?: string | null;
  context?: Record<string, unknown>;
  occurred_at: string;
}): Promise<SecurityEventEnvelope> {
  return requestJson<SecurityEventEnvelope>("/api/v1/security-events", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function exportSecurityEvents(filters: {
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
} = {}): Promise<SecurityEventExportPayload> {
  const params = new URLSearchParams();
  Object.entries(filters).forEach(([key, value]) => {
    if (value !== undefined && value !== null && String(value).trim() !== "") {
      params.set(key, String(value));
    }
  });
  const suffix = params.size > 0 ? `?${params.toString()}` : "";

  return requestJson<SecurityEventExportPayload>(`/api/v1/export/security-events${suffix}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function exportSecurityEventsCsv(filters: {
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
} = {}): Promise<string> {
  const params = new URLSearchParams();
  Object.entries(filters).forEach(([key, value]) => {
    if (value !== undefined && value !== null && String(value).trim() !== "") {
      params.set(key, String(value));
    }
  });
  const suffix = params.size > 0 ? `?${params.toString()}` : "";

  return requestText(`/api/v1/export/security-events.csv${suffix}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchResults(): Promise<ResultListPayload> {
  return requestJson<ResultListPayload>("/api/v1/results", {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchResultChunk<TItem = Record<string, unknown>>(
  jobId: string,
  kind: ResultChunkKind,
  options: { offset?: number; limit?: number } = {},
): Promise<ResultChunkPayload<TItem>> {
  const params = new URLSearchParams();

  if (typeof options.offset === "number") params.set("offset", String(options.offset));
  if (typeof options.limit === "number") params.set("limit", String(options.limit));

  const suffix = params.size > 0 ? `?${params.toString()}` : "";

  return requestJson<ResultChunkPayload<TItem>>(`/api/v1/results/${jobId}/chunks/${kind}${suffix}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchDirectMeshResultChunk<TItem = Record<string, unknown>>(
  jobId: string,
  kind: ResultChunkKind,
  options: { offset?: number; limit?: number } = {},
): Promise<ResultChunkPayload<TItem>> {
  const params = new URLSearchParams();

  if (typeof options.offset === "number") params.set("offset", String(options.offset));
  if (typeof options.limit === "number") params.set("limit", String(options.limit));

  const suffix = params.size > 0 ? `?${params.toString()}` : "";

  return requestJson<ResultChunkPayload<TItem>>(`/api/direct-mesh/results/${jobId}/chunks/${kind}${suffix}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function updateResultRecord(
  jobId: string,
  result: Record<string, unknown>,
): Promise<{ job_id: string; result: Record<string, unknown> }> {
  return requestJson<{ job_id: string; result: Record<string, unknown> }>(`/api/v1/results/${jobId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ result }),
  });
}

export function deleteResultRecord(
  jobId: string,
): Promise<{ job_id: string; result: Record<string, unknown>; deleted: boolean }> {
  return requestJson<{ job_id: string; result: Record<string, unknown>; deleted: boolean }>(
    `/api/v1/results/${jobId}`,
    { method: "DELETE" },
  );
}
