"use client";

type PyodideInterface = {
  runPythonAsync<T = unknown>(code: string): Promise<T>;
};

type LoadPyodideFunction = (options?: {
  indexURL?: string;
}) => Promise<PyodideInterface>;

declare global {
  interface Window {
    loadPyodide?: LoadPyodideFunction;
    __kyuubikiPyodidePromise?: Promise<PyodideInterface>;
    __kyuubikiBridge?: {
      invoke: (action: string, payloadJson?: string) => Promise<string>;
      state_json: () => string;
      actions_json: () => string;
      macros_json: () => string;
      log: (message: string) => void;
      sleep: (seconds?: number) => Promise<void>;
    };
  }
}

const PYODIDE_VERSION = "0.27.7";
const PYODIDE_SCRIPT_URL = `https://cdn.jsdelivr.net/pyodide/v${PYODIDE_VERSION}/full/pyodide.js`;
const PYODIDE_INDEX_URL = `https://cdn.jsdelivr.net/pyodide/v${PYODIDE_VERSION}/full/`;

let pyodideScriptPromise: Promise<void> | null = null;

function loadPyodideBrowserScript(): Promise<void> {
  if (typeof window === "undefined") {
    return Promise.reject(new Error("Pyodide can only load in the browser."));
  }

  if (window.loadPyodide) {
    return Promise.resolve();
  }

  if (pyodideScriptPromise) {
    return pyodideScriptPromise;
  }

  pyodideScriptPromise = new Promise((resolve, reject) => {
    const existing = document.querySelector<HTMLScriptElement>('script[data-pyodide="true"]');
    if (existing) {
      existing.addEventListener("load", () => resolve(), { once: true });
      existing.addEventListener("error", () => reject(new Error("Unable to load the Pyodide runtime.")), {
        once: true,
      });
      return;
    }

    const script = document.createElement("script");
    script.src = PYODIDE_SCRIPT_URL;
    script.async = true;
    script.dataset.pyodide = "true";
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("Unable to load the Pyodide runtime."));
    document.head.appendChild(script);
  });

  return pyodideScriptPromise;
}

export async function ensurePyodideRuntime(): Promise<PyodideInterface> {
  if (typeof window === "undefined") {
    throw new Error("Pyodide can only initialize in the browser.");
  }

  await loadPyodideBrowserScript();

  if (!window.loadPyodide) {
    throw new Error("Pyodide loader did not become available.");
  }

  if (!window.__kyuubikiPyodidePromise) {
    window.__kyuubikiPyodidePromise = window.loadPyodide({
      indexURL: PYODIDE_INDEX_URL,
    });
  }

  return window.__kyuubikiPyodidePromise;
}

export const DEFAULT_WORKBENCH_PYTHON = `# Kyuubiki frontend automation
# Available helpers:
# - state: current frontend snapshot (dict)
# - actions: action catalog (list[dict])
# - macros: macro catalog (list[dict])
# - ky.log(*parts)
# - await ky.invoke("action/id", payload_dict)
# - await ky.run_macro("macro/id", payload_dict)
# - await ky.run_macro_definition(macro_dict)
# - await ky.sleep(seconds)
# - await ky.wait_until(...)
# - await ky.wait_for_job_done()
# - await ky.wait_for_message("completed")

ky.log("Current study:", state["studyKind"])
ky.log("Current sidebar:", state["sidebarSection"])

# Example: refresh runtime surfaces first.
await ky.invoke("runtime/refreshAll")

# Example: branch your automation on the current study kind.
if state["studyKind"] == "axial_bar_1d":
    await ky.invoke("nav/setStudyKind", {"studyKind": "truss_2d"})
    await ky.invoke("state/setParametric", {"bays": 6, "span": 18, "height": 3.5, "loadY": -1500})
    await ky.invoke("model/generateTruss")

ky.log("Submitting study...")
await ky.invoke("job/run")
`;

export function buildWorkbenchPythonPrelude(): string {
  return `
import json
from js import __kyuubikiBridge

class _KyuubikiBridge:
    def state(self):
        return json.loads(__kyuubikiBridge.state_json())

    def actions(self):
        return json.loads(__kyuubikiBridge.actions_json())

    def macros(self):
        return json.loads(__kyuubikiBridge.macros_json())

    def log(self, *parts):
        __kyuubikiBridge.log(" ".join(str(part) for part in parts))

    async def invoke(self, action, payload=None):
        if payload is None:
            payload = {}
        result = await __kyuubikiBridge.invoke(action, json.dumps(payload))
        return json.loads(result)

    async def run_macro(self, macro, payload=None):
        if payload is None:
            payload = {}
        return await self.invoke("macro/run", {"macroId": macro, **payload})

    async def run_steps(self, steps):
        results = []
        for step in steps:
            action = step.get("action")
            payload = step.get("payload", {})
            results.append(await self.invoke(action, payload))
        return results

    async def run_macro_definition(self, macro):
        return await self.run_steps(macro.get("steps", []))

    async def sleep(self, seconds=0.0):
        await __kyuubikiBridge.sleep(seconds)

    async def wait_until(self, predicate, timeout=30.0, interval=0.25):
        elapsed = 0.0
        while elapsed <= timeout:
            current = self.state()
            if predicate(current):
                return current
            await self.sleep(interval)
            elapsed += interval
        raise TimeoutError(f"Condition not met within {timeout} seconds")

    async def wait_for_job_done(self, timeout=90.0, interval=0.5):
        terminal = {"completed", "failed", "cancelled"}
        return await self.wait_until(
            lambda current: current.get("jobStatus") in terminal,
            timeout=timeout,
            interval=interval,
        )

    async def wait_for_message(self, text, timeout=30.0, interval=0.25):
        needle = str(text)
        return await self.wait_until(
            lambda current: needle in str(current.get("message", "")),
            timeout=timeout,
            interval=interval,
        )

ky = _KyuubikiBridge()
state = ky.state()
actions = ky.actions()
macros = ky.macros()
`;
}

