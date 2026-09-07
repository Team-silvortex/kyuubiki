import test, { type TestContext } from "node:test";
import assert from "node:assert/strict";

let moduleSequence = 0;
async function browserFixture(t: TestContext) {
  const scripts: FakeScript[] = [];
  class FakeScript extends EventTarget {
    src = "";
    async = false;
    dataset: Record<string, string> = {};
    onload?: () => void;
    onerror?: () => void;
    remove() { const index = scripts.indexOf(this); if (index >= 0) scripts.splice(index, 1); }
    emit(type: "load" | "error") {
      this.dispatchEvent(new Event(type));
      (type === "load" ? this.onload : this.onerror)?.();
    }
  }
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  const previousDocument = Object.getOwnPropertyDescriptor(globalThis, "document");
  const browser: Partial<Window> = {};
  Object.defineProperty(globalThis, "window", { value: browser, configurable: true });
  Object.defineProperty(globalThis, "document", { value: {
    querySelector: () => scripts[0] ?? null,
    createElement: () => new FakeScript(),
    head: { appendChild: (script: FakeScript) => { scripts.push(script); } },
  }, configurable: true });
  t.after(() => {
    for (const [key, descriptor] of [["window", previousWindow], ["document", previousDocument]] as const) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor);
      else Reflect.deleteProperty(globalThis, key);
    }
  });
  const moduleUrl = new URL("../../src/lib/scripting/workbench-script-pyodide-loader.ts", import.meta.url);
  moduleUrl.searchParams.set("case", `${++moduleSequence}.ts`);
  const { ensurePyodideRuntime } = await import(moduleUrl.href) as typeof import("../../src/lib/scripting/workbench-script-pyodide-loader.ts");
  const runtime = { runPythonAsync: async <T>() => undefined as T };
  return { browser, scripts, runtime, ensurePyodideRuntime };
}

for (const failure of ["error", "missing-loader", "timeout"] as const) {
  test(`PWDT runtime retries script ${failure} without reloading the page`, { timeout: 2_000 }, async (t) => {
    const fixture = await browserFixture(t);
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const first = fixture.ensurePyodideRuntime();
    const second = fixture.ensurePyodideRuntime();
    const outcomes = Promise.allSettled([first, second]);
    assert.equal(fixture.scripts.length, 1, "concurrent callers share one script download");
    const failedScript = fixture.scripts[0];
    if (failure === "timeout") t.mock.timers.tick(30_001);
    else failedScript.emit(failure === "error" ? "error" : "load");
    assert.deepEqual((await outcomes).map((entry) => entry.status), ["rejected", "rejected"]);
    assert.equal(fixture.scripts.length, 0, "failed tags must not trap future attempts");
    const retried = fixture.ensurePyodideRuntime();
    assert.equal(fixture.scripts.length, 1);
    fixture.browser.loadPyodide = async () => fixture.runtime;
    fixture.scripts[0].emit("load");
    assert.equal(await retried, fixture.runtime);
    failedScript.emit("error");
    assert.equal(await fixture.ensurePyodideRuntime(), fixture.runtime, "late events cannot clear a newer success");
  });
}

for (const failure of ["reject", "throw", "invalid-runtime"] as const) {
  test(`PWDT runtime retries initialization ${failure} and shares the successful instance`, async (t) => {
    const { browser, runtime, scripts, ensurePyodideRuntime } = await browserFixture(t);
    let calls = 0;
    browser.loadPyodide = () => {
      calls += 1;
      if (calls === 1) {
        if (failure === "throw") throw new Error("initialization failed");
        if (failure === "invalid-runtime") return Promise.resolve({} as typeof runtime);
        return Promise.reject(new Error("initialization failed"));
      }
      return Promise.resolve(runtime);
    };
    const outcomes = await Promise.allSettled([ensurePyodideRuntime(), ensurePyodideRuntime()]);
    assert.deepEqual(outcomes.map((entry) => entry.status), ["rejected", "rejected"]);
    assert.equal(calls, 1);
    assert.equal(browser.__kyuubikiPyodidePromise, undefined);
    assert.equal(await ensurePyodideRuntime(), runtime);
    assert.equal(await ensurePyodideRuntime(), runtime);
    assert.equal(calls, 2);
    assert.equal(scripts.length, 0);
  });
}

test("PWDT runtime adopts an in-flight script without creating a duplicate", async (t) => {
  const { browser, scripts, runtime, ensurePyodideRuntime } = await browserFixture(t);
  const existing = document.createElement("script");
  existing.dataset.pyodide = "true";
  document.head.appendChild(existing);
  const pending = ensurePyodideRuntime();
  assert.equal(scripts.length, 1);
  browser.loadPyodide = async () => runtime;
  scripts[0].emit("load");
  assert.equal(await pending, runtime);
});
