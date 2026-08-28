import test from "node:test";
import assert from "node:assert/strict";

import {
  FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY,
  FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY,
  readWorkflowTemplateChainPreferences,
  writeWorkflowTemplateChainPreferences,
} from "../../src/components/workbench/workflow/workbench-workflow-template-chain-storage.ts";

function createMemoryStorage(): Storage {
  const records = new Map<string, string>();
  return {
    get length() {
      return records.size;
    },
    clear: () => records.clear(),
    getItem: (key) => records.get(key) ?? null,
    key: (index) => [...records.keys()][index] ?? null,
    removeItem: (key) => records.delete(key),
    setItem: (key, value) => records.set(key, String(value)),
  };
}

const originalWindowDescriptor = Object.getOwnPropertyDescriptor(globalThis, "window");
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: { localStorage: createMemoryStorage() } as unknown as Window,
});

test.after(() => {
  if (originalWindowDescriptor) Object.defineProperty(globalThis, "window", originalWindowDescriptor);
  else Reflect.deleteProperty(globalThis, "window");
});

test("template chain preferences deduplicate, bound, and align aliases", () => {
  const ids = Array.from({ length: 15 }, (_, index) => `chain-${index}`);
  assert.equal(writeWorkflowTemplateChainPreferences({
    favoriteChainIds: [ids[0]!, "", ...ids, ids[0]!],
    favoriteChainAliases: {
      [ids[0]!]: "Primary",
      [ids[12]!]: "Outside limit",
      stale: "Stale alias",
      [ids[1]!]: "",
    },
  }), true);

  const stored = readWorkflowTemplateChainPreferences();
  assert.equal(stored.favoriteChainIds.length, 12);
  assert.equal(new Set(stored.favoriteChainIds).size, 12);
  assert.deepEqual(stored.favoriteChainAliases, { [ids[0]!]: "Primary" });
});

test("template chain preference writes report storage failure without throwing", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  browserWindow.localStorage = {
    ...createMemoryStorage(),
    getItem: (key) => {
      if (key === FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY) return JSON.stringify(["chain-a"]);
      if (key === FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY) return JSON.stringify({ "chain-a": "A" });
      return null;
    },
    setItem: () => {
      throw new Error("quota exceeded");
    },
  } as Storage;

  try {
    assert.equal(writeWorkflowTemplateChainPreferences({
      favoriteChainIds: ["chain-a"],
      favoriteChainAliases: { "chain-a": "A" },
    }), false);
    assert.deepEqual(readWorkflowTemplateChainPreferences(), {
      favoriteChainIds: ["chain-a"],
      favoriteChainAliases: { "chain-a": "A" },
    });
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});

test("template chain preference writes roll back when the alias write fails", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const storage = createMemoryStorage();
  const previousIds = JSON.stringify(["chain-old"]);
  const previousAliases = JSON.stringify({ "chain-old": "Old" });
  storage.setItem(FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY, previousIds);
  storage.setItem(FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY, previousAliases);
  const originalSetItem = storage.setItem.bind(storage);
  storage.setItem = (key, value) => {
    if (
      key === FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY &&
      String(value) !== previousAliases
    ) {
      throw new Error("alias write failed");
    }
    originalSetItem(key, value);
  };
  browserWindow.localStorage = storage;

  try {
    assert.equal(writeWorkflowTemplateChainPreferences({
      favoriteChainIds: ["chain-next"],
      favoriteChainAliases: { "chain-next": "Next" },
    }), false);
    assert.equal(storage.getItem(FAVORITE_TEMPLATE_CHAIN_STORAGE_KEY), previousIds);
    assert.equal(storage.getItem(FAVORITE_TEMPLATE_CHAIN_ALIAS_STORAGE_KEY), previousAliases);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});
