"use client";

import type { JobEnvelope, JobState } from "@/lib/api/fem-shared";
import { isWorkflowJobStatusContractValid } from "@/lib/api/job-status";
import type {
  ResultListPayload,
  ResultRecord,
} from "@/lib/api/security-results-types";

export type WorkbenchJobRecordPatch = Partial<{
  message: string;
  model_version_id: string | null;
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function validateJobMutation<T extends { job: JobState; deleted?: unknown }>(payload: T, jobId: string, deleted = false): T {
  if (!isRecord(payload) || !isRecord(payload.job) || payload.job.job_id !== jobId ||
    !isWorkflowJobStatusContractValid({ status: payload.job.status, progress: payload.job.progress,
      statusDetail: payload.job.status_detail }) || (deleted && payload.deleted !== true)) {
    throw new Error("JOB_RECORD_MUTATION_UNCONFIRMED: refresh records before retrying this write.");
  }
  return payload;
}

function validateResultMutation<T extends WorkbenchResultRecordEnvelope & { deleted?: unknown }>(
  payload: T, jobId: string, deleted = false,
): T {
  if (!isRecord(payload) || payload.job_id !== jobId || !isRecord(payload.result) ||
      (deleted && payload.deleted !== true)) {
    throw new Error("RESULT_RECORD_MUTATION_UNCONFIRMED: refresh records before retrying this write.");
  }
  return payload;
}

function validateResultHistory(payload: ResultListPayload): ResultListPayload {
  const ids = new Set<string>();
  if (!isRecord(payload) || !Array.isArray(payload.results) || payload.results.some((entry: unknown) => {
    if (!isRecord(entry) || typeof entry.job_id !== "string" || !entry.job_id.trim() ||
        !isRecord(entry.result) || ids.has(entry.job_id)) return true;
    ids.add(entry.job_id);
    return [entry.inserted_at, entry.updated_at].some((at) => at !== undefined &&
      (typeof at !== "string" || !at.trim() || !Number.isFinite(Date.parse(at))));
  })) {
    throw new Error("RESULT_HISTORY_INVALID: invalid result history response.");
  }
  return payload;
}

export function createAdminDataBackendService(
  transport: WorkbenchAdminDataBackendTransport,
): WorkbenchAdminDataBackendService {
  const fetchResults = async () => validateResultHistory(await transport.fetchResults());
  return {
    deleteJob: async (jobId) => validateJobMutation(await transport.deleteJob(jobId), jobId, true),
    deleteResult: async (jobId) => validateResultMutation(await transport.deleteResult(jobId), jobId, true),
    fetchJob: transport.fetchJob,
    fetchResults,
    listResults: async () => {
      const payload = await fetchResults();
      return payload.results;
    },
    updateJob: async (jobId, input) => validateJobMutation(await transport.updateJob(jobId, input), jobId),
    updateResult: async (jobId, result) => validateResultMutation(await transport.updateResult(jobId, result), jobId),
  };
}
