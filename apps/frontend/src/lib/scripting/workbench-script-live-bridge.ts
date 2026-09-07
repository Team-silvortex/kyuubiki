import { buildWorkbenchPyodideBridge } from "./workbench-script-browser-bridge.ts";

export function buildLiveWorkbenchPythonBridge(appendOutput: (line: string) => void) {
  const controller = typeof window === "undefined" ? undefined : window.__kyuubikiPwdt;
  const currentController = () => {
    if (!controller || window.__kyuubikiPwdt !== controller) {
      throw new Error("WORKBENCH_CONTEXT_CHANGED: the Python execution workspace is no longer available.");
    }
    return controller;
  };
  currentController();
  return buildWorkbenchPyodideBridge({
    appendOutput,
    getSnapshot: () => currentController().state(),
    invokeAction: (action, payload) => currentController().invoke(action, payload),
  });
}
