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

export function createSecurityEventBackendService(
  transport: WorkbenchSecurityEventBackendTransport,
): WorkbenchSecurityEventBackendService {
  return {
    createEvent: transport.createEvent,
    fetchEvents: transport.fetchEvents,
  };
}
