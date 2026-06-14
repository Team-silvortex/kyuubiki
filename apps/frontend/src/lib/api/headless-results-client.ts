"use client";

import { requestJson } from "./core";

export function fetchResultRecord(jobId: string): Promise<{ job_id: string; result: Record<string, unknown> }> {
  return requestJson<{ job_id: string; result: Record<string, unknown> }>(`/api/v1/results/${jobId}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchDirectMeshResultRecord(jobId: string): Promise<{ job_id: string; result: Record<string, unknown> }> {
  return requestJson<{ job_id: string; result: Record<string, unknown> }>(`/api/direct-mesh/results/${jobId}`, {
    method: "GET",
    cache: "no-store",
  });
}
