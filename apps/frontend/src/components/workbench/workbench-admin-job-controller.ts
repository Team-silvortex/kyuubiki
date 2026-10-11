"use client";

import type { Dispatch, SetStateAction } from "react";
import type { ResultRecord } from "@/lib/api/security-results-types";
import type { WorkbenchAdminDataBackendService } from "@/lib/workbench/admin-data-backend-service-core";

type AdminJobMutationDeps = {
  selectedAdminJobId: string | null;
  adminDataBackendService: WorkbenchAdminDataBackendService;
  refreshJobHistory: () => Promise<void>;
  setMessage: (value: string) => void;
  labels: { jobSaved: string; jobDeleted: string; initialFailed: string };
};

export async function saveWorkbenchAdminJobRecord(deps: AdminJobMutationDeps & {
  adminJobMessage: string;
  adminJobProjectId: string;
  adminJobModelVersionId: string;
  adminJobCaseId: string;
}) {
  const jobId = deps.selectedAdminJobId;
  if (!jobId) return;
  try {
    await deps.adminDataBackendService.updateJob(jobId, {
      message: deps.adminJobMessage,
      project_id: deps.adminJobProjectId,
      model_version_id: deps.adminJobModelVersionId || null,
      simulation_case_id: deps.adminJobCaseId,
    });
  } catch (error) {
    deps.setMessage(error instanceof Error ? error.message : deps.labels.initialFailed);
    return;
  }
  // Read controllers own recovery feedback; read failure does not revoke a confirmed write.
  await Promise.allSettled([deps.refreshJobHistory()]);
  deps.setMessage(deps.labels.jobSaved);
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
  try {
    await deps.adminDataBackendService.deleteJob(jobId);
  } catch (error) {
    deps.setMessage(error instanceof Error ? error.message : deps.labels.initialFailed);
    return;
  }
  deps.removeConfirmedJobRecord(jobId);
  deps.resultRefreshSeqRef.current += 1;
  deps.setResultRecords((current) => current.filter((entry) => entry.job_id !== jobId));
  deps.setSelectedAdminResultJobId((current) => current === jobId ? null : current);
  await Promise.allSettled([deps.refreshJobHistory(), deps.refreshResults()]);
  deps.setMessage(deps.labels.jobDeleted);
}
