"use client";

export type WorkbenchRequestFailureKind = "offline" | "timeout" | "http" | "unknown";

type WorkbenchRequestErrorInput = {
  cause?: unknown;
  kind: WorkbenchRequestFailureKind;
  message: string;
  responseMessage?: string | null;
  statusCode?: number;
  url: string;
};

function buildRecoveryHint(kind: WorkbenchRequestFailureKind, statusCode?: number) {
  if (kind === "offline") {
    return "Runtime endpoint is unavailable. Verify the hub, agent, or local backend process and retry.";
  }

  if (kind === "timeout") {
    return "Runtime request timed out. Retry once the service is responsive, or reduce concurrent load.";
  }

  if (kind === "http" && statusCode === 401) {
    return "Authentication failed. Recheck control-plane or direct-mesh tokens before retrying.";
  }

  if (kind === "http" && statusCode === 403) {
    return "Access was denied by runtime governance. Review the current execution mode and token scope.";
  }

  if (kind === "http" && statusCode === 404) {
    return "Requested runtime resource was not found. Refresh the catalog or verify the selected target still exists.";
  }

  if (kind === "http" && statusCode && statusCode >= 500) {
    return "Runtime service reported an internal failure. Retry after the backend stabilizes.";
  }

  return "Request failed. Inspect runtime status, then retry the affected refresh step.";
}

function inferFailureKind(error: unknown): WorkbenchRequestFailureKind {
  if (error instanceof WorkbenchRequestError) {
    return error.kind;
  }

  if (error instanceof TypeError && /fetch/i.test(error.message)) {
    return "offline";
  }

  if (error instanceof Error && /timed out/i.test(error.message)) {
    return "timeout";
  }

  return "unknown";
}

export class WorkbenchRequestError extends Error {
  readonly kind: WorkbenchRequestFailureKind;
  readonly retryable: boolean;
  readonly responseMessage: string | null;
  readonly recoveryHint: string;
  readonly statusCode?: number;
  readonly url: string;

  constructor(input: WorkbenchRequestErrorInput) {
    super(input.message, input.cause ? { cause: input.cause } : undefined);
    this.name = "WorkbenchRequestError";
    this.kind = input.kind;
    this.statusCode = input.statusCode;
    this.url = input.url;
    this.responseMessage = input.responseMessage ?? null;
    this.retryable =
      input.kind === "offline" ||
      input.kind === "timeout" ||
      (input.kind === "http" && Boolean(input.statusCode && input.statusCode >= 500));
    this.recoveryHint = buildRecoveryHint(input.kind, input.statusCode);
  }
}

export function createHttpWorkbenchRequestError(params: {
  message: string;
  responseMessage?: string | null;
  statusCode: number;
  url: string;
}) {
  return new WorkbenchRequestError({
    kind: "http",
    message: params.message,
    responseMessage: params.responseMessage,
    statusCode: params.statusCode,
    url: params.url,
  });
}

export function normalizeWorkbenchRequestError(error: unknown, url: string) {
  if (error instanceof WorkbenchRequestError) {
    return error;
  }

  const fallbackMessage = error instanceof Error ? error.message : "Request failed.";
  const kind = inferFailureKind(error);

  return new WorkbenchRequestError({
    cause: error,
    kind,
    message: fallbackMessage,
    url,
  });
}
