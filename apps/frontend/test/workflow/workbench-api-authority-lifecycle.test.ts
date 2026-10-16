import test from "node:test";
import assert from "node:assert/strict";
import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";

const inspect = "/api/direct-mesh/tasks/inspect";
function fixture(run: (f: { emit: (type: string) => void; listeners: Map<string, Set<() => void>> }) => void) {
  const previous = globalThis.window, listeners = new Map<string, Set<() => void>>();
  globalThis.window = {
    location: { search: "" },
    localStorage: { getItem: () => null },
    addEventListener(type: string, callback: () => void) {
      if (!listeners.has(type)) listeners.set(type, new Set());
      listeners.get(type)!.add(callback);
    },
  } as unknown as Window & typeof globalThis;
  try { run({ listeners, emit: type => { for (const callback of listeners.get(type) ?? []) callback(); } }); }
  finally { globalThis.window = previous; }
}

for (const url of [inspect, "https://original.invalid/api/v1/results"]) {
  test(`pagehide invalidates ${url} even after pageshow before any guard check`, () => fixture(f => {
    const current = captureWorkbenchApiAuthority(url);
    assert.equal(current(), true);
    f.emit("pagehide"); f.emit("pageshow");
    assert.equal(current(), false);
    assert.equal(captureWorkbenchApiAuthority(url)(), true);
  }));
}

test("requests captured while suspended never become authorized after restoration", () => fixture(f => {
  const original = captureWorkbenchApiAuthority(inspect);
  f.emit("pagehide");
  assert.equal(original(), false);
  const hidden = captureWorkbenchApiAuthority(inspect);
  f.emit("pageshow");
  assert.equal(hidden(), false);
  assert.equal(original(), false);
  assert.equal(captureWorkbenchApiAuthority(inspect)(), true);
}));

test("ordinary backgrounding, cancelled navigation and duplicate pageshow do not revoke authority", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  for (const event of ["visibilitychange", "blur", "beforeunload", "pageshow", "pageshow"]) f.emit(event);
  assert.equal(current(), true);
}));

test("each restored generation fences its own requests without unbounded listeners", () => fixture(f => {
  for (let index = 0; index < 1000; index++) {
    const current = captureWorkbenchApiAuthority(inspect);
    f.emit("pagehide"); f.emit("pageshow");
    assert.equal(current(), false);
  }
  assert.equal(f.listeners.get("pagehide")?.size, 1);
  assert.equal(f.listeners.get("pageshow")?.size, 1);
  assert.equal(f.listeners.get("storage")?.size, 1);
}));

test("replacement window with identical settings cannot inherit an earlier capture", () => fixture(() => {
  const current = captureWorkbenchApiAuthority(inspect), original = window;
  fixture(() => assert.equal(current(), false));
  assert.equal(window, original);
  assert.equal(current(), false, "observed invalidity stays latched");
}));

test("server-side authority captures do not require a browser lifecycle", () => {
  const previous = globalThis.window;
  Reflect.deleteProperty(globalThis, "window");
  try { assert.equal(captureWorkbenchApiAuthority(inspect)(), true); }
  finally { if (previous !== undefined) globalThis.window = previous; }
});
