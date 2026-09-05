"use client";

import { useEffect, useRef, useState } from "react";
import type { createWorkbenchScriptInvoker } from "./workbench-script-invoker";

export type WorkbenchScriptInvoker = ReturnType<typeof createWorkbenchScriptInvoker>;

export function useWorkbenchScriptCommitBoundary(invoke: WorkbenchScriptInvoker): WorkbenchScriptInvoker {
  const latest = useRef(invoke);
  latest.current = invoke;
  const mounted = useRef(true);
  const requested = useRef(0);
  const [committed, setCommitted] = useState(0);
  const pending = useRef(new Map<number, { resolve: () => void; reject: (error: Error) => void }>());

  useEffect(() => {
    for (const [revision, waiter] of pending.current) {
      if (revision <= committed) {
        pending.current.delete(revision);
        waiter.resolve();
      }
    }
  }, [committed]);

  useEffect(() => {
    mounted.current = true;
    const waiters = pending.current;
    return () => {
      mounted.current = false;
      for (const waiter of waiters.values()) waiter.reject(new Error("Workbench automation host unmounted."));
      waiters.clear();
    };
  }, []);

  const stable = useRef<WorkbenchScriptInvoker | null>(null);
  if (!stable.current) {
    stable.current = async (...args) => {
      if (!mounted.current) throw new Error("Workbench automation host unmounted.");
      try {
        return await latest.current(...args);
      } finally {
        // Resolve only after React commits, so the next action reads the new controller state.
        await new Promise<void>((resolve, reject) => {
          if (!mounted.current) {
            reject(new Error("Workbench automation host unmounted."));
            return;
          }
          const revision = ++requested.current;
          pending.current.set(revision, { resolve, reject });
          setCommitted(revision);
        });
      }
    };
  }
  return stable.current;
}
