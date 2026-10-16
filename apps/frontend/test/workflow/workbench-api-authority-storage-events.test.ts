import test from "node:test";
import assert from "node:assert/strict";
import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import { WORKBENCH_API_BASE_URL_STORAGE_KEY } from "@/lib/api/backend-target";
import { readInMemoryWorkbenchSecrets, writeInMemoryWorkbenchSecrets } from "@/lib/workbench/workbench-secrets";

const settingsKey = "kyuubiki-workbench-settings", inspect = "/api/direct-mesh/tasks/inspect";
const direct = JSON.stringify({ frontendRuntimeMode: "direct_mesh_gui" });
const orchestra = JSON.stringify({ frontendRuntimeMode: "orchestrated_gui" });

function fixture(run: (f: {
  events: Set<(event: StorageEvent) => void>;
  entries: Map<string, string>;
  location: { search: string };
  notify: (key: string | null, oldValue: string | null, newValue: string | null, local?: boolean) => void;
}) => void) {
  const previousWindow = globalThis.window;
  const entries = new Map([[settingsKey, direct]]);
  const events = new Set<(event: StorageEvent) => void>();
  const storage = { getItem: (key: string) => entries.get(key) ?? null };
  const sessionStorage = {};
  const location = { search: "" };
  globalThis.window = { localStorage: storage, sessionStorage, location,
    addEventListener: (type: string, listener: (event: StorageEvent) => void) => {
      if (type === "storage") events.add(listener);
    },
  } as unknown as Window & typeof globalThis;
  try {
    writeInMemoryWorkbenchSecrets({});
    run({ events, entries, location, notify: (key, oldValue, newValue, local = true) => {
      // Storage has already returned to its final value before queued events arrive.
      for (const listener of events) listener({ key, oldValue, newValue,
        storageArea: local ? storage : sessionStorage } as StorageEvent);
    } });
  } finally { writeInMemoryWorkbenchSecrets({}); globalThis.window = previousWindow; }
}

test("delivered cross-window mode roundtrip invalidates the old request using event history", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  f.notify(settingsKey, direct, orchestra);
  f.notify(settingsKey, orchestra, direct);
  assert.equal(current(), false);
  assert.equal(captureWorkbenchApiAuthority(inspect)(), true);
}));

test("delivered cross-window gateway roundtrip invalidates the old request using event history", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  f.notify(WORKBENCH_API_BASE_URL_STORAGE_KEY, null, "https://other.invalid");
  f.notify(WORKBENCH_API_BASE_URL_STORAGE_KEY, "https://other.invalid", null);
  assert.equal(current(), false);
  assert.equal(captureWorkbenchApiAuthority(inspect)(), true);
}));

test("cross-window legacy settings target changes invalidate even when the final stored target matches", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  const foreign = JSON.stringify({ frontendRuntimeMode: "direct_mesh_gui", backendApiBaseUrl: "https://other.invalid" });
  f.notify(settingsKey, direct, foreign);
  f.notify(settingsKey, foreign, direct);
  assert.equal(current(), false);
}));

for (const key of [settingsKey, WORKBENCH_API_BASE_URL_STORAGE_KEY]) {
  test(`query-pinned and absolute requests ignore a cross-window target edit in ${key}`, () => fixture(f => {
    f.location.search = "?apiBaseUrl=https%3A%2F%2Ffixed.invalid";
    const query = captureWorkbenchApiAuthority(inspect);
    f.location.search = "";
    const absolute = captureWorkbenchApiAuthority("https://fixed.invalid/api/health");
    f.location.search = "?apiBaseUrl=https%3A%2F%2Ffixed.invalid";
    const from = key === settingsKey ? direct : null;
    const to = key === settingsKey ? JSON.stringify({ frontendRuntimeMode: "direct_mesh_gui", backendApiBaseUrl: "https://other.invalid" }) : "https://other.invalid";
    f.notify(key, from, to);
    assert.equal(query(), true);
    f.location.search = "";
    assert.equal(absolute(), true);
  }));
}

test("equivalent settings and gateway values do not invalidate active requests", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  f.notify(settingsKey, direct, JSON.stringify({ frontendRuntimeMode: "direct_mesh_gui", theme: "linen", language: "zh" }));
  f.notify(WORKBENCH_API_BASE_URL_STORAGE_KEY, "https://same.invalid/", " https://same.invalid//#fragment ");
  assert.equal(current(), true);
}));

test("session storage and unrelated storage keys never change API authority", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  f.notify(settingsKey, direct, orchestra, false);
  f.notify("unrelated", null, "invalid JSON");
  assert.equal(current(), true);
}));

test("storage events never import another window's persisted credential fields", () => fixture(f => {
  writeInMemoryWorkbenchSecrets({ directMeshApiToken: "this-window" });
  const current = captureWorkbenchApiAuthority(inspect);
  f.notify(settingsKey, direct, JSON.stringify({ frontendRuntimeMode: "direct_mesh_gui", directMeshApiToken: "other-window" }));
  f.notify("kyuubiki-workbench-secrets", null, JSON.stringify({ directMeshApiToken: "other-window" }));
  assert.deepEqual(readInMemoryWorkbenchSecrets(), { directMeshApiToken: "this-window" });
  assert.equal(current(), true);
}));

for (const invalid of ["{", "null", "[]", '"string"', JSON.stringify({ frontendRuntimeMode: "foreign" }),
  JSON.stringify({ backendApiBaseUrl: {} }), "x".repeat(65537)]) {
  test(`malformed or oversized settings event fails closed (${invalid.slice(0, 24)})`, () => fixture(f => {
    const current = captureWorkbenchApiAuthority(inspect);
    f.notify(settingsKey, direct, invalid);
    f.notify(settingsKey, invalid, direct);
    assert.equal(current(), false);
  }));
}

test("a cross-window clear revokes old requests without deleting their recovery data", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  f.notify(null, null, null);
  assert.equal(current(), false);
  assert.deepEqual([...f.entries], [[settingsKey, direct]]);
}));

test("oversized gateway events revoke relative requests without parsing an unbounded URL", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  const absolute = captureWorkbenchApiAuthority("https://fixed.invalid/api/health");
  f.notify(WORKBENCH_API_BASE_URL_STORAGE_KEY, null, "x".repeat(65537));
  assert.equal(current(), false);
  assert.equal(absolute(), true);
}));

test("unreadable storage while receiving a relevant event cannot revive the old request after recovery", () => fixture(f => {
  const current = captureWorkbenchApiAuthority(inspect);
  const descriptor = Object.getOwnPropertyDescriptor(window, "localStorage")!;
  Object.defineProperty(window, "localStorage", { configurable: true, get() { throw new Error("unavailable"); } });
  try { f.notify(settingsKey, direct, orchestra); }
  finally { Object.defineProperty(window, "localStorage", descriptor); }
  assert.equal(current(), false);
  assert.equal(captureWorkbenchApiAuthority(inspect)(), true);
}));

test("authority captures install one storage listener per window rather than one per request", () => fixture(f => {
  for (let index = 0; index < 1000; index++) captureWorkbenchApiAuthority(inspect);
  assert.equal(f.events.size, 1);
}));
