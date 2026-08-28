import test from "node:test";
import assert from "node:assert/strict";

import {
  FAVORITE_OPERATOR_STORAGE_KEY,
  RECENT_OPERATOR_STORAGE_KEY,
  persistFavoriteWorkflowOperatorIds,
  persistRecentWorkflowOperatorIds,
  prependRecentWorkflowOperatorId,
  readFavoriteWorkflowOperatorIds,
  readRecentWorkflowOperatorIds,
  toggleFavoriteWorkflowOperatorId,
} from "../../src/components/workbench/workflow/workbench-workflow-operator-preference-storage.ts";

function installStorageHarness() {
  const values = new Map<string, string>();
  const previousWindow = globalThis.window;
  let failWrites = false;
  globalThis.window = {
    localStorage: {
      getItem: (key: string) => values.get(key) ?? null,
      setItem(key: string, value: string) {
        if (failWrites) throw new Error("storage write failed");
        values.set(key, value);
      },
    },
  } as unknown as Window & typeof globalThis;
  return {
    values,
    failWrites: () => {
      failWrites = true;
    },
    restore: () => {
      globalThis.window = previousWindow;
    },
  };
}

test("operator preference reads distinguish missing and corrupt stores", () => {
  const harness = installStorageHarness();
  try {
    assert.deepEqual(readRecentWorkflowOperatorIds(), { operatorIds: [], readable: true });
    harness.values.set(RECENT_OPERATOR_STORAGE_KEY, "");
    assert.deepEqual(readRecentWorkflowOperatorIds(), { operatorIds: [], readable: false });

    harness.values.set(FAVORITE_OPERATOR_STORAGE_KEY, JSON.stringify(["solve.bar_1d", 42]));
    assert.deepEqual(readFavoriteWorkflowOperatorIds(), {
      operatorIds: ["solve.bar_1d"],
      readable: false,
    });
  } finally {
    harness.restore();
  }
});

test("operator preference mutations deduplicate and enforce their retention limits", () => {
  const recent = Array.from({ length: 12 }, (_, index) => `solve.recent_${index}`);
  const nextRecent = prependRecentWorkflowOperatorId(recent, "solve.recent_5");
  assert.equal(nextRecent.length, 12);
  assert.equal(nextRecent[0], "solve.recent_5");
  assert.equal(new Set(nextRecent).size, nextRecent.length);

  let favorites: string[] = [];
  for (let index = 0; index < 20; index += 1) {
    favorites = toggleFavoriteWorkflowOperatorId(favorites, `solve.favorite_${index}`);
  }
  assert.equal(favorites.length, 16);
  assert.equal(favorites[0], "solve.favorite_19");
});

test("operator preference persistence reports write failure and preserves prior data", () => {
  const harness = installStorageHarness();
  try {
    assert.equal(persistRecentWorkflowOperatorIds(["solve.bar_1d"]), true);
    assert.equal(persistFavoriteWorkflowOperatorIds(["solve.heat_bar_1d"]), true);
    const priorRecent = harness.values.get(RECENT_OPERATOR_STORAGE_KEY);
    const priorFavorites = harness.values.get(FAVORITE_OPERATOR_STORAGE_KEY);

    harness.failWrites();
    assert.equal(persistRecentWorkflowOperatorIds(["solve.frame_2d"]), false);
    assert.equal(persistFavoriteWorkflowOperatorIds(["solve.thermal_bar_1d"]), false);
    assert.equal(harness.values.get(RECENT_OPERATOR_STORAGE_KEY), priorRecent);
    assert.equal(harness.values.get(FAVORITE_OPERATOR_STORAGE_KEY), priorFavorites);
  } finally {
    harness.restore();
  }
});
