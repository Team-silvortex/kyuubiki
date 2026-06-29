"use client";

import type {
  WorkbenchAdminDataBackendService,
} from "@/lib/workbench/admin-data-backend-service-core";

type RefreshWorkbenchResultsDeps = {
  adminDataBackendService: WorkbenchAdminDataBackendService;
  resultRefreshSeqRef: { current: number };
  setResultRecords: (value: any[]) => void;
  setSelectedAdminResultJobId: (value: string | null | ((current: string | null) => string | null)) => void;
};

type AdminResultMutationDeps = RefreshWorkbenchResultsDeps & {
  selectedAdminResultJobId: string | null;
  adminResultDraft: string;
  downloadTextFile: (name: string, contents: string) => void;
  setMessage: (value: string) => void;
  labels: {
    resultSaved: string;
    resultDeleted: string;
    resultJsonDownloaded: string;
    invalidJson: string;
    initialFailed: string;
  };
};

export async function refreshWorkbenchResults({
  adminDataBackendService,
  resultRefreshSeqRef,
  setResultRecords,
  setSelectedAdminResultJobId,
}: RefreshWorkbenchResultsDeps) {
  const refreshSeq = ++resultRefreshSeqRef.current;

  try {
    const payload = await adminDataBackendService.fetchResults();
    if (refreshSeq !== resultRefreshSeqRef.current) return;
    setResultRecords(payload.results);
    setSelectedAdminResultJobId((current) =>
      current && payload.results.some((entry) => entry.job_id === current) ? current : payload.results[0]?.job_id ?? null,
    );
  } catch {
    if (refreshSeq !== resultRefreshSeqRef.current) return;
    setResultRecords([]);
    setSelectedAdminResultJobId(null);
  }
}

export async function saveWorkbenchAdminResultRecord({
  selectedAdminResultJobId,
  adminResultDraft,
  setMessage,
  labels,
  ...refreshDeps
}: AdminResultMutationDeps) {
  if (!selectedAdminResultJobId) return;

  try {
    const parsed = JSON.parse(adminResultDraft) as Record<string, unknown>;
    await refreshDeps.adminDataBackendService.updateResult(selectedAdminResultJobId, parsed);
    await refreshWorkbenchResults(refreshDeps);
    setMessage(labels.resultSaved);
  } catch (error) {
    setMessage(error instanceof SyntaxError ? labels.invalidJson : error instanceof Error ? error.message : labels.initialFailed);
  }
}

export async function deleteWorkbenchAdminResultRecord({
  selectedAdminResultJobId,
  setMessage,
  labels,
  ...refreshDeps
}: AdminResultMutationDeps) {
  if (!selectedAdminResultJobId) return;

  try {
    await refreshDeps.adminDataBackendService.deleteResult(selectedAdminResultJobId);
    await refreshWorkbenchResults(refreshDeps);
    setMessage(labels.resultDeleted);
  } catch (error) {
    setMessage(error instanceof Error ? error.message : labels.initialFailed);
  }
}

export function exportWorkbenchAdminResultRecord({
  selectedAdminResultJobId,
  adminResultDraft,
  downloadTextFile,
  setMessage,
  labels,
}: AdminResultMutationDeps) {
  if (!selectedAdminResultJobId) return;

  try {
    const parsed = JSON.parse(adminResultDraft);
    downloadTextFile(`${selectedAdminResultJobId}-result.json`, JSON.stringify(parsed, null, 2));
    setMessage(labels.resultJsonDownloaded);
  } catch {
    setMessage(labels.invalidJson);
  }
}
