import test from "node:test";
import assert from "node:assert/strict";
import { checkpointDigest, createCheckpointJournal, validateCheckpointIntent, type CheckpointIntent, type CheckpointJournal } from "../../src/lib/workbench/checkpoint-journal.ts";
import { createCheckpointRetry } from "../../src/lib/workbench/checkpoint-retry.ts";
import { createCheckpointRecovery } from "../../src/lib/workbench/checkpoint-recovery.ts";
import type { CheckpointReceipt, ModelVersionRecord } from "../../src/lib/api/project-types.ts";

const input = { name: "Private geometry", payload: { h: 1.5 }, request_id: undefined as string | undefined };
const committed = { status: "committed", project_id: "project", model_id: "model", version_id: "version" } as const;
function fixture() {
  const entries = new Map<string, CheckpointIntent>();
  const journal: CheckpointJournal = {
    async reserve(value) {
      const previous = entries.get(value.key);
      if (previous) return { intent: previous, isNew: false };
      entries.set(value.key, structuredClone(value));
      return { intent: value, isNew: true };
    },
    list: async () => structuredClone([...entries.values()]),
    async remove(key, requestId) { if (entries.get(key)?.requestId === requestId) entries.delete(key); },
  };
  let scope = "https://private-server:4000 secret-token";
  const options = { journal, scope: () => scope };
  const reads: string[][] = [];
  let receipt: unknown = committed;
  const recovery = createCheckpointRecovery({ ...options, lookup: async (...identity) => {
    reads.push(identity);
    return { checkpoint: receipt as CheckpointReceipt };
  } });
  const fail = async () => { throw new Error("response lost"); };
  const prepare = async () => {
    await assert.rejects(createCheckpointRetry(options)("model", "project", input, fail), /response lost/);
    return [...entries.values()][0];
  };
  return { journal, entries, options, recovery, reads, prepare, setScope: (value: string) => { scope = value; }, setReceipt: (value: unknown) => { receipt = value; } };
}

test("a fresh retry manager recovers the automatic identity without persisting payload or authority", async () => {
  const f = fixture();
  const original = await f.prepare();
  assert.deepEqual(Object.keys(original).sort(), ["createdAt", "key", "operation", "parentId", "requestId", "scope"]);
  assert.doesNotMatch(JSON.stringify(original), /Private|geometry|private-server|secret-token|https/);
  assert.equal(await createCheckpointRetry(f.options)("model", "project", input, async (body) => body.request_id), original.requestId);
  assert.equal(f.entries.size, 0);
});

test("journal storage failure stops a save before transport and does not leak in-flight capacity", async () => {
  const f = fixture();
  const journal = { ...f.journal, reserve: async () => { throw new Error("denied"); } };
  const retry = createCheckpointRetry({ journal, limit: 1 });
  let writes = 0;
  for (const parent of ["first", "second"]) {
    await assert.rejects(retry("model", parent, input, async () => { writes += 1; }), /denied/);
  }
  assert.equal(writes, 0);
  await assert.rejects(createCheckpointJournal(() => undefined).list(), /journal_unavailable/);
});

test("failure to clean a confirmed receipt is not reported as a failed save", async () => {
  const f = fixture();
  const journal = { ...f.journal, remove: async () => { throw new Error("quota/storage failure"); } };
  assert.equal(await createCheckpointRetry({ ...f.options, journal })("model", "project", input, async () => "saved"), "saved");
  assert.equal(f.entries.size, 1);
  assert.equal((await f.recovery.check([...f.entries.keys()][0])).status, "committed");
});

test("a rejected retry cannot erase an earlier uncertain commit", async () => {
  const f = fixture();
  const entry = await f.prepare();
  await assert.rejects(createCheckpointRetry(f.options)("model", "project", input, async () => {
    throw Object.assign(new Error("temporarily unauthorized"), { statusCode: 401 });
  }));
  assert.equal([...f.entries.values()][0].requestId, entry.requestId);
});

test("persistent retry coalesces concurrent calls and reserves once", async () => {
  const f = fixture();
  let writes = 0;
  let reserves = 0;
  let started!: () => void;
  let release!: () => void;
  const writing = new Promise<void>((resolve) => { started = resolve; });
  const held = new Promise<void>((resolve) => { release = resolve; });
  const retry = createCheckpointRetry({ ...f.options, journal: { ...f.journal, reserve: async (entry) => {
    reserves += 1; return f.journal.reserve(entry);
  } } });
  const requests = Array.from({ length: 8 }, () => retry("model", "project", input, async () => {
    writes += 1; started(); await held; return "saved";
  }));
  await writing;
  // Allow all real digest promises to reach the in-flight registry before release.
  await new Promise((resolve) => setTimeout(resolve, 20));
  release();
  assert.deepEqual(await Promise.all(requests), Array(8).fill("saved"));
  assert.equal(writes, 1);
  assert.equal(reserves, 1);
});

test("read-only recovery requires the original scope and strips unexpected server fields", async () => {
  const f = fixture();
  const entry = await f.prepare();
  f.setReceipt({ ...committed, payload: { secret: true }, token: "secret" });
  assert.deepEqual(await f.recovery.check(entry.key), committed);
  assert.deepEqual(f.reads, [["model", "project", entry.requestId]]);
  assert.equal(f.entries.size, 1, "check does not acknowledge or remove");
  f.setScope("other-authority");
  assert.deepEqual(await f.recovery.list(), []);
  await assert.rejects(f.recovery.check(entry.key), /pending_not_found/);
  assert.equal(f.reads.length, 1);
});

test("unknown receipt can neither be opened nor acknowledged", async () => {
  const f = fixture();
  const entry = await f.prepare();
  f.setReceipt({ status: "unknown" });
  assert.deepEqual(await f.recovery.check(entry.key), { status: "unknown" });
  await assert.rejects(f.recovery.acknowledge(entry.key), /commit_unconfirmed/);
  await assert.rejects(f.recovery.open(entry.key, async () => assert.fail("must not open")), /commit_unconfirmed/);
  assert.equal(f.entries.size, 1);
});

test("deleted saved result can be acknowledged, never resurrected", async () => {
  const f = fixture();
  const entry = await f.prepare();
  f.setReceipt({ ...committed, status: "deleted" });
  await assert.rejects(f.recovery.open(entry.key, async () => assert.fail("must not open")), /result_deleted/);
  assert.equal((await f.recovery.acknowledge(entry.key)).status, "deleted");
  assert.equal(f.entries.size, 0);
});

test("opening validates exact checkpoint identity before workspace mutation", async () => {
  const f = fixture();
  const entry = await f.prepare();
  let opens = 0;
  assert.deepEqual(await f.recovery.open(entry.key, async (id, validate) => {
    assert.equal(id, committed.version_id);
    validate(committed as unknown as ModelVersionRecord);
    opens += 1;
    return { ok: true };
  }), committed);
  assert.equal(opens, 1);
  assert.equal(f.entries.size, 1, "opening does not silently acknowledge");
  await assert.rejects(f.recovery.open(entry.key, async (_id, validate) => {
    validate({ ...committed, version_id: "another-version" } as unknown as ModelVersionRecord);
    return { ok: true };
  }), /invalid_response/);
});

test("authority changes during receipt or version fetch reject before use", async () => {
  const f = fixture();
  const entry = await f.prepare();
  const service = createCheckpointRecovery({ ...f.options, lookup: async () => {
    f.setScope("new-server"); return { checkpoint: committed };
  } });
  await assert.rejects(service.check(entry.key), /context_changed/);
  f.setScope("https://private-server:4000 secret-token");
  await assert.rejects(f.recovery.open(entry.key, async (_id, validate) => {
    f.setScope("new-server"); validate(committed as unknown as ModelVersionRecord); return { ok: true };
  }), /context_changed/);
  assert.equal(f.entries.size, 1);
});

test("corrupt, wrong-parent or failed lookup keeps the uncertain journal entry", async () => {
  const f = fixture();
  const entry = await f.prepare();
  for (const receipt of [null, {}, { status: "committed" }, { ...committed, project_id: "elsewhere" }]) {
    f.setReceipt(receipt);
    await assert.rejects(f.recovery.check(entry.key), /invalid_response/);
  }
  const service = createCheckpointRecovery({ ...f.options, lookup: async () => { throw new Error("offline"); } });
  await assert.rejects(service.acknowledge(entry.key), /offline/);
  assert.equal(f.entries.size, 1);
});

test("journal validator rejects extra sensitive fields and malformed metadata", async () => {
  const f = fixture();
  const entry = await f.prepare();
  for (const value of [null, { ...entry, payload: {} }, { ...entry, token: "x" }, { ...entry, requestId: "short" },
    { ...entry, key: "bad" }, { ...entry, createdAt: NaN }, { ...entry, operation: "delete" }]) {
    assert.throws(() => validateCheckpointIntent(value), /journal_corrupt/);
  }
  assert.equal(await checkpointDigest("a"), "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb");
});
