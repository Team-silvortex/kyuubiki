"use client";

import { ensurePyodideRuntime } from "@/lib/scripting/workbench-script-runtime";
import { buildLiveWorkbenchPythonBridge } from "@/lib/scripting/workbench-script-live-bridge";

type ExecuteWorkbenchPythonSourceInput = {
  appendOutput: (line: string) => void;
  source: string;
};

export async function executeWorkbenchPythonSource({
  appendOutput,
  source,
}: ExecuteWorkbenchPythonSourceInput) {
  const bridge = buildLiveWorkbenchPythonBridge(appendOutput);
  const pyodide = await ensurePyodideRuntime();
  window.__kyuubikiBridge = bridge;
  await pyodide.runPythonAsync(source);
}
