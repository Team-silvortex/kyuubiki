"use client";

import {
  ensurePyodideRuntime,
  buildWorkbenchPyodideBridge,
} from "@/lib/scripting/workbench-script-runtime";

type ExecuteWorkbenchPythonSourceInput = {
  appendOutput: (line: string) => void;
  getSnapshot: () => unknown;
  onInvokeAction: (action: string, payload?: Record<string, unknown>) => Promise<unknown>;
  source: string;
};

export async function executeWorkbenchPythonSource({
  appendOutput,
  getSnapshot,
  onInvokeAction,
  source,
}: ExecuteWorkbenchPythonSourceInput) {
  const pyodide = await ensurePyodideRuntime();
  window.__kyuubikiBridge = buildWorkbenchPyodideBridge({
    appendOutput,
    getSnapshot,
    invokeAction: onInvokeAction,
  });
  await pyodide.runPythonAsync(source);
}
