"use client";

import type { JobEnvelope, JobHistoryPayload } from "@/lib/api/fem-shared";

export type WorkbenchJobHistoryBackendTransport = {
  cancelJob(jobId: string): Promise<JobEnvelope>;
  fetchJobHistory(): Promise<JobHistoryPayload>;
};

export type WorkbenchJobHistoryBackendService = {
  cancelJob(jobId: string): Promise<JobEnvelope>;
  fetchHistory(): Promise<JobHistoryPayload>;
};

export function createJobHistoryBackendService(
  transport: WorkbenchJobHistoryBackendTransport,
): WorkbenchJobHistoryBackendService {
  return {
    cancelJob: transport.cancelJob,
    fetchHistory: transport.fetchJobHistory,
  };
}
