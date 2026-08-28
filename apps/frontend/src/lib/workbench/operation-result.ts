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

export function runWorkbenchTransitionOperation<T>(
  startTransition: ((callback: () => void) => void) | undefined,
  run: () => Promise<T>,
): Promise<T> {
  if (!startTransition) return run();

  return new Promise((resolve, reject) => {
    startTransition(() => {
      void run().then(resolve, reject);
    });
  });
}
