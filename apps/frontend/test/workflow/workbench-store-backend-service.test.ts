import assert from "node:assert/strict";
import test from "node:test";
import type { AssetStoreEntry, AssetStorePayload } from "../../src/lib/api/runtime-types.ts";
import { createWorkbenchStoreBackendService } from "../../src/lib/workbench/store-backend-service-core.ts";

const entry: AssetStoreEntry = {
  id: "solve.bar", kind: "operator", title: "Bar", source_id: "lab", source_kind: "catalog_file",
  version: "1.0.0", package_ref: "operator://bar/1", tags: [],
  install: { mode: "source_reference", requires_download: true, target: "macos-aarch64" },
};

function harness(entries: AssetStoreEntry[], count = entries.length) {
  const queries: unknown[] = [];
  let legacyCalls = 0;
  const service = createWorkbenchStoreBackendService({
    fetchCatalog: async (query) => {
      queries.push(query);
      return { entries, sources: [], summary: { entry_count: count, kinds: {}, sources: {} } };
    },
    // The old endpoint intentionally returns the wrong source.
    ...{ fetchEntry: async () => { legacyCalls++; return { entry: { ...entry, source_id: "builtin.local" } }; } },
  });
  return { service, queries, legacyCalls: () => legacyCalls };
}

test("Store lookup rejects an ambiguous source instead of taking the first detail", async () => {
  const h = harness([entry, { ...entry, source_id: "builtin.local" }]);
  await assert.rejects(h.service.fetchEntry("operator", entry.id), { code: "store_entry_ambiguous" });
  assert.equal(h.legacyCalls(), 0);
});

test("Store lookup pins source, version, package reference and target locally", async () => {
  const h = harness([
    { ...entry, source_id: "other" }, { ...entry, version: "2.0.0" },
    { ...entry, package_ref: "operator://other" },
    { ...entry, install: { ...entry.install, target: "linux-x86_64" } }, entry,
    { ...entry, id: "solve.bar.other" }, { ...entry, kind: "workflow_template" },
  ]);
  const selected = await h.service.fetchEntry("operator", entry.id, {
    sourceId: " lab ", version: "1.0.0", packageRef: "operator://bar/1", target: "macos-aarch64",
  });
  assert.deepEqual(selected, entry);
  assert.deepEqual(h.queries, [{ kind: "operator", q: entry.id, source_id: "lab" }]);
  assert.equal(h.legacyCalls(), 0);
});

test("Store lookup requires a unique match even within a source or for duplicate rows", async () => {
  for (const other of [{ ...entry, version: "2.0.0" }, { ...entry }]) {
    const h = harness([entry, other]);
    await assert.rejects(h.service.fetchEntry("operator", entry.id, { sourceId: "lab" }),
      { code: "store_entry_ambiguous" });
  }
});

test("Store lookup does not fall back when an explicit identity is missing", async () => {
  for (const selection of [{ sourceId: "missing" }, { version: "missing" }, { target: "missing" }, { packageRef: "missing" }]) {
    const h = harness([entry]);
    await assert.rejects(h.service.fetchEntry("operator", entry.id, selection), { code: "store_entry_not_found" });
    assert.equal(h.legacyCalls(), 0);
  }
});

test("Store lookup keeps the unique shorthand and normalizes bounded selectors", async () => {
  const h = harness([entry]);
  assert.deepEqual(await h.service.fetchEntry("operator", entry.id), entry);
  assert.deepEqual(await h.service.fetchEntry("operator", ` ${entry.id} `), entry);
});

test("Store lookup rejects malformed selectors before making a request", async () => {
  for (const selection of [{ sourceId: " " }, { sourceId: null }, { version: 1 }, { target: [] },
    { packageRef: "x".repeat(2049) }, { sourceId: "x".repeat(257) }, { source_id: "lab" }, null, []]) {
    const h = harness([entry]);
    await assert.rejects(h.service.fetchEntry("operator", entry.id, selection as never),
      { code: "store_entry_invalid_selection" });
    assert.equal(h.queries.length, 0);
    assert.equal(h.legacyCalls(), 0);
  }
});

test("Store lookup refuses a partial or malformed catalog rather than claiming uniqueness", async () => {
  await assert.rejects(harness([entry], 2).service.fetchEntry("operator", entry.id), { code: "store_catalog_invalid" });
  const service = createWorkbenchStoreBackendService({
    fetchCatalog: async () => ({ entries: null } as unknown as AssetStorePayload),
    ...{ fetchEntry: async () => ({ entry }) },
  });
  await assert.rejects(service.fetchEntry("operator", entry.id), { code: "store_catalog_invalid" });
});

test("Store lookup rejects a malformed matching entry", async () => {
  const h = harness([{ ...entry, title: " " }]);
  await assert.rejects(h.service.fetchEntry("operator", entry.id), { code: "store_entry_invalid" });
});

test("Store lookup propagates failure and can retry without stale detail fallback", async () => {
  let failed = true;
  const service = createWorkbenchStoreBackendService({
    fetchCatalog: async () => {
      if (failed) throw new Error("catalog unavailable");
      return { entries: [entry], sources: [], summary: { entry_count: 1, kinds: {}, sources: {} } };
    },
    ...{ fetchEntry: async () => ({ entry }) },
  });
  await assert.rejects(service.fetchEntry("operator", entry.id), /catalog unavailable/);
  failed = false;
  assert.deepEqual(await service.fetchEntry("operator", entry.id), entry);
});
