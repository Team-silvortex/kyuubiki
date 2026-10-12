export function createWorkbenchAdminRecordMutations(onChange: (jobIds: string[]) => void) {
  const flights = new Map<string, Promise<void>>();
  let mounted = true;
  const publish = () => { if (mounted) onChange([...flights.keys()]); };

  return {
    run(jobId: string, action: () => Promise<void>, onAccepted?: () => void): Promise<void> {
      if (!mounted) return Promise.resolve();
      const pending = flights.get(jobId);
      if (pending) return pending;
      onAccepted?.();
      // Job and Result share their parent identity; reserve it before the first async write.
      const operation = Promise.resolve().then(action);
      flights.set(jobId, operation);
      publish();
      const release = () => {
        if (flights.get(jobId) !== operation) return;
        flights.delete(jobId);
        publish();
      };
      void operation.then(release, release);
      return operation;
    },
    mount() { mounted = true; publish(); },
    dispose() { mounted = false; },
  };
}

export type WorkbenchAdminRecordMutations = ReturnType<typeof createWorkbenchAdminRecordMutations>;
