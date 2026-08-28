import test from "node:test";
import assert from "node:assert/strict";

import {
  safeWorkbenchPanelStorageGetResult,
  writeWorkbenchPanelStorage,
} from "../../src/components/workbench/workbench-script-panel-storage.ts";

function installStorageHarness() {
  const local = new Map<string, string>();
  const session = new Map<string, string>();
  const previousWindow = globalThis.window;
  let failSessionWrites = false;
  globalThis.window = {
    localStorage: {
      getItem: (key: string) => local.get(key) ?? null,
      setItem: (key: string, value: string) => local.set(key, value),
      removeItem: (key: string) => local.delete(key),
    },
    sessionStorage: {
      getItem: (key: string) => session.get(key) ?? null,
      setItem(key: string, value: string) {
        if (failSessionWrites) throw new Error("session storage write failed");
        session.set(key, value);
      },
      removeItem: (key: string) => session.delete(key),
    },
  } as unknown as Window & typeof globalThis;
  return {
    local,
    session,
    failSessionWrites: () => {
      failSessionWrites = true;
    },
    restore: () => {
      globalThis.window = previousWindow;
    },
  };
}

test("script panel storage migrates a valid legacy draft into session scope", () => {
  const harness = installStorageHarness();
  const key = "script-draft";
  harness.local.set(key, JSON.stringify({ code: "print('legacy')" }));
  try {
    assert.deepEqual(safeWorkbenchPanelStorageGetResult(key), {
      value: { code: "print('legacy')" },
      readable: true,
    });
    assert.equal(harness.local.has(key), false);
    assert.equal(harness.session.get(key), JSON.stringify({ code: "print('legacy')" }));
  } finally {
    harness.restore();
  }
});

test("script panel storage reports corrupt drafts without overwriting them", () => {
  const harness = installStorageHarness();
  const key = "script-draft";
  harness.session.set(key, "");
  try {
    assert.deepEqual(safeWorkbenchPanelStorageGetResult(key), {
      value: null,
      readable: false,
    });
    assert.equal(harness.session.get(key), "");
  } finally {
    harness.restore();
  }
});

test("script panel storage reports write failure without throwing or deleting legacy data", () => {
  const harness = installStorageHarness();
  const key = "script-draft";
  harness.local.set(key, JSON.stringify({ code: "legacy" }));
  harness.failSessionWrites();
  try {
    assert.equal(writeWorkbenchPanelStorage(key, "replacement"), false);
    assert.equal(harness.local.get(key), JSON.stringify({ code: "legacy" }));
  } finally {
    harness.restore();
  }
});
