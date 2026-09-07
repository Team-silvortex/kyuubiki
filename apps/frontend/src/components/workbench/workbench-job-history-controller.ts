"use client";

import { useCallback, useRef, useState, type Dispatch, type MutableRefObject, type SetStateAction, type TransitionStartFunction } from "react";
import type { JobEnvelope, JobHistoryPayload, JobState } from "@/lib/api/fem-shared";
import { isWorkflowJobStatusContractValid } from "@/lib/api/job-status";
import { normalizeWorkbenchRequestError } from "@/lib/api/request-errors";
import {
  clearWorkbenchRuntimeRecoveryIssue, upsertWorkbenchRuntimeRecoveryIssue,
  type WorkbenchRuntimeRecoveryState,
} from "./workbench-runtime-recovery";
import {
  workbenchJobHistoryBackendService,
} from "@/lib/workbench/job-history-backend-service";
import type {
  WorkbenchJobHistoryBackendService,
} from "@/lib/workbench/job-history-backend-service-core";
import {
  runWorkbenchTransitionOperation,
  workbenchOperationFailure,
  type WorkbenchOperationResult,
} from "@/lib/workbench/operation-result";

type JobHistoryControllerLabels = {
  jobCancelled: string;
  initialFailed: string;
  requestTimedOut: string;
};

type UseWorkbenchJobHistoryControllerArgs = {
  labels: JobHistoryControllerLabels;
  historyScopeLabel: string;
  job: JobEnvelope["job"] | null;
  jobHistoryBackendService?: WorkbenchJobHistoryBackendService;
  jobIsActive: boolean;
  jobPollTokenRef: MutableRefObject<number>;
  setJob: Dispatch<SetStateAction<JobEnvelope["job"] | null>>;
  setMessage: Dispatch<SetStateAction<string>>;
  setResult: (value: null) => void;
  setRuntimeRecovery: Dispatch<SetStateAction<WorkbenchRuntimeRecoveryState>>;
  startTransition: TransitionStartFunction;
};

type CancelWorkbenchJobArgs = {
  jobId: string;
  jobHistoryBackendService: WorkbenchJobHistoryBackendService;
  jobPollTokenRef: MutableRefObject<number>;
  labels: JobHistoryControllerLabels;
  refreshJobHistory: () => Promise<void>;
  setJob: Dispatch<SetStateAction<JobEnvelope["job"] | null>>;
  setMessage: Dispatch<SetStateAction<string>>;
  setResult: (value: null) => void;
};

export type WorkbenchCancellationResult = WorkbenchOperationResult<{ jobId: string; contextChanged?: boolean }>;

export function readWorkbenchJobHistory(payload: unknown): JobState[] {
  const jobs = (payload as JobHistoryPayload | null)?.jobs;
  if (!Array.isArray(jobs)) throw new Error("JOB_HISTORY_INVALID: expected a job list.");
  const ids = new Set<string>();
  for (const job of jobs) {
    if (!job || typeof job.job_id !== "string" || !job.job_id.trim() || ids.has(job.job_id) ||
      !isWorkflowJobStatusContractValid({ status: job.status, progress: job.progress, statusDetail: job.status_detail })) {
      throw new Error("JOB_HISTORY_INVALID: invalid or duplicate job record.");
    }
    ids.add(job.job_id);
  }
  return jobs;
}

export async function cancelWorkbenchJob({
  jobId,
  jobHistoryBackendService,
  jobPollTokenRef,
  labels,
  refreshJobHistory,
  setJob,
  setMessage,
  setResult,
}: CancelWorkbenchJobArgs): Promise<WorkbenchCancellationResult> {
  let observation = jobPollTokenRef.current;
  try {
    const payload = await jobHistoryBackendService.cancelJob(jobId);
    if (payload.job?.job_id !== jobId) throw new Error("Cancellation response belongs to a different job.");
    if (!isWorkflowJobStatusContractValid({
      status: payload.job.status, progress: payload.job.progress, statusDetail: payload.job.status_detail,
    })) throw new Error("JOB_CANCELLATION_INVALID: invalid job status contract.");
    if (payload.job.status !== "cancelled") {
      throw new Error(`JOB_CANCELLATION_NOT_CONFIRMED: backend reported ${payload.job.status}; job observation remains active.`);
    }
    if (observation === jobPollTokenRef.current) {
      observation = ++jobPollTokenRef.current;
      setJob(payload.job);
      setResult(null);
      setMessage(labels.jobCancelled);
    }
  } catch (error) {
    const message = error instanceof Error
      ? error.message.startsWith("request timed out:")
        ? labels.requestTimedOut
        : error.message
      : labels.initialFailed;
    if (observation === jobPollTokenRef.current) setMessage(message);
    return workbenchOperationFailure(new Error(message), labels.initialFailed);
  }
  // A read failure cannot turn a confirmed backend write into a failed cancellation.
  try { await refreshJobHistory(); } catch { /* The refresh controller owns recovery feedback. */ }
  return { ok: true, jobId, ...(observation !== jobPollTokenRef.current ? { contextChanged: true } : {}) };
}

export function useWorkbenchJobHistoryController({
  labels,
  historyScopeLabel,
  job,
  jobHistoryBackendService = workbenchJobHistoryBackendService,
  jobIsActive,
  jobPollTokenRef,
  setJob,
  setMessage,
  setResult,
  setRuntimeRecovery,
  startTransition,
}: UseWorkbenchJobHistoryControllerArgs) {
  const [jobHistory, setJobHistory] = useState<JobState[]>([]);
  const [selectedAdminJobId, setSelectedAdminJobId] = useState<string | null>(null);
  const jobHistoryRefreshSeqRef = useRef(0);
  const pendingCancellations = useRef(new Map<string, Promise<WorkbenchCancellationResult>>());

  const refreshJobHistory = useCallback(async () => {
    const refreshSeq = ++jobHistoryRefreshSeqRef.current;

    try {
      const payload = await jobHistoryBackendService.fetchHistory();
      if (refreshSeq !== jobHistoryRefreshSeqRef.current) return;
      const jobs = readWorkbenchJobHistory(payload);
      setJobHistory(jobs);
      setSelectedAdminJobId((current) =>
        current && jobs.some((entry) => entry.job_id === current) ? current : jobs[0]?.job_id ?? null,
      );
      setRuntimeRecovery((current) => clearWorkbenchRuntimeRecoveryIssue(current, "job_history"));
    } catch (error) {
      if (refreshSeq !== jobHistoryRefreshSeqRef.current) return;
      setRuntimeRecovery((current) => upsertWorkbenchRuntimeRecoveryIssue({
        current, channel: "job_history", scopeLabel: historyScopeLabel,
        error: normalizeWorkbenchRequestError(error, "/api/v1/jobs"),
      }));
    }
  }, [jobHistoryBackendService, historyScopeLabel, setRuntimeRecovery]);

  const cancelCurrentJob = useCallback((): Promise<WorkbenchCancellationResult> => {
    // The receipt remains in flight through refresh, even after the job becomes terminal.
    const pending = job?.job_id ? pendingCancellations.current.get(job.job_id) : undefined;
    if (pending) return pending;
    if (!job?.job_id || !jobIsActive) {
      return Promise.resolve(workbenchOperationFailure(
        new Error("No active job is available to cancel."),
        labels.initialFailed,
      ));
    }
    const jobId = job.job_id;

    const operation = runWorkbenchTransitionOperation(startTransition, () => cancelWorkbenchJob({
      jobId,
      jobHistoryBackendService,
      jobPollTokenRef,
      labels,
      refreshJobHistory,
      setJob,
      setMessage,
      setResult,
    }));
    pendingCancellations.current.set(jobId, operation);
    const release = () => { pendingCancellations.current.delete(jobId); };
    void operation.then(release, release);
    return operation;
  }, [
    job,
    jobHistoryBackendService,
    jobIsActive,
    jobPollTokenRef,
    labels.initialFailed,
    labels.jobCancelled,
    labels.requestTimedOut,
    refreshJobHistory,
    setJob,
    setMessage,
    setResult,
    startTransition,
  ]);

  return {
    jobHistory,
    setJobHistory,
    selectedAdminJobId,
    setSelectedAdminJobId,
    refreshJobHistory,
    cancelCurrentJob,
  };
}
