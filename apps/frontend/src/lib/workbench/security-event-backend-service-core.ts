"use client";

import type {
  SecurityEventEnvelope,
  SecurityEventListPayload,
} from "@/lib/api/security-results-types";

export type WorkbenchSecurityEventFilters = {
  action?: string;
  limit?: number;
  model_version_id?: string;
  occurred_after?: string;
  occurred_before?: string;
  project_id?: string;
  risk?: string;
  source?: string;
  status?: string;
  study_kind?: string;
};

export type WorkbenchSecurityEventInput = {
  action: string;
  context?: Record<string, unknown>;
  event_id: string;
  event_type: string;
  note?: string | null;
  occurred_at: string;
  risk: string;
  source: string;
  status: string;
};

export type WorkbenchSecurityEventBackendTransport = {
  createEvent(input: WorkbenchSecurityEventInput): Promise<SecurityEventEnvelope>;
  fetchEvents(filters?: WorkbenchSecurityEventFilters): Promise<SecurityEventListPayload>;
};

export type WorkbenchSecurityEventBackendService = {
  createEvent(input: WorkbenchSecurityEventInput): Promise<SecurityEventEnvelope>;
  fetchEvents(filters?: WorkbenchSecurityEventFilters): Promise<SecurityEventListPayload>;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function isTimestamp(value: unknown) {
  return typeof value === "string" && value.trim().length > 0 && Number.isFinite(Date.parse(value));
}

function validateHistory(payload: unknown): asserts payload is SecurityEventListPayload {
  const identities = new Set<string>();
  if (!isRecord(payload) || !Array.isArray(payload.events) || payload.events.some((entry: unknown) => {
    if (!isRecord(entry)) return true;
    if (["event_id", "event_type", "source", "action", "risk", "status"].some((field) =>
      typeof entry[field] !== "string" || !entry[field].trim())) return true;
    if (!isTimestamp(entry.occurred_at) || !isRecord(entry.context)) return true;
    if (entry.note != null && typeof entry.note !== "string") return true;
    if (["inserted_at", "updated_at"].some((field) =>
      entry[field] !== undefined && !isTimestamp(entry[field]))) return true;
    const id = entry.event_id as string;
    if (identities.has(id)) return true;
    identities.add(id);
    return false;
  })) throw new Error("SECURITY_EVENT_HISTORY_INVALID: Security event history failed validation.");
}

export function createSecurityEventBackendService(
  transport: WorkbenchSecurityEventBackendTransport,
): WorkbenchSecurityEventBackendService {
  return {
    createEvent: transport.createEvent,
    async fetchEvents(filters) {
      const payload = await transport.fetchEvents(filters);
      validateHistory(payload);
      return payload;
    },
  };
}
