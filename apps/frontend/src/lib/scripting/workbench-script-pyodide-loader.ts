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
      recipes_json: () => string;
      ui_contract_json: () => string;
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

  if (typeof window.loadPyodide === "function") {
    return Promise.resolve();
  }

  if (pyodideScriptPromise) {
    return pyodideScriptPromise;
  }

  const attempt = new Promise<void>((resolve, reject) => {
    const existing = document.querySelector<HTMLScriptElement>('script[data-pyodide="true"]');
    const script = existing ?? document.createElement("script");
    const finish = (error?: Error) => {
      clearTimeout(timer);
      script.removeEventListener("load", loaded);
      script.removeEventListener("error", failed);
      if (error) {
        script.remove();
        reject(error);
      } else resolve();
    };
    const loaded = () => finish(typeof window.loadPyodide === "function"
      ? undefined : new Error("Pyodide loader did not become available."));
    const failed = () => finish(new Error("Unable to load the Pyodide runtime."));
    const timer = setTimeout(() => finish(new Error("Pyodide script download timed out. Retry loading the runtime.")), 30_000);
    script.addEventListener("load", loaded, { once: true });
    script.addEventListener("error", failed, { once: true });
    if (!existing) {
      script.src = PYODIDE_SCRIPT_URL;
      script.async = true;
      script.dataset.pyodide = "true";
      try { document.head.appendChild(script); } catch { failed(); }
    }
  });
  const tracked = attempt.catch((error) => {
    if (pyodideScriptPromise === tracked) pyodideScriptPromise = null;
    throw error;
  });
  pyodideScriptPromise = tracked;
  return pyodideScriptPromise;
}

export async function ensurePyodideRuntime(): Promise<PyodideInterface> {
  if (typeof window === "undefined") {
    throw new Error("Pyodide can only initialize in the browser.");
  }

  await loadPyodideBrowserScript();

  if (typeof window.loadPyodide !== "function") {
    throw new Error("Pyodide loader did not become available.");
  }

  if (!window.__kyuubikiPyodidePromise) {
    const load = window.loadPyodide;
    // Share even synchronous loader failures, but never cache a rejected initialization.
    const attempt = Promise.resolve().then(() => load({ indexURL: PYODIDE_INDEX_URL })).then((runtime) => {
      if (!runtime || typeof runtime.runPythonAsync !== "function") throw new Error("Invalid Pyodide runtime.");
      return runtime;
    });
    const tracked = attempt.catch((error) => {
      if (window.__kyuubikiPyodidePromise === tracked) delete window.__kyuubikiPyodidePromise;
      throw error;
    });
    window.__kyuubikiPyodidePromise = tracked;
  }

  return window.__kyuubikiPyodidePromise;
}
