"use client";

import {
  buildWorkbenchUiAutomationContractSnapshot,
  ensurePyodideRuntime,
  WORKBENCH_SCRIPT_ACTIONS,
  WORKBENCH_SCRIPT_MACROS,
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
  window.__kyuubikiBridge = {
    invoke: async (action: string, payloadJson?: string) => {
      const payload =
        payloadJson && payloadJson.trim().length > 0
          ? (JSON.parse(payloadJson) as Record<string, unknown>)
          : {};
      const result = await onInvokeAction(action, payload);
      return JSON.stringify(result ?? { ok: true, action });
    },
    state_json: () => JSON.stringify(getSnapshot()),
    actions_json: () => JSON.stringify(WORKBENCH_SCRIPT_ACTIONS),
    macros_json: () => JSON.stringify(WORKBENCH_SCRIPT_MACROS),
    ui_contract_json: () => JSON.stringify(buildWorkbenchUiAutomationContractSnapshot()),
    log: (message: string) => appendOutput(message),
    sleep: async (seconds = 0) =>
      new Promise<void>((resolve) =>
        window.setTimeout(resolve, Math.max(0, seconds) * 1000),
      ),
  };
  await pyodide.runPythonAsync(source);
}
