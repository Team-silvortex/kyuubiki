import assert from "node:assert/strict";
import test from "node:test";
import { selectStoreEntries, storeEntryIdentity, storeProjectEntryState } from "../../src/lib/workbench/store-catalog.ts";
import type { AssetStoreEntry } from "../../src/lib/api/runtime-types.ts";

const entry: AssetStoreEntry = {
  id: "solve.bar", kind: "operator", title: "Bar", source_id: "lab", source_kind: "catalog_file",
  version: "1.0.0", domain: "mechanical", tags: [],
  install: { mode: "source_reference", requires_download: true, target: "agent" },
};
const staged = { ...entry, target: "agent", installed_at: "2026-11-04T00:00:00Z" };

test("catalog identities separate source, kind and version without separator collisions", () => {
  assert.notEqual(storeEntryIdentity(entry), storeEntryIdentity({ ...entry, source_id: "other" }));
  assert.notEqual(storeEntryIdentity(entry), storeEntryIdentity({ ...entry, version: "2" }));
  assert.notEqual(storeEntryIdentity({ ...entry, source_id: "a:b", id: "c" }), storeEntryIdentity({ ...entry, source_id: "a", id: "b:c" }));
});
test("project state is a reference, and never silently replaces another source or version", () => {
  assert.equal(storeProjectEntryState(entry, []), "available");
  assert.equal(storeProjectEntryState(entry, [staged]), "staged");
  for (const change of [{ source_id: "other" }, { version: "2" }, { package_ref: "different" }, { target: "different" }]) {
    assert.equal(storeProjectEntryState(entry, [{ ...staged, ...change }]), "conflict");
  }
});
test("source and domain filters compose without mutating the catalog", () => {
  const entries = [entry, { ...entry, id: "heat", domain: "thermal" }, { ...entry, source_id: "other" }];
  assert.deepEqual(selectStoreEntries(entries, "lab", "mechanical"), [entry]);
  assert.equal(selectStoreEntries(entries, "", "mechanical").length, 2);
  assert.equal(selectStoreEntries(entries, "missing", "").length, 0);
  assert.equal(entries.length, 3);
});
