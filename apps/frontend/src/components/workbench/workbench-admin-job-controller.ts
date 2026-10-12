"use client";

import type { Dispatch, SetStateAction } from "react";
import type { ResultRecord } from "@/lib/api/security-results-types";
import type { JobEnvelope } from "@/lib/api/fem-shared";
import type { WorkbenchAdminDataBackendService } from "@/lib/workbench/admin-data-backend-service-core";
import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import type { WorkbenchFeedback } from "./workbench-feedback";
import { isWorkbenchAdminRecordSourceCurrent, workbenchAdminRecordSources, type WorkbenchAdminRecordSource } from "./workbench-admin-record-source";
import { readWorkbenchCancellationReceipt } from "./workbench-job-cancellation-receipt";

export async function cancelWorkbenchAdminJobRecord(deps: {
  selectedAdminJobId: string | null;
  recordSource: WorkbenchAdminRecordSource | null;
  feedback: WorkbenchFeedback;
  ownsFeedback?: () => boolean;
  cancelJob: (jobId: string) => Promise<JobEnvelope>;
  refreshJobHistory: () => Promise<void>;
  setMessage: (message: string) => void;
  labels: { jobCancelled: string; sourceChanged: string; initialFailed: string; requestTimedOut: string };
}) {
  const jobId = deps.selectedAdminJobId;
  if (!jobId) return;
  const ownsFeedback = deps.ownsFeedback ?? deps.feedback.begin();
  const ownsSource = () => isWorkbenchAdminRecordSourceCurrent(deps.recordSource, deps.recordSource);
  if (!ownsSource()) {
    if (ownsFeedback()) deps.setMessage(deps.labels.sourceChanged);
    return;
  }
  let ownsAuthority = () => false;
  const isCurrent = () => {
    try { return ownsSource() && ownsAuthority(); } catch { return false; }
  };
  try {
    ownsAuthority = captureWorkbenchApiAuthority(`/api/v1/jobs/${encodeURIComponent(jobId)}/cancel`, "/api/v1/jobs");
    readWorkbenchCancellationReceipt(await deps.cancelJob(jobId), jobId);
  } catch (error) {
    if (isCurrent() && ownsFeedback()) deps.setMessage(error instanceof Error
      ? error.message.startsWith("request timed out:") ? deps.labels.requestTimedOut : error.message
      : deps.labels.initialFailed);
    return;
  }
  if (!isCurrent()) return;
  // A historical record never takes ownership of the live observation merely by matching its ID.
  await Promise.allSettled([deps.refreshJobHistory()]);
  if (isCurrent() && ownsFeedback()) deps.setMessage(deps.labels.jobCancelled);
}

type AdminJobMutationDeps = {
  selectedAdminJobId: string | null;
  adminDataBackendService: WorkbenchAdminDataBackendService;
  refreshJobHistory: () => Promise<void>;
  setMessage: (value: string) => void;
  feedback: WorkbenchFeedback;
  ownsFeedback?: () => boolean;
  recordSource: WorkbenchAdminRecordSource | null;
  draftSource: WorkbenchAdminRecordSource | null;
  labels: { jobSaved: string; jobDeleted: string; initialFailed: string; sourceChanged: string };
};

export async function saveWorkbenchAdminJobRecord(deps: AdminJobMutationDeps & {
  adminJobMessage: string;
  adminJobProjectId: string;
  adminJobModelVersionId: string;
  adminJobCaseId: string;
}) {
  const jobId = deps.selectedAdminJobId;
  if (!jobId) return;
  const ownsFeedback = deps.ownsFeedback ?? deps.feedback.begin();
  if (!isWorkbenchAdminRecordSourceCurrent(deps.recordSource, deps.draftSource)) {
    if (ownsFeedback()) deps.setMessage(deps.labels.sourceChanged);
    return;
  }
  const ownsAuthority = captureWorkbenchApiAuthority(`/api/v1/jobs/${encodeURIComponent(jobId)}`, "/api/v1/jobs");
  try {
    await deps.adminDataBackendService.updateJob(jobId, {
      message: deps.adminJobMessage,
      project_id: deps.adminJobProjectId,
      model_version_id: deps.adminJobModelVersionId || null,
      simulation_case_id: deps.adminJobCaseId,
    });
  } catch (error) {
    if (ownsAuthority() && ownsFeedback()) deps.setMessage(error instanceof Error ? error.message : deps.labels.initialFailed);
    return;
  }
  // Read controllers own recovery feedback; read failure does not revoke a confirmed write.
  if (!ownsAuthority()) return;
  await Promise.allSettled([deps.refreshJobHistory()]);
  if (ownsAuthority() && ownsFeedback()) deps.setMessage(deps.labels.jobSaved);
}

export async function deleteWorkbenchAdminJobRecord(deps: AdminJobMutationDeps & {
  removeConfirmedJobRecord: (jobId: string) => void;
  resultRefreshSeqRef: { current: number };
  setResultRecords: Dispatch<SetStateAction<ResultRecord[]>>;
  setSelectedAdminResultJobId: Dispatch<SetStateAction<string | null>>;
  refreshResults: () => Promise<void>;
}) {
  const jobId = deps.selectedAdminJobId;
  if (!jobId) return;
  const ownsFeedback = deps.ownsFeedback ?? deps.feedback.begin();
  if (!isWorkbenchAdminRecordSourceCurrent(deps.recordSource, deps.draftSource)) {
    if (ownsFeedback()) deps.setMessage(deps.labels.sourceChanged);
    return;
  }
  const ownsAuthority = captureWorkbenchApiAuthority(`/api/v1/jobs/${encodeURIComponent(jobId)}`, "/api/v1/jobs", "/api/v1/results");
  try {
    await deps.adminDataBackendService.deleteJob(jobId);
  } catch (error) {
    if (ownsAuthority() && ownsFeedback()) deps.setMessage(error instanceof Error ? error.message : deps.labels.initialFailed);
    return;
  }
  if (!ownsAuthority()) return;
  deps.removeConfirmedJobRecord(jobId);
  deps.resultRefreshSeqRef.current += 1;
  deps.setResultRecords((current) => workbenchAdminRecordSources.filter(current, (entry) => entry.job_id !== jobId));
  deps.setSelectedAdminResultJobId((current) => current === jobId ? null : current);
  await Promise.allSettled([deps.refreshJobHistory(), deps.refreshResults()]);
  if (ownsAuthority() && ownsFeedback()) deps.setMessage(deps.labels.jobDeleted);
}
