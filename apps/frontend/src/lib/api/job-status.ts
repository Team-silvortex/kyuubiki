import type { WorkflowProgressEvent } from "./workflow-types.ts";

export type WorkflowRunStatus = WorkflowProgressEvent["stage"];
export type WorkflowRunPollingState = "attached" | "detached";
export type JobStatusFailureClass =
  | "watchdog_stalled"
  | "watchdog_timeout"
  | "execution_timeout"
  | "runtime_failure"
  | "operator_cancelled"
  | "cancelled";
export type JobStatusTimingDetail = {
  phase: "queue" | "execution";
  queue_wait_ms: number;
  execution_elapsed_ms: number | null;
  total_elapsed_ms: number;
  queue_timeout_ms: number | null;
  execution_timeout_ms: number | null;
  effective_timeout_ms: number | null;
  job_submission_deadline: string | null;
  execution_started_at: string | null;
  effective_deadline: string | null;
};
export type JobStatusDetail = {
  lifecycle: "active" | "terminal";
  active: boolean;
  terminal: boolean;
  failure_class?: JobStatusFailureClass | null;
  recoverable: boolean;
  timing: JobStatusTimingDetail;
};

const ACTIVE_WORKFLOW_RUN_STATUSES: WorkflowRunStatus[] = [
  "queued",
  "preprocessing",
  "partitioning",
  "solving",
  "postprocessing",
];

const TERMINAL_WORKFLOW_RUN_STATUSES: WorkflowRunStatus[] = [
  "completed",
  "failed",
  "cancelled",
];

const JOB_STATUS_FAILURE_CLASSES: JobStatusFailureClass[] = [
  "watchdog_stalled",
  "watchdog_timeout",
  "execution_timeout",
  "runtime_failure",
  "operator_cancelled",
  "cancelled",
];

export function isWorkflowRunStatus(value: string): value is WorkflowRunStatus {
  return (
    ACTIVE_WORKFLOW_RUN_STATUSES.includes(value as WorkflowRunStatus) ||
    TERMINAL_WORKFLOW_RUN_STATUSES.includes(value as WorkflowRunStatus)
  );
}

export function isWorkflowRunActiveStatus(status: string): status is WorkflowRunStatus {
  return ACTIVE_WORKFLOW_RUN_STATUSES.includes(status as WorkflowRunStatus);
}

export function isWorkflowRunTerminalStatus(status: string): status is WorkflowRunStatus {
  return TERMINAL_WORKFLOW_RUN_STATUSES.includes(status as WorkflowRunStatus);
}

export function isWorkflowRunFailureStatus(status: string) {
  return status === "failed" || status === "cancelled";
}

function isRecoverableFailureClass(failureClass?: JobStatusFailureClass | null) {
  return (
    failureClass === "watchdog_stalled" ||
    failureClass === "watchdog_timeout" ||
    failureClass === "execution_timeout" ||
    failureClass === "operator_cancelled"
  );
}

function isJobStatusFailureClass(value: unknown): value is JobStatusFailureClass {
  return JOB_STATUS_FAILURE_CLASSES.includes(value as JobStatusFailureClass);
}

function isNonNegativeInteger(value: unknown) {
  return Number.isInteger(value) && Number(value) >= 0;
}

function isNullablePositiveInteger(value: unknown) {
  return value === null || (Number.isInteger(value) && Number(value) > 0);
}

function isNullableDateTime(value: unknown) {
  return value === null || (typeof value === "string" && value.length > 0 && Number.isFinite(Date.parse(value)));
}

function isJobStatusTimingDetailValid(value: unknown): value is JobStatusTimingDetail {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const timing = value as Partial<JobStatusTimingDetail>;
  return (
    (timing.phase === "queue" || timing.phase === "execution") &&
    isNonNegativeInteger(timing.queue_wait_ms) &&
    (timing.execution_elapsed_ms === null || isNonNegativeInteger(timing.execution_elapsed_ms)) &&
    isNonNegativeInteger(timing.total_elapsed_ms) &&
    isNullablePositiveInteger(timing.queue_timeout_ms) &&
    isNullablePositiveInteger(timing.execution_timeout_ms) &&
    isNullablePositiveInteger(timing.effective_timeout_ms) &&
    isNullableDateTime(timing.job_submission_deadline) &&
    isNullableDateTime(timing.execution_started_at) &&
    isNullableDateTime(timing.effective_deadline)
  );
}

export function isJobStatusDetailConsistent(
  status: string,
  detail?: JobStatusDetail | null,
) {
  if (!isWorkflowRunStatus(status)) return false;
  if (!detail) return true;

  const failureClass = detail.failure_class;
  if ((failureClass != null && !isJobStatusFailureClass(failureClass)) || !isJobStatusTimingDetailValid(detail.timing)) {
    return false;
  }

  const active = isWorkflowRunActiveStatus(status);
  const terminal = isWorkflowRunTerminalStatus(status);
  if (
    detail.active !== active ||
    detail.terminal !== terminal ||
    detail.lifecycle !== (active ? "active" : "terminal") ||
    detail.recoverable !== isRecoverableFailureClass(failureClass)
  ) {
    return false;
  }
  if ((active || status === "completed") && failureClass) return false;
  if (status === "failed") {
    return Boolean(failureClass && !["operator_cancelled", "cancelled"].includes(failureClass));
  }
  if (status === "cancelled") {
    return failureClass === "operator_cancelled" || failureClass === "cancelled";
  }
  return true;
}

export function isWorkflowJobStatusContractValid(input: {
  status: string;
  progress: unknown;
  statusDetail?: JobStatusDetail | null;
}) {
  return (
    typeof input.progress === "number" &&
    Number.isFinite(input.progress) &&
    input.progress >= 0 &&
    input.progress <= 1 &&
    isJobStatusDetailConsistent(input.status, input.statusDetail)
  );
}

export function normalizeWorkflowRunProgress(value: unknown) {
  if (typeof value !== "number" || !Number.isFinite(value)) return 0;
  return Math.min(Math.max(value, 0), 1);
}

export type WorkflowRunPollDisposition = "continue" | "completed" | "failure" | "invalid";

export function resolveWorkflowRunPollDisposition(
  status: string,
  detail: JobStatusDetail | null | undefined,
  hasExpectedResult: boolean,
): WorkflowRunPollDisposition {
  if (!isJobStatusDetailConsistent(status, detail)) return "invalid";
  if (isWorkflowRunActiveStatus(status)) return "continue";
  if (status === "completed") return hasExpectedResult ? "completed" : "invalid";
  if (isWorkflowRunFailureStatus(status)) return "failure";
  return "invalid";
}

export function resolveWorkflowRunStatusTone(
  status: string,
  pollingState: WorkflowRunPollingState = "attached",
  detail?: JobStatusDetail | null,
) {
  if (!isJobStatusDetailConsistent(status, detail)) return "risk";
  if (status === "completed") return "good";
  if (isWorkflowRunFailureStatus(status)) return "risk";
  return pollingState === "detached" ? "risk" : "watch";
}

export function resolveJobStatusDetailLabel(detail?: JobStatusDetail | null) {
  const failureClass = detail?.failure_class;
  if (!failureClass) return null;
  if (failureClass === "watchdog_stalled") return "stalled";
  if (failureClass === "watchdog_timeout") return "watchdog timeout";
  if (failureClass === "execution_timeout") return "execution timeout";
  if (failureClass === "operator_cancelled") return "operator cancelled";
  if (failureClass === "cancelled") return "cancelled";
  return "runtime failure";
}

export function resolveJobStatusDetailTone(
  detail?: JobStatusDetail | null,
  status?: string,
) {
  if (status && !isJobStatusDetailConsistent(status, detail)) return "risk";
  if (!detail?.failure_class) return "watch";
  return detail.recoverable ? "watch" : "risk";
}
