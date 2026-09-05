import test from "node:test";
import assert from "node:assert/strict";

import type { AssetStoreEntry } from "../../src/lib/api/runtime-types.ts";
import {
  addManifestEntry,
  blankWorkspaceStoreManifest,
  manifestForSelectedProject,
  normalizeWorkspaceStoreManifest,
  persistWorkspaceStoreManifest,
  readWorkspaceStoreManifestResult,
  STORE_MANIFEST_ENTRY_LIMIT,
  STORE_MANIFEST_PROJECT_LIMIT,
  STORE_MANIFEST_SCHEMA_VERSION,
  STORE_MANIFEST_STORAGE_KEY,
} from "../../src/lib/workbench/store-manifest.ts";

class MemoryStorage implements Storage {
  private readonly records = new Map<string, string>();
  failReads = false;
  failWrites = false;

  get length() {
    return this.records.size;
  }

  clear() {
    this.records.clear();
  }

  getItem(key: string) {
    if (this.failReads) throw new Error("storage read failed");
    return this.records.get(key) ?? null;
  }

  key(index: number) {
    return [...this.records.keys()][index] ?? null;
  }

  removeItem(key: string) {
    this.records.delete(key);
  }

  setItem(key: string, value: string) {
    if (this.failWrites) throw new Error("storage write failed");
    this.records.set(key, String(value));
  }
}

const storage = new MemoryStorage();
const originalWindowDescriptor = Object.getOwnPropertyDescriptor(globalThis, "window");
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: { localStorage: storage } as unknown as Window,
});

test.beforeEach(() => {
  storage.failReads = false;
  storage.failWrites = false;
  storage.clear();
});

test.after(() => {
  if (originalWindowDescriptor) Object.defineProperty(globalThis, "window", originalWindowDescriptor);
  else Reflect.deleteProperty(globalThis, "window");
});

function storeEntry(id: string, title = `Asset ${id}`): AssetStoreEntry {
  return {
    id,
    kind: "operator",
    title,
    version: "1.0.0",
    source_id: "builtin",
    source_kind: "builtin",
    tags: ["test"],
    install: {
      mode: "stage",
      requires_download: false,
      target: `operators/${id}`,
    },
  };
}

test("store manifests persist independently and reject stale project state", () => {
  const projectA = addManifestEntry(blankWorkspaceStoreManifest("project-a"), storeEntry("solve-a"));
  const projectB = addManifestEntry(blankWorkspaceStoreManifest("project-b"), storeEntry("solve-b"));
  assert.ok(projectA);
  assert.ok(projectB);
  assert.equal(persistWorkspaceStoreManifest(projectA), true);
  assert.equal(persistWorkspaceStoreManifest(projectB), true);

  const restoredA = readWorkspaceStoreManifestResult("project-a");
  const restoredB = readWorkspaceStoreManifestResult("project-b");
  assert.equal(restoredA.readable, true);
  assert.equal(restoredA.manifest.entries[0]?.id, "solve-a");
  assert.equal(restoredB.manifest.entries[0]?.id, "solve-b");

  const aligned = manifestForSelectedProject(restoredA.manifest, "project-b");
  assert.equal(aligned.project_id, "project-b");
  assert.deepEqual(aligned.entries, []);
});

test("manifest normalization keeps the newest unique valid bounded entries", () => {
  const entries = Array.from({ length: STORE_MANIFEST_ENTRY_LIMIT + 2 }, (_, index) => ({
    id: `operator-${index}`,
    kind: "operator",
    title: `Operator ${index}`,
    source_id: "catalog",
    installed_at: new Date(Date.UTC(2026, 0, 1, 0, index)).toISOString(),
  }));
  const normalized = normalizeWorkspaceStoreManifest(
    {
      schema_version: STORE_MANIFEST_SCHEMA_VERSION,
      project_id: "wrong-project",
      updated_at: "2026-08-28T00:00:00.000Z",
      entries: [
        ...entries,
        { ...entries.at(-1), title: "Newest duplicate" },
        { id: "", kind: "operator", title: "Invalid", source_id: "catalog" },
      ],
    },
    "project-a",
  );

  assert.equal(normalized.project_id, "project-a");
  assert.equal(normalized.entries.length, STORE_MANIFEST_ENTRY_LIMIT);
  assert.equal(new Set(normalized.entries.map((entry) => entry.id)).size, STORE_MANIFEST_ENTRY_LIMIT);
  assert.equal(normalized.entries.at(-1)?.title, "Newest duplicate");
  assert.equal(normalized.entries.some((entry) => entry.id === "operator-0"), false);
});

test("manifest writes never overwrite unreadable or unsupported storage", () => {
  const candidate = addManifestEntry(blankWorkspaceStoreManifest("project-a"), storeEntry("solve-a"));
  assert.ok(candidate);

  storage.setItem(STORE_MANIFEST_STORAGE_KEY, "{broken-json");
  const brokenRaw = storage.getItem(STORE_MANIFEST_STORAGE_KEY);
  assert.equal(readWorkspaceStoreManifestResult("project-a").readable, false);
  assert.equal(persistWorkspaceStoreManifest(candidate), false);
  assert.equal(storage.getItem(STORE_MANIFEST_STORAGE_KEY), brokenRaw);

  storage.setItem(STORE_MANIFEST_STORAGE_KEY, JSON.stringify({
    "project-a": { schema_version: "kyuubiki.workspace-store-manifest/v999", entries: [] },
  }));
  const futureRaw = storage.getItem(STORE_MANIFEST_STORAGE_KEY);
  assert.equal(persistWorkspaceStoreManifest(candidate), false);
  assert.equal(storage.getItem(STORE_MANIFEST_STORAGE_KEY), futureRaw);
});

test("manifest mutations report storage failures without optimistic success", () => {
  const candidate = addManifestEntry(blankWorkspaceStoreManifest("project-a"), storeEntry("solve-a"));
  assert.ok(candidate);
  storage.failWrites = true;
  assert.equal(persistWorkspaceStoreManifest(candidate), false);
  storage.failWrites = false;
  assert.equal(storage.getItem(STORE_MANIFEST_STORAGE_KEY), null);

  storage.failReads = true;
  assert.equal(readWorkspaceStoreManifestResult("project-a").readable, false);
  assert.equal(persistWorkspaceStoreManifest(candidate), false);
});

test("manifest collection retention is bounded by newest project updates", () => {
  const manifests = Object.fromEntries(
    Array.from({ length: STORE_MANIFEST_PROJECT_LIMIT }, (_, index) => {
      const projectId = `project-${index}`;
      return [projectId, {
        ...blankWorkspaceStoreManifest(projectId),
        updated_at: new Date(Date.UTC(2026, 0, 1, 0, index)).toISOString(),
      }];
    }),
  );
  storage.setItem(STORE_MANIFEST_STORAGE_KEY, JSON.stringify(manifests));

  const newest = addManifestEntry(blankWorkspaceStoreManifest("project-new"), storeEntry("solve-new"));
  assert.ok(newest);
  assert.equal(persistWorkspaceStoreManifest(newest), true);
  const persisted = JSON.parse(storage.getItem(STORE_MANIFEST_STORAGE_KEY) ?? "{}") as Record<string, unknown>;
  assert.equal(Object.keys(persisted).length, STORE_MANIFEST_PROJECT_LIMIT);
  assert.equal("project-new" in persisted, true);
  assert.equal("project-0" in persisted, false);
});

test("invalid API asset entries cannot enter a project manifest", () => {
  const invalid = { ...storeEntry("invalid"), id: " " };
  assert.equal(addManifestEntry(blankWorkspaceStoreManifest("project-a"), invalid), null);
});
