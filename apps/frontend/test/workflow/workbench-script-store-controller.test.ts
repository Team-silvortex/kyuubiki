import assert from "node:assert/strict";
import test from "node:test";

import { handleWorkbenchScriptStoreAction } from "../../src/components/workbench/workbench-script-store-controller.ts";
import type { WorkbenchStoreBackendService } from "../../src/lib/workbench/store-backend-service-core.ts";
import { readWorkspaceStoreManifest } from "../../src/lib/workbench/store-manifest.ts";

class MemoryStorage implements Storage {
  private readonly records = new Map<string, string>();
  get length() { return this.records.size; }
  clear() { this.records.clear(); }
  getItem(key: string) { return this.records.get(key) ?? null; }
  key(index: number) { return [...this.records.keys()][index] ?? null; }
  removeItem(key: string) { this.records.delete(key); }
  setItem(key: string, value: string) { this.records.set(key, value); }
}

const storage = new MemoryStorage();
const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: { localStorage: storage, dispatchEvent: () => true } as unknown as Window,
});

test.beforeEach(() => storage.clear());
test.after(() => {
  if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
  else Reflect.deleteProperty(globalThis, "window");
});

function harness(overrides: Partial<WorkbenchStoreBackendService> = {}, selectedProjectId: string | null = "project-a") {
  const messages: string[] = [];
  const downloads: Array<{ filename: string; contents: string }> = [];
  const service: WorkbenchStoreBackendService = {
    fetchCatalog: async () => ({ entries: [], sources: [], summary: { entry_count: 0, kinds: {}, sources: {} } }),
    fetchEntry: async (kind, entryId) => ({
      id: entryId,
      kind,
      title: "Bar solver",
      source_id: "builtin",
      source_kind: "builtin",
      tags: [],
      install: { mode: "stage", requires_download: false },
    }),
    ...overrides,
  };
  const invoke = (action: string, payload: Record<string, unknown> = {}) =>
    handleWorkbenchScriptStoreAction({
      action,
      payload,
      selectedProjectId,
      language: "en",
      setMessage: (message) => messages.push(message),
      storeBackendService: service,
      downloadTextFile: (filename, contents) => downloads.push({ filename, contents }),
    });
  return { downloads, invoke, messages };
}

test("script Store controller closes stage, export, and remove actions", async () => {
  const testHarness = harness();
  const staged = await testHarness.invoke("store/stageEntry", { kind: "operator", entryId: "solve.bar_1d" });
  assert.equal(staged?.manifestEntryCount, 1);

  const exported = await testHarness.invoke("store/exportManifest");
  assert.equal(exported?.manifestEntryCount, 1);
  assert.equal(testHarness.downloads[0]?.filename, "project-a.store-manifest.json");

  const removed = await testHarness.invoke("store/removeEntry", { kind: "operator", entryId: "solve.bar_1d" });
  assert.equal(removed?.manifestEntryCount, 0);
  assert.equal(testHarness.messages.length, 3);
});

test("script Store controller forwards exact selectors and returns the staged identity", async () => {
  const selection = { sourceId: "lab", version: "1.0.0", packageRef: "operator://bar/1", target: "mac" };
  const h = harness({ fetchEntry: async (kind, id, actualSelection) => {
    assert.deepEqual(actualSelection, selection);
    return { id, kind, title: "Bar", source_id: "lab", source_kind: "catalog_file", tags: [],
      version: "1.0.0", package_ref: "operator://bar/1",
      install: { mode: "source_reference", requires_download: true, target: "mac" } };
  } });
  const result = await h.invoke("store/stageEntry", { kind: "operator", entryId: "bar", ...selection });
  for (const [key, value] of Object.entries(selection)) assert.equal(result?.[key], value);
  assert.equal(readWorkspaceStoreManifest("project-a").entries[0].source_id, "lab");
});

test("script Store controller rejects an adapter that ignores requested identity fields", async () => {
  for (const selection of [{ sourceId: "lab" }, { version: "2" }, { packageRef: "operator://other" }, { target: "other" }]) {
    const h = harness();
    await assert.rejects(h.invoke("store/stageEntry", { kind: "operator", entryId: "bar", ...selection }),
      { code: "store_entry_not_found" });
    assert.equal(storage.length, 0);
    assert.deepEqual(h.messages, []);
  }
});

test("script Store controller rejects missing projects and malformed selectors before lookup", async () => {
  let calls = 0;
  const overrides = { fetchEntry: async () => { calls++; throw new Error("unexpected lookup"); } };
  await assert.rejects(harness(overrides, null).invoke("store/stageEntry", { kind: "operator", entryId: "bar" }),
    { code: "project_required" });
  await assert.rejects(harness(overrides).invoke("store/stageEntry", { kind: "operator", entryId: "bar", sourceId: " " }),
    { code: "store_entry_invalid_selection" });
  assert.equal(calls, 0);
  assert.equal(storage.length, 0);
});

test("script Store controller rejects invalid requests and mismatched backend entries", async () => {
  const testHarness = harness({
    fetchEntry: async () => ({
      id: "different",
      kind: "operator",
      title: "Mismatch",
      source_id: "builtin",
      source_kind: "builtin",
      tags: [],
      install: { mode: "stage", requires_download: false },
    }),
  });
  await assert.rejects(
    () => testHarness.invoke("store/stageEntry", { kind: "missing", entryId: "x" }),
    /Invalid kind/u,
  );
  await assert.rejects(
    () => testHarness.invoke("store/stageEntry", { kind: "operator", entryId: "expected" }),
    /response mismatch/u,
  );
});
