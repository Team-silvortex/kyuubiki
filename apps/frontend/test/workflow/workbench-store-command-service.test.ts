import assert from "node:assert/strict";
import test from "node:test";

import type { AssetStoreEntry } from "../../src/lib/api/runtime-types.ts";
import {
  buildWorkspaceStoreManifestExport,
  removeWorkspaceStoreEntry,
  stageWorkspaceStoreEntry,
  WorkspaceStoreCommandError,
} from "../../src/lib/workbench/store-command-service.ts";
import { STORE_MANIFEST_STORAGE_KEY } from "../../src/lib/workbench/store-manifest.ts";

class MemoryStorage implements Storage {
  private readonly records = new Map<string, string>();
  failWrites = false;
  get length() { return this.records.size; }
  clear() { this.records.clear(); }
  getItem(key: string) { return this.records.get(key) ?? null; }
  key(index: number) { return [...this.records.keys()][index] ?? null; }
  removeItem(key: string) { this.records.delete(key); }
  setItem(key: string, value: string) {
    if (this.failWrites) throw new Error("write failed");
    this.records.set(key, value);
  }
}

const storage = new MemoryStorage();
const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: { localStorage: storage, dispatchEvent: () => true } as unknown as Window,
});

test.beforeEach(() => {
  storage.clear();
  storage.failWrites = false;
});

test.after(() => {
  if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
  else Reflect.deleteProperty(globalThis, "window");
});

function storeEntry(id = "solve.bar_1d"): AssetStoreEntry {
  return {
    id,
    kind: "operator",
    title: "Bar solver",
    version: "2.17.0",
    source_id: "builtin",
    source_kind: "builtin",
    tags: ["mechanical"],
    install: { mode: "stage", requires_download: false, target: `operators/${id}` },
  };
}

test("shared Store commands stage, export, and remove one project asset", () => {
  const staged = stageWorkspaceStoreEntry("project / unsafe", storeEntry());
  assert.equal(staged.entries.length, 1);

  const exported = buildWorkspaceStoreManifestExport("project / unsafe");
  assert.equal(exported.filename, "project-unsafe.store-manifest.json");
  assert.equal(JSON.parse(exported.contents).entries[0].id, "solve.bar_1d");

  const removed = removeWorkspaceStoreEntry("project / unsafe", "operator", "solve.bar_1d");
  assert.equal(removed.entries.length, 0);
  assert.ok(storage.getItem(STORE_MANIFEST_STORAGE_KEY));
});

test("shared Store commands fail closed for missing projects, entries, and writes", () => {
  assert.throws(
    () => stageWorkspaceStoreEntry(null, storeEntry()),
    (error) => error instanceof WorkspaceStoreCommandError && error.code === "project_required",
  );
  assert.throws(
    () => removeWorkspaceStoreEntry("project-a", "operator", "missing"),
    (error) => error instanceof WorkspaceStoreCommandError && error.code === "entry_missing",
  );
  storage.failWrites = true;
  assert.throws(
    () => stageWorkspaceStoreEntry("project-a", storeEntry()),
    (error) => error instanceof WorkspaceStoreCommandError && error.code === "manifest_write_failed",
  );
});
