"use client";

import type { Dispatch, SetStateAction } from "react";
import type { ResultRecord } from "@/lib/api/security-results-types";
import { normalizeWorkbenchRequestError } from "@/lib/api/request-errors";
import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import type { WorkbenchFeedback } from "./workbench-feedback";
import { isWorkbenchAdminRecordSourceCurrent, workbenchAdminRecordSources,
  type WorkbenchAdminRecordSource } from "./workbench-admin-record-source";
import { clearWorkbenchRuntimeRecoveryIssue, upsertWorkbenchRuntimeRecoveryIssue,
  type WorkbenchRuntimeRecoveryState } from "./workbench-runtime-recovery";
import type {
  WorkbenchAdminDataBackendService,
} from "@/lib/workbench/admin-data-backend-service-core";

type RefreshWorkbenchResultsDeps = {
  adminDataBackendService: WorkbenchAdminDataBackendService;
  resultRefreshSeqRef: { current: number };
  setResultRecords: Dispatch<SetStateAction<ResultRecord[]>>;
  setSelectedAdminResultJobId: (value: string | null | ((current: string | null) => string | null)) => void;
  setResultReadPending: (value: boolean) => void;
  setRuntimeRecovery: Dispatch<SetStateAction<WorkbenchRuntimeRecoveryState>>;
  resultScopeLabel: string;
};

type AdminResultMutationDeps = RefreshWorkbenchResultsDeps & {
  selectedAdminResultJobId: string | null;
  adminResultDraft: string;
  downloadTextFile: (name: string, contents: string) => void;
  setMessage: (value: string) => void;
  feedback: WorkbenchFeedback;
  ownsFeedback?: () => boolean;
  recordSource: WorkbenchAdminRecordSource | null;
  draftSource: WorkbenchAdminRecordSource | null;
  labels: {
    resultSaved: string;
    resultDeleted: string;
    resultJsonDownloaded: string;
    invalidJson: string;
    initialFailed: string;
    sourceChanged: string;
  };
};

function parseResultDraft(draft: string): Record<string, unknown> {
  const parsed: unknown = JSON.parse(draft, (_key, value: unknown) => {
    // JSON.parse can overflow finite JSON numbers; serializing Infinity would silently emit null.
    if (typeof value === "number" && !Number.isFinite(value)) throw new SyntaxError("Non-finite result number.");
    return value;
  });
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new SyntaxError("Result payload must be an object.");
  }
  return parsed as Record<string, unknown>;
}

export async function refreshWorkbenchResults({
  adminDataBackendService,
  resultRefreshSeqRef,
  setResultRecords,
  setSelectedAdminResultJobId,
  setResultReadPending,
  setRuntimeRecovery,
  resultScopeLabel,
}: RefreshWorkbenchResultsDeps) {
  const refreshSeq = ++resultRefreshSeqRef.current;
  const ownsAuthority = captureWorkbenchApiAuthority("/api/v1/results");
  setResultReadPending(true);

  try {
    const payload = await adminDataBackendService.fetchResults();
    if (refreshSeq !== resultRefreshSeqRef.current) return;
    if (!ownsAuthority()) { setResultReadPending(false); return; }
    workbenchAdminRecordSources.mark(payload.results);
    setResultRecords(payload.results);
    setSelectedAdminResultJobId((current) =>
      current && payload.results.some((entry) => entry.job_id === current) ? current : payload.results[0]?.job_id ?? null,
    );
    setResultReadPending(false);
    setRuntimeRecovery((current) => clearWorkbenchRuntimeRecoveryIssue(current, "results"));
  } catch (error) {
    if (refreshSeq !== resultRefreshSeqRef.current) return;
    if (!ownsAuthority()) { setResultReadPending(false); return; }
    setResultReadPending(false);
    const requestError = normalizeWorkbenchRequestError(error, resultScopeLabel);
    setRuntimeRecovery((current) => upsertWorkbenchRuntimeRecoveryIssue({
      current, channel: "results", error: requestError, scopeLabel: resultScopeLabel,
    }));
  }
}

export async function saveWorkbenchAdminResultRecord({
  selectedAdminResultJobId,
  adminResultDraft,
  setMessage,
  feedback,
  ownsFeedback: capturedFeedback,
  recordSource,
  draftSource,
  labels,
  ...refreshDeps
}: AdminResultMutationDeps) {
  if (!selectedAdminResultJobId) return;
  const ownsFeedback = capturedFeedback ?? feedback.begin();
  if (!isWorkbenchAdminRecordSourceCurrent(recordSource, draftSource)) {
    if (ownsFeedback()) setMessage(labels.sourceChanged);
    return;
  }
  const ownsAuthority = captureWorkbenchApiAuthority(`/api/v1/results/${encodeURIComponent(selectedAdminResultJobId)}`, "/api/v1/results");

  let parsed: Record<string, unknown>;
  try {
    parsed = parseResultDraft(adminResultDraft);
  } catch {
    if (ownsAuthority() && ownsFeedback()) setMessage(labels.invalidJson);
    return;
  }
  try {
    await refreshDeps.adminDataBackendService.updateResult(selectedAdminResultJobId, parsed);
    if (!ownsAuthority()) return;
    await refreshWorkbenchResults(refreshDeps);
    if (ownsAuthority() && ownsFeedback()) setMessage(labels.resultSaved);
  } catch (error) {
    if (ownsAuthority() && ownsFeedback()) setMessage(error instanceof Error ? error.message : labels.initialFailed);
  }
}

export async function deleteWorkbenchAdminResultRecord({
  selectedAdminResultJobId,
  setMessage,
  feedback,
  ownsFeedback: capturedFeedback,
  recordSource,
  draftSource,
  labels,
  ...refreshDeps
}: AdminResultMutationDeps) {
  if (!selectedAdminResultJobId) return;
  const ownsFeedback = capturedFeedback ?? feedback.begin();
  if (!isWorkbenchAdminRecordSourceCurrent(recordSource, draftSource)) {
    if (ownsFeedback()) setMessage(labels.sourceChanged);
    return;
  }
  const ownsAuthority = captureWorkbenchApiAuthority(`/api/v1/results/${encodeURIComponent(selectedAdminResultJobId)}`, "/api/v1/results");

  try {
    await refreshDeps.adminDataBackendService.deleteResult(selectedAdminResultJobId);
    if (!ownsAuthority()) return;
    // A confirmed deletion must not reappear locally when the follow-up read fails.
    refreshDeps.setResultRecords((current) => workbenchAdminRecordSources.filter(current, (entry) => entry.job_id !== selectedAdminResultJobId));
    refreshDeps.setSelectedAdminResultJobId((current) => current === selectedAdminResultJobId ? null : current);
    await refreshWorkbenchResults(refreshDeps);
    if (ownsAuthority() && ownsFeedback()) setMessage(labels.resultDeleted);
  } catch (error) {
    if (ownsAuthority() && ownsFeedback()) setMessage(error instanceof Error ? error.message : labels.initialFailed);
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

  let parsed: Record<string, unknown>;
  try {
    parsed = parseResultDraft(adminResultDraft);
  } catch {
    setMessage(labels.invalidJson);
    return;
  }
  try {
    downloadTextFile(`${selectedAdminResultJobId}-result.json`, JSON.stringify(parsed, null, 2));
    setMessage(labels.resultJsonDownloaded);
  } catch (error) {
    setMessage(error instanceof Error ? error.message : labels.initialFailed);
  }
}
