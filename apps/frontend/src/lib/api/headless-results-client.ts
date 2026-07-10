"use client";

import { requestJson } from "./core.ts";

type HeadlessResultsRequestJson = <T>(url: string, init?: RequestInit, timeoutMs?: number) => Promise<T>;
type HeadlessResultRecordPayload = { job_id: string; result: Record<string, unknown> };

export function fetchResultRecord(jobId: string): Promise<{ job_id: string; result: Record<string, unknown> }> {
  return defaultHeadlessResultsApiClient.fetchResultRecord(jobId);
}

export function fetchDirectMeshResultRecord(jobId: string): Promise<{ job_id: string; result: Record<string, unknown> }> {
  return defaultHeadlessResultsApiClient.fetchDirectMeshResultRecord(jobId);
}

export function createHeadlessResultsApiClient(request: HeadlessResultsRequestJson) {
  return {
    fetchResultRecord(jobId: string) {
      return request<HeadlessResultRecordPayload>(`/api/v1/results/${jobId}`, {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchDirectMeshResultRecord(jobId: string) {
      return request<HeadlessResultRecordPayload>(`/api/direct-mesh/results/${jobId}`, {
        method: "GET",
        cache: "no-store",
      });
    },
  };
}

export type HeadlessResultsApiClient = ReturnType<typeof createHeadlessResultsApiClient>;

export const defaultHeadlessResultsApiClient = createHeadlessResultsApiClient(requestJson);
