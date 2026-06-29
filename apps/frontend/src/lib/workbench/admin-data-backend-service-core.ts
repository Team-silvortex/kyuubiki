"use client";

import type { JobEnvelope, JobState } from "@/lib/api/fem-shared";
import type {
  ResultListPayload,
  ResultRecord,
} from "@/lib/api/security-results-types";

export type WorkbenchJobRecordPatch = Partial<{
  message: string;
  model_version_id: string;
  project_id: string;
  simulation_case_id: string;
}>;

export type WorkbenchResultRecordEnvelope = {
  job_id: string;
  result: Record<string, unknown>;
};

export type WorkbenchDeletedJobEnvelope = {
  deleted: boolean;
  job: JobState;
};

export type WorkbenchDeletedResultEnvelope = WorkbenchResultRecordEnvelope & {
  deleted: boolean;
};

export type WorkbenchAdminDataBackendTransport = {
  deleteJob(jobId: string): Promise<WorkbenchDeletedJobEnvelope>;
  deleteResult(jobId: string): Promise<WorkbenchDeletedResultEnvelope>;
  fetchJob<TResult = unknown>(jobId: string): Promise<JobEnvelope<TResult>>;
  fetchResults(): Promise<ResultListPayload>;
  updateJob(jobId: string, input: WorkbenchJobRecordPatch): Promise<JobEnvelope>;
  updateResult(jobId: string, result: Record<string, unknown>): Promise<WorkbenchResultRecordEnvelope>;
};

export type WorkbenchAdminDataBackendService = WorkbenchAdminDataBackendTransport & {
  listResults(): Promise<ResultRecord[]>;
};

export function createAdminDataBackendService(
  transport: WorkbenchAdminDataBackendTransport,
): WorkbenchAdminDataBackendService {
  return {
    deleteJob: transport.deleteJob,
    deleteResult: transport.deleteResult,
    fetchJob: transport.fetchJob,
    fetchResults: transport.fetchResults,
    listResults: async () => {
      const payload = await transport.fetchResults();
      return payload.results;
    },
    updateJob: transport.updateJob,
    updateResult: transport.updateResult,
  };
}
