export function createWorkbenchFeedback() {
  let revision = 0;
  let mounted = true;
  return {
    begin() {
      const owner = ++revision;
      return () => mounted && owner === revision;
    },
    invalidate() { revision += 1; },
    mount() { mounted = true; revision += 1; },
    dispose() { mounted = false; revision += 1; },
  };
}

export type WorkbenchFeedback = ReturnType<typeof createWorkbenchFeedback>;

export function withWorkbenchFeedbackInvalidation<T>(feedback: WorkbenchFeedback, setter: (value: T) => void) {
  return (value: T) => {
    // Intent matters even when React receives the same value or a functional update.
    feedback.invalidate();
    setter(value);
  };
}
