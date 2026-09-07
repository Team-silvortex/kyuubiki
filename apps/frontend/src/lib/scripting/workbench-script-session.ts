type RuntimeStatus = "idle" | "loading" | "ready" | "running" | "error";
type SessionSnapshot = { output: string[]; runtimeStatus: RuntimeStatus; runtimeError: string | null };

export function createWorkbenchScriptSession() {
  const initial: SessionSnapshot = { output: [], runtimeStatus: "idle", runtimeError: null };
  let snapshot = initial;
  const listeners = new Set<() => void>();
  const publish = (patch: Partial<SessionSnapshot>) => {
    snapshot = { ...snapshot, ...patch };
    listeners.forEach((listener) => listener());
  };
  return {
    getSnapshot: () => snapshot,
    getServerSnapshot: () => initial,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },
    isBusy: () => snapshot.runtimeStatus === "loading" || snapshot.runtimeStatus === "running",
    setOutput(value: string[] | ((current: string[]) => string[])) {
      const output = typeof value === "function" ? value(snapshot.output) : value;
      publish({ output: output.slice(-200) });
    },
    setRuntimeStatus(runtimeStatus: RuntimeStatus) { publish({ runtimeStatus }); },
    setRuntimeError(runtimeError: string | null) { publish({ runtimeError }); },
  };
}

// The view can unload; an in-flight script and its bounded output belong to this WebView session.
export const workbenchScriptSession = createWorkbenchScriptSession();
