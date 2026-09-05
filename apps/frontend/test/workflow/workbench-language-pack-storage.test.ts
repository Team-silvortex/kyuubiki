import test from "node:test";
import assert from "node:assert/strict";

import {
  persistWorkbenchLanguagePacks,
  readWorkbenchLanguagePacksResult,
  WORKBENCH_LANGUAGE_PACKS_KEY,
  type WorkbenchLanguagePack,
} from "../../src/lib/workbench/helpers.ts";

const pack: WorkbenchLanguagePack = {
  schema_version: "kyuubiki.language-pack/v1",
  id: "test-pack",
  language: "en",
  targetSurface: "workbench",
  name: "Test pack",
  version: "2.17.0",
  source: "imported",
  updatedAt: "2026-08-28T00:00:00.000Z",
  overrides: { workflowCatalogTitle: "Workflow catalog" },
};

function installStorageHarness() {
  const values = new Map<string, string>();
  const previousWindow = globalThis.window;
  let failReads = false;
  let failWrites = false;
  globalThis.window = {
    localStorage: {
      getItem(key: string) {
        if (failReads) throw new Error("storage read failed");
        return values.get(key) ?? null;
      },
      setItem(key: string, value: string) {
        if (failWrites) throw new Error("storage write failed");
        values.set(key, value);
      },
      removeItem: (key: string) => values.delete(key),
    },
  } as unknown as Window & typeof globalThis;
  return {
    values,
    failReads: () => {
      failReads = true;
    },
    failWrites: () => {
      failWrites = true;
    },
    restore: () => {
      globalThis.window = previousWindow;
    },
  };
}

test("language pack storage distinguishes an empty collection from corrupt data", () => {
  const harness = installStorageHarness();
  try {
    assert.deepEqual(readWorkbenchLanguagePacksResult(), { packs: [], readable: true });

    harness.values.set(WORKBENCH_LANGUAGE_PACKS_KEY, "");
    assert.deepEqual(readWorkbenchLanguagePacksResult(), { packs: [], readable: false });

    harness.values.set(WORKBENCH_LANGUAGE_PACKS_KEY, "{}");
    assert.deepEqual(readWorkbenchLanguagePacksResult(), { packs: [], readable: false });

    harness.failReads();
    assert.deepEqual(readWorkbenchLanguagePacksResult(), { packs: [], readable: false });
  } finally {
    harness.restore();
  }
});

test("language pack storage preserves valid entries without blessing a partial read", () => {
  const harness = installStorageHarness();
  try {
    harness.values.set(
      WORKBENCH_LANGUAGE_PACKS_KEY,
      JSON.stringify([pack, { id: "broken-pack" }]),
    );
    const result = readWorkbenchLanguagePacksResult();
    assert.equal(result.readable, false);
    assert.deepEqual(result.packs, [pack]);
  } finally {
    harness.restore();
  }
});

test("language pack persistence reports failures without destroying the prior value", () => {
  const harness = installStorageHarness();
  try {
    assert.equal(persistWorkbenchLanguagePacks([pack]), true);
    const prior = harness.values.get(WORKBENCH_LANGUAGE_PACKS_KEY);

    harness.failWrites();
    assert.equal(persistWorkbenchLanguagePacks([{ ...pack, id: "replacement" }]), false);
    assert.equal(harness.values.get(WORKBENCH_LANGUAGE_PACKS_KEY), prior);
  } finally {
    harness.restore();
  }
});
