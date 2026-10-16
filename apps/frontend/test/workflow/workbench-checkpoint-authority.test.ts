import test from "node:test";
import assert from "node:assert/strict";
import { createCheckpointRecovery } from "../../src/lib/workbench/checkpoint-recovery.ts";
import { createCheckpointRetry } from "../../src/lib/workbench/checkpoint-retry.ts";
import { checkpointDigest, type CheckpointIntent, type CheckpointJournal } from "../../src/lib/workbench/checkpoint-journal.ts";
import type { ModelVersionRecord } from "../../src/lib/api/project-types.ts";

const receipt = { status: "committed", project_id: "project", model_id: "model", version_id: "version" } as const;
const input = { payload: { length: 1 }, request_id: undefined as string | undefined };
async function fixture() {
  let generation = 0;
  const entries = new Map<string, CheckpointIntent>();
  const journal: CheckpointJournal = {
    list: async () => [...entries.values()],
    async reserve(intent) {
      const previous = entries.get(intent.key);
      if (previous) return { intent: previous, isNew: false };
      entries.set(intent.key, intent); return { intent, isNew: true };
    },
    async remove(key, id, check) { check?.(); if (entries.get(key)?.requestId === id) entries.delete(key); },
  };
  const options = { journal, scope: () => "same-scope", captureAuthority: () => {
    const original = generation; return () => original === generation;
  } };
  const intent = { key: "a".repeat(64), scope: await checkpointDigest(options.scope()), operation: "model" as const,
    parentId: "project", requestId: "checkpoint-original-0001", createdAt: 1 };
  return { options, entries, journal, intent, retire: () => { generation++; } };
}

for (const action of ["check", "acknowledge", "open"] as const) {
  test(`checkpoint ${action} rejects a restored authority before publishing or deleting`, async () => {
    const f = await fixture(); f.entries.set(f.intent.key, f.intent);
    const recovery = createCheckpointRecovery({ ...f.options, lookup: async () => {
      f.retire(); return { checkpoint: receipt };
    } });
    let opens = 0;
    const outcome = action === "open" ? recovery.open(f.intent.key, async (_id, validate) => {
      validate(receipt as unknown as ModelVersionRecord); opens++; return { ok: true };
    }) : recovery[action](f.intent.key);
    await assert.rejects(outcome, /context_changed/);
    assert.equal(opens, 0); assert.equal(f.entries.size, 1);
    const fresh = createCheckpointRecovery({ ...f.options, lookup: async () => ({ checkpoint: receipt }) });
    assert.deepEqual(await fresh.acknowledge(f.intent.key), receipt);
    assert.equal(f.entries.size, 0);
  });
}

test("checkpoint open carries its original guard through the version fetch", async () => {
  const f = await fixture(); f.entries.set(f.intent.key, f.intent);
  const recovery = createCheckpointRecovery({ ...f.options, lookup: async () => ({ checkpoint: receipt }) });
  let applied = false;
  await assert.rejects(recovery.open(f.intent.key, async (_id, validate) => {
    f.retire(); validate(receipt as unknown as ModelVersionRecord); applied = true; return { ok: true };
  }), /context_changed/);
  assert.equal(applied, false); assert.equal(f.entries.size, 1);
});

for (const afterCommit of [false, true]) {
  test(`checkpoint acknowledgement retains its owner through journal cleanup; afterCommit=${afterCommit}`, async () => {
    const f = await fixture(); f.entries.set(f.intent.key, f.intent);
    const recovery = createCheckpointRecovery({ ...f.options, lookup: async () => ({ checkpoint: receipt }),
      journal: { ...f.journal, async remove(key, id, check) {
        if (afterCommit) await f.journal.remove(key, id, check);
        f.retire();
        if (!afterCommit) await f.journal.remove(key, id, check);
      } },
    });
    await assert.rejects(recovery.acknowledge(f.intent.key), /context_changed/);
    assert.equal(f.entries.size, afterCommit ? 0 : 1, "a committed deletion is not rolled back");
  });
}

test("checkpoint dispatch is fenced after reservation without erasing shared recovery identity", async () => {
  const f = await fixture();
  const retry = createCheckpointRetry({ ...f.options, journal: { ...f.journal, async reserve(intent) {
    const result = await f.journal.reserve(intent); f.retire(); return result;
  } } });
  let writes = 0;
  await assert.rejects(retry("model", "project", input, async () => { writes++; }), /context_changed/);
  assert.equal(writes, 0); assert.equal(f.entries.size, 1);
});

for (const persistent of [false, true]) {
  test(`late save acknowledgement preserves its original retry key; persistent=${persistent}`, async () => {
    const f = await fixture(), ids: string[] = [];
    const retry = createCheckpointRetry({ ...f.options, journal: persistent ? f.journal : undefined });
    await assert.rejects(retry("model", "project", input, async (body) => {
      ids.push(body.request_id!); f.retire(); return "committed";
    }), /context_changed/);
    assert.equal(f.entries.size, persistent ? 1 : 0);
    assert.equal(await retry("model", "project", input, async body => { ids.push(body.request_id!); return "reconciled"; }), "reconciled");
    assert.equal(ids[0], ids[1]); assert.equal(f.entries.size, 0);
  });

  test(`restored callers cannot replay or adopt an older coalesced save; persistent=${persistent}`, async () => {
    const f = await fixture();
    const retry = createCheckpointRetry({ ...f.options, journal: persistent ? f.journal : undefined });
    let release!: () => void, started!: () => void, writes = 0, original = "";
    const gate = new Promise<void>(resolve => { release = resolve; });
    const writing = new Promise<void>(resolve => { started = resolve; });
    const first = assert.rejects(retry("model", "project", input, async body => {
      writes++; original = body.request_id!; started(); await gate; return "committed";
    }), /context_changed/);
    await writing; f.retire();
    const second = assert.rejects(retry("model", "project", input, async () => { writes++; return "must not replay"; }), /context_changed/);
    // Let the real digest finish so the restored caller joins the existing flight.
    await new Promise(resolve => setTimeout(resolve, 20));
    release(); await Promise.all([first, second]);
    assert.equal(writes, 1);
    assert.equal(await retry("model", "project", input, async body => body.request_id), original);
    assert.equal(f.entries.size, 0);
  });
}

test("automatic save cleanup passes the original guard and keeps a retired reservation", async () => {
  const f = await fixture();
  const retry = createCheckpointRetry({ ...f.options, journal: { ...f.journal, async remove(key, id, check) {
    f.retire(); await f.journal.remove(key, id, check);
  } } });
  await assert.rejects(retry("model", "project", input, async () => "committed"), /context_changed/);
  assert.equal(f.entries.size, 1);
});
