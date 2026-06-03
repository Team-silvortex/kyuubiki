"use client";

type RefreshWorkbenchResultsDeps = {
  resultRefreshSeqRef: { current: number };
  fetchResults: () => Promise<{ results: any[] }>;
  setResultRecords: (value: any[]) => void;
  setSelectedAdminResultJobId: (value: string | null | ((current: string | null) => string | null)) => void;
};

type AdminResultMutationDeps = RefreshWorkbenchResultsDeps & {
  selectedAdminResultJobId: string | null;
  adminResultDraft: string;
  updateResultRecord: (jobId: string, payload: Record<string, unknown>) => Promise<unknown>;
  deleteResultRecord: (jobId: string) => Promise<unknown>;
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
  resultRefreshSeqRef,
  fetchResults,
  setResultRecords,
  setSelectedAdminResultJobId,
}: RefreshWorkbenchResultsDeps) {
  const refreshSeq = ++resultRefreshSeqRef.current;

  try {
    const payload = await fetchResults();
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
  updateResultRecord,
  setMessage,
  labels,
  ...refreshDeps
}: AdminResultMutationDeps) {
  if (!selectedAdminResultJobId) return;

  try {
    const parsed = JSON.parse(adminResultDraft) as Record<string, unknown>;
    await updateResultRecord(selectedAdminResultJobId, parsed);
    await refreshWorkbenchResults(refreshDeps);
    setMessage(labels.resultSaved);
  } catch (error) {
    setMessage(error instanceof SyntaxError ? labels.invalidJson : error instanceof Error ? error.message : labels.initialFailed);
  }
}

export async function deleteWorkbenchAdminResultRecord({
  selectedAdminResultJobId,
  deleteResultRecord,
  setMessage,
  labels,
  ...refreshDeps
}: AdminResultMutationDeps) {
  if (!selectedAdminResultJobId) return;

  try {
    await deleteResultRecord(selectedAdminResultJobId);
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
