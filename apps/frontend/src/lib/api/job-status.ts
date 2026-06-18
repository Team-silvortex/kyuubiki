import type { WorkflowProgressEvent } from "./workflow-types";

export type WorkflowRunStatus = WorkflowProgressEvent["stage"];
export type WorkflowRunPollingState = "attached" | "detached";
export type JobStatusFailureClass =
  | "watchdog_stalled"
  | "watchdog_timeout"
  | "execution_timeout"
  | "runtime_failure"
  | "operator_cancelled"
  | "cancelled";
export type JobStatusDetail = {
  lifecycle: "active" | "terminal";
  active: boolean;
  terminal: boolean;
  failure_class?: JobStatusFailureClass | null;
  recoverable: boolean;
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

export function resolveWorkflowRunStatusTone(
  status: string,
  pollingState: WorkflowRunPollingState = "attached",
) {
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

export function resolveJobStatusDetailTone(detail?: JobStatusDetail | null) {
  if (!detail?.failure_class) return "watch";
  return detail.recoverable ? "watch" : "risk";
}
