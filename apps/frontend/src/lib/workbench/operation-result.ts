export type WorkbenchOperationFailure = {
  ok: false;
  error: Error;
};

export type WorkbenchOperationResult<T extends object = Record<never, never>> =
  | ({ ok: true } & T)
  | WorkbenchOperationFailure;

export function workbenchOperationFailure(
  error: unknown,
  fallback: string,
): WorkbenchOperationFailure {
  return {
    ok: false,
    error: error instanceof Error ? error : new Error(fallback),
  };
}
