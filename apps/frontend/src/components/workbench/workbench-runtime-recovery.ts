"use client";

import type { WorkbenchRequestError, WorkbenchRequestFailureKind } from "@/lib/api/request-errors";

export type WorkbenchRuntimeRecoveryChannel =
  | "health"
  | "projects"
  | "security_events"
  | "workflow_catalog";

export type WorkbenchRuntimeRecoveryIssue = {
  channel: WorkbenchRuntimeRecoveryChannel;
  kind: WorkbenchRequestFailureKind;
  lastFailureAt: string;
  message: string;
  recoveryHint: string;
  retryable: boolean;
  scopeLabel: string;
  statusCode?: number;
};

export type WorkbenchRuntimeRecoveryState = {
  availability: "healthy" | "degraded" | "offline";
  issues: WorkbenchRuntimeRecoveryIssue[];
  lastFailureAt: string | null;
};

export const EMPTY_WORKBENCH_RUNTIME_RECOVERY_STATE: WorkbenchRuntimeRecoveryState = {
  availability: "healthy",
  issues: [],
  lastFailureAt: null,
};

function deriveAvailability(issues: WorkbenchRuntimeRecoveryIssue[]) {
  if (issues.some((issue) => issue.kind === "offline")) {
    return "offline" as const;
  }

  return issues.length > 0 ? ("degraded" as const) : ("healthy" as const);
}

export function clearWorkbenchRuntimeRecoveryIssue(
  current: WorkbenchRuntimeRecoveryState,
  channel: WorkbenchRuntimeRecoveryChannel,
): WorkbenchRuntimeRecoveryState {
  const issues = current.issues.filter((issue) => issue.channel !== channel);
  return {
    availability: deriveAvailability(issues),
    issues,
    lastFailureAt: issues[0]?.lastFailureAt ?? null,
  };
}

export function upsertWorkbenchRuntimeRecoveryIssue(params: {
  channel: WorkbenchRuntimeRecoveryChannel;
  current: WorkbenchRuntimeRecoveryState;
  error: WorkbenchRequestError;
  scopeLabel: string;
}) {
  const nextIssue: WorkbenchRuntimeRecoveryIssue = {
    channel: params.channel,
    kind: params.error.kind,
    lastFailureAt: new Date().toISOString(),
    message: params.error.message,
    recoveryHint: params.error.recoveryHint,
    retryable: params.error.retryable,
    scopeLabel: params.scopeLabel,
    statusCode: params.error.statusCode,
  };
  const issues = [nextIssue, ...params.current.issues.filter((issue) => issue.channel !== params.channel)].sort(
    (left, right) => right.lastFailureAt.localeCompare(left.lastFailureAt),
  );

  return {
    availability: deriveAvailability(issues),
    issues,
    lastFailureAt: issues[0]?.lastFailureAt ?? null,
  } satisfies WorkbenchRuntimeRecoveryState;
}
