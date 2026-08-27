import test from "node:test";
import assert from "node:assert/strict";

import {
  createAssistantTransactionEntry,
  pushHistoryEntry,
  type WorkbenchSnapshot,
} from "../../src/lib/workbench/history.ts";

function snapshot(marker: number): WorkbenchSnapshot {
  return {
    memberDraftNodes: [marker],
  } as unknown as WorkbenchSnapshot;
}

test("history capacity one retains only the newest entry", () => {
  const initial = [
    { label: "first", snapshot: snapshot(1) },
    { label: "second", snapshot: snapshot(2) },
  ];

  const next = pushHistoryEntry(initial, "third", snapshot(3), 1);

  assert.equal(next.length, 1);
  assert.equal(next[0]?.label, "third");
});

test("assistant transaction IDs remain unique inside one millisecond", () => {
  const originalNow = Date.now;
  Date.now = () => 1_700_000_000_000;
  try {
    const first = createAssistantTransactionEntry("first", ["study/run"], snapshot(1));
    const second = createAssistantTransactionEntry("second", ["study/run"], snapshot(2));

    assert.notEqual(first.id, second.id);
  } finally {
    Date.now = originalNow;
  }
});

test("assistant transaction audit actions are isolated from caller mutation", () => {
  const actions = ["study/run"];
  const entry = createAssistantTransactionEntry("run", actions, snapshot(1));

  actions.push("project/delete");

  assert.deepEqual(entry.executedActions, ["study/run"]);
});
