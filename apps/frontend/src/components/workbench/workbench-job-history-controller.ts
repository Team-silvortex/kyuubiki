"use client";

import { useCallback, useRef, useState, type Dispatch, type MutableRefObject, type SetStateAction, type TransitionStartFunction } from "react";
import type { JobEnvelope, JobState } from "@/lib/api/fem-shared";
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
  job: JobEnvelope["job"] | null;
  jobHistoryBackendService?: WorkbenchJobHistoryBackendService;
  jobIsActive: boolean;
  jobPollTokenRef: MutableRefObject<number>;
  setJob: Dispatch<SetStateAction<JobEnvelope["job"] | null>>;
  setMessage: Dispatch<SetStateAction<string>>;
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
};

export async function cancelWorkbenchJob({
  jobId,
  jobHistoryBackendService,
  jobPollTokenRef,
  labels,
  refreshJobHistory,
  setJob,
  setMessage,
}: CancelWorkbenchJobArgs): Promise<WorkbenchOperationResult<{ jobId: string }>> {
  try {
    const payload = await jobHistoryBackendService.cancelJob(jobId);
    jobPollTokenRef.current += 1;
    setJob(payload.job);
    setMessage(labels.jobCancelled);
    await refreshJobHistory();
    return { ok: true, jobId };
  } catch (error) {
    const message = error instanceof Error
      ? error.message.startsWith("request timed out:")
        ? labels.requestTimedOut
        : error.message
      : labels.initialFailed;
    setMessage(message);
    return workbenchOperationFailure(new Error(message), labels.initialFailed);
  }
}

export function useWorkbenchJobHistoryController({
  labels,
  job,
  jobHistoryBackendService = workbenchJobHistoryBackendService,
  jobIsActive,
  jobPollTokenRef,
  setJob,
  setMessage,
  startTransition,
}: UseWorkbenchJobHistoryControllerArgs) {
  const [jobHistory, setJobHistory] = useState<JobState[]>([]);
  const [selectedAdminJobId, setSelectedAdminJobId] = useState<string | null>(null);
  const jobHistoryRefreshSeqRef = useRef(0);

  const refreshJobHistory = useCallback(async () => {
    const refreshSeq = ++jobHistoryRefreshSeqRef.current;

    try {
      const payload = await jobHistoryBackendService.fetchHistory();
      if (refreshSeq !== jobHistoryRefreshSeqRef.current) return;
      setJobHistory(payload.jobs);
      setSelectedAdminJobId((current) =>
        current && payload.jobs.some((entry) => entry.job_id === current) ? current : payload.jobs[0]?.job_id ?? null,
      );
    } catch {
      if (refreshSeq !== jobHistoryRefreshSeqRef.current) return;
      setJobHistory([]);
      setSelectedAdminJobId(null);
    }
  }, [jobHistoryBackendService]);

  const cancelCurrentJob = useCallback((): Promise<WorkbenchOperationResult<{ jobId: string }>> => {
    if (!job?.job_id || !jobIsActive) {
      return Promise.resolve(workbenchOperationFailure(
        new Error("No active job is available to cancel."),
        labels.initialFailed,
      ));
    }
    const jobId = job.job_id;

    return runWorkbenchTransitionOperation(startTransition, () => cancelWorkbenchJob({
      jobId,
      jobHistoryBackendService,
      jobPollTokenRef,
      labels,
      refreshJobHistory,
      setJob,
      setMessage,
    }));
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
