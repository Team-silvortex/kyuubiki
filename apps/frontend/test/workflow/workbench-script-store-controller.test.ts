import assert from "node:assert/strict";
import test from "node:test";

import { handleWorkbenchScriptStoreAction } from "../../src/components/workbench/workbench-script-store-controller.ts";
import type { WorkbenchStoreBackendService } from "../../src/lib/workbench/store-backend-service-core.ts";

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

function harness(overrides: Partial<WorkbenchStoreBackendService> = {}) {
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
      selectedProjectId: "project-a",
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
