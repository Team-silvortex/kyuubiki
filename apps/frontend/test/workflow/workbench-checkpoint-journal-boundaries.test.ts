import test from "node:test";
import assert from "node:assert/strict";
import { createCheckpointJournal } from "@/lib/workbench/checkpoint-journal";

const key = "a".repeat(64), id = "checkpoint-original-0001";
test("retired checkpoint cleanup cannot open IndexedDB", async () => {
  let opens = 0;
  const journal = createCheckpointJournal(() => { opens++; throw new Error("must not open"); });
  await assert.rejects(journal.remove(key, id, () => { throw new Error("caller retired"); }), /caller retired/);
  assert.equal(opens, 0);
});

test("checkpoint cleanup rechecks after database open and closes without a transaction", async () => {
  let current = true, closes = 0, transactions = 0;
  const request = { onsuccess: null as (() => void) | null, result: {
    close() { closes++; }, transaction() { transactions++; throw new Error("must not transact"); },
  } };
  const journal = createCheckpointJournal(() => ({ open: () => request }) as unknown as IDBFactory);
  const pending = journal.remove(key, id, () => { if (!current) throw new Error("caller retired"); });
  current = false; request.onsuccess!();
  await assert.rejects(pending, /caller retired/);
  assert.equal(closes, 1); assert.equal(transactions, 0);
});
