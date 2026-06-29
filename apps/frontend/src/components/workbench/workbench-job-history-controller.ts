"use client";

import { useCallback, useRef, useState, type Dispatch, type MutableRefObject, type SetStateAction, type TransitionStartFunction } from "react";
import type { JobEnvelope, JobState } from "@/lib/api/fem-shared";
import {
  workbenchJobHistoryBackendService,
} from "@/lib/workbench/job-history-backend-service";
import type {
  WorkbenchJobHistoryBackendService,
} from "@/lib/workbench/job-history-backend-service-core";

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

  const cancelCurrentJob = useCallback(() => {
    if (!job?.job_id || !jobIsActive) return;
    jobPollTokenRef.current += 1;

    startTransition(async () => {
      try {
        const payload = await jobHistoryBackendService.cancelJob(job.job_id);
        setJob(payload.job);
        setMessage(labels.jobCancelled);
        await refreshJobHistory();
      } catch (error) {
        setMessage(
          error instanceof Error
            ? error.message.startsWith("request timed out:")
              ? labels.requestTimedOut
              : error.message
            : labels.initialFailed,
        );
      }
    });
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
