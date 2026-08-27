import test from "node:test";
import assert from "node:assert/strict";

import {
  appendWorkflowActivityLogEntry,
  readWorkflowActivityLog,
} from "../../src/lib/workbench/workflow-activity-log.ts";

const storageKey = "kyuubiki-workflow-activity-log";

function withSessionStorage(
  initial: unknown,
  run: (storage: Map<string, string>) => void,
) {
  const storage = new Map<string, string>();
  if (initial !== undefined) storage.set(storageKey, JSON.stringify(initial));
  const previousWindow = globalThis.window;
  globalThis.window = {
    sessionStorage: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
      removeItem: (key: string) => storage.delete(key),
    },
  } as unknown as Window & typeof globalThis;

  try {
    run(storage);
  } finally {
    globalThis.window = previousWindow;
  }
}

test("workflow activity log rejects unknown persisted event kinds", () => {
  withSessionStorage([
    {
      id: "valid",
      at: "2026-08-26T00:00:00.000Z",
      workflowId: "workflow-a",
      kind: "snapshot_saved",
      message: "saved",
    },
    {
      id: "forged",
      at: "2026-08-26T00:00:01.000Z",
      workflowId: "workflow-a",
      kind: "workflow_root_replaced",
      message: "forged",
    },
  ], () => {
    assert.deepEqual(readWorkflowActivityLog().map((entry) => entry.id), ["valid"]);
  });
});

test("workflow activity entries remain unique inside one millisecond", () => {
  const originalNow = Date.now;
  const originalRandom = Math.random;
  Date.now = () => 1_700_000_000_000;
  Math.random = () => 0.5;

  try {
    withSessionStorage([], () => {
      const first = appendWorkflowActivityLogEntry({
        workflowId: "workflow-a",
        kind: "snapshot_saved",
        message: "first",
      });
      const second = appendWorkflowActivityLogEntry({
        workflowId: "workflow-a",
        kind: "snapshot_saved",
        message: "second",
      });

      assert.notEqual(first?.id, second?.id);
      assert.equal(readWorkflowActivityLog().length, 2);
    });
  } finally {
    Date.now = originalNow;
    Math.random = originalRandom;
  }
});
