import test from "node:test";
import assert from "node:assert/strict";
import { createBrowserDirectExecutionJournal } from "@/lib/workbench/direct-execution-journal";

const intent = { scope: "a".repeat(64), endpoint: "agent:5001", study_kind: "axial_bar_1d",
  execution_id: "b".repeat(32), store_id: "c".repeat(32), created_at: 1 };

test("expired browser cleanup does not even open IndexedDB", async () => {
  let opens = 0;
  const journal = createBrowserDirectExecutionJournal(() => { opens++; throw new Error("must not open"); });
  await assert.rejects(journal.remove(intent, () => { throw new Error("caller retired"); }), /caller retired/);
  assert.equal(opens, 0);
});

test("browser cleanup rechecks ownership after database open and closes without a transaction", async () => {
  let current = true, closes = 0, transactions = 0;
  const request = { onsuccess: null as (() => void) | null, result: {
    close() { closes++; }, transaction() { transactions++; throw new Error("must not transact"); },
  } };
  const journal = createBrowserDirectExecutionJournal(() => ({ open: () => request }) as unknown as IDBFactory);
  const pending = journal.remove(intent, () => { if (!current) throw new Error("caller retired"); });
  current = false; request.onsuccess!();
  await assert.rejects(pending, /caller retired/);
  assert.equal(closes, 1); assert.equal(transactions, 0);
});
