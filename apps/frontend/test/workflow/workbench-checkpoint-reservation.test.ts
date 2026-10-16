import test from "node:test";
import assert from "node:assert/strict";
import { createCheckpointJournal, type CheckpointIntent, type CheckpointJournal } from "../../src/lib/workbench/checkpoint-journal.ts";
import { createCheckpointRetry } from "../../src/lib/workbench/checkpoint-retry.ts";

const intent: CheckpointIntent = { key: "a".repeat(64), scope: "b".repeat(64), operation: "model",
  parentId: "project", requestId: "checkpoint-original-0001", createdAt: 1 };
const input = { payload: { length: 1 }, request_id: undefined as string | undefined };
const changes: Partial<CheckpointIntent>[] = [
  { key: "c".repeat(64) }, { scope: "c".repeat(64) }, { operation: "version" },
  { parentId: "another-project" }, { requestId: "checkpoint-foreign-0002" }, { createdAt: 0 },
];

for (const change of changes) {
  test(`checkpoint retry rejects a mismatched new reservation before transport: ${Object.keys(change)[0]}`, async () => {
    let writes = 0, removals = 0, corrupt = true;
    const journal: CheckpointJournal = {
      list: async () => [], remove: async () => { removals++; },
      reserve: async requested => ({ intent: { ...requested, ...(corrupt ? change : {}) }, isNew: true }),
    };
    const retry = createCheckpointRetry({ journal, limit: 1 });
    const write = async () => { writes++; return "saved"; };
    await assert.rejects(retry("model", "project", input, write), /checkpoint:journal_corrupt/);
    assert.equal(writes, 0); assert.equal(removals, 0);
    corrupt = false;
    assert.equal(await retry("model", "project", input, write), "saved", "failed validation releases only the memory slot");
    assert.equal(writes, 1); assert.equal(removals, 1);
  });
}

test("checkpoint retry validates reservation shape and does not trust an adapter-mutated request", async () => {
  for (const mutate of [false, true]) {
    let writes = 0, removals = 0;
    const journal: CheckpointJournal = {
      list: async () => [], remove: async () => { removals++; },
      async reserve(requested) {
        if (mutate) requested.parentId = "another-project";
        return { intent: requested, isNew: mutate ? true : "true" as unknown as boolean };
      },
    };
    await assert.rejects(createCheckpointRetry({ journal })("model", "project", input, async () => { writes++; }), /journal_corrupt/);
    assert.equal(writes, 0); assert.equal(removals, 0);
  }
});

test("checkpoint retry reuses the original request and timestamp only for an existing reservation", async () => {
  let removed: string | undefined;
  const journal: CheckpointJournal = {
    list: async () => [], remove: async (_key, id) => { removed = id; },
    reserve: async requested => ({ intent: { ...requested, requestId: intent.requestId, createdAt: 1 }, isNew: false }),
  };
  const result = await createCheckpointRetry({ journal })("model", "project", input, async body => body.request_id);
  assert.equal(result, intent.requestId); assert.equal(removed, intent.requestId);
});

// A controlled transaction fixture isolates the open/validation timing. Real
// IndexedDB conflict/abort behavior is covered by the browser journey suite.
function databaseFixture(initial: CheckpointIntent[] = []) {
  let entries = structuredClone(initial), aborted = false;
  const read = { result: structuredClone(entries), onsuccess: null as (() => void) | null };
  const tx = {
    oncomplete: null as (() => void) | null, onabort: null as (() => void) | null, onerror: null,
    abort() { aborted = true; queueMicrotask(() => tx.onabort?.()); },
    objectStore: () => ({
      getAll: () => {
        queueMicrotask(() => {
          read.onsuccess?.();
          queueMicrotask(() => { if (!aborted) tx.oncomplete?.(); });
        });
        return read;
      },
      add: (entry: CheckpointIntent) => { entries.push(structuredClone(entry)); },
    }),
  };
  const request = { onsuccess: null as (() => void) | null, result: { close() {}, transaction: () => tx } };
  const journal = createCheckpointJournal(() => ({ open: () => request }) as unknown as IDBFactory);
  return { journal, open: () => request.onsuccess!(), entries: () => entries, aborted: () => aborted };
}

test("browser checkpoint reservation snapshots metadata before database open", async () => {
  const f = databaseFixture(), submitted = { ...intent };
  const pending = f.journal.reserve(submitted);
  submitted.parentId = "another-project"; submitted.requestId = "invalid";
  f.open();
  const reserved = await pending;
  assert.deepEqual(reserved, { intent, isNew: true });
  assert.notEqual(reserved.intent, submitted);
  assert.deepEqual(f.entries(), [intent]);
});

for (const change of changes.slice(1, 4)) {
  test(`browser checkpoint reservation rejects same-key identity conflicts: ${Object.keys(change)[0]}`, async () => {
    const existing = { ...intent, ...change }, f = databaseFixture([existing]);
    const rejected = assert.rejects(f.journal.reserve(intent), /checkpoint:journal_corrupt/);
    f.open(); await rejected;
    assert.equal(f.aborted(), true); assert.deepEqual(f.entries(), [existing]);
  });
}

test("browser checkpoint reservation keeps the earlier identity for a legitimate retry", async () => {
  const f = databaseFixture([intent]);
  const pending = f.journal.reserve({ ...intent, requestId: "checkpoint-later-0002", createdAt: 2 });
  f.open();
  assert.deepEqual(await pending, { intent, isNew: false });
  assert.deepEqual(f.entries(), [intent]);
});
