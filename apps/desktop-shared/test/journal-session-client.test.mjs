import test from "node:test";
import assert from "node:assert/strict";
import { createDirectExecutionJournalClient } from "../ui/direct-execution-journal-bridge.js";
import { createWorkbenchCheckpointClient } from "../ui/checkpoint-recovery-bridge.js";

const settle = () => new Promise(resolve => setImmediate(resolve));
const token = "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa", fresh = "bbbbbbbb-bbbb-4bbb-bbbb-bbbbbbbbbbbb";
const cases = [
  { name: "direct", protocol: "kyuubiki:direct-execution-journal/v2", create: createDirectExecutionJournalClient,
    intent: { scope: "a".repeat(64), endpoint: "agent:5001", study_kind: "axial_bar_1d", execution_id: "b".repeat(32), created_at: 1 },
    result(intent) { return { intent, is_new: true }; } },
  { name: "checkpoint", protocol: "kyuubiki:checkpoint-recovery/v2", create: createWorkbenchCheckpointClient,
    intent: { key: "a".repeat(64), scope: "b".repeat(64), operation: "model", parentId: "p", requestId: "request-original-0001", createdAt: 1 },
    result(intent) { return { intent, isNew: true }; } },
];
function fixture(spec) {
  const listeners = new Map(), sent = [];
  const host = { crypto: globalThis.crypto, location: { search: "?desktopLayout=1" }, setTimeout, clearTimeout,
    parent: { postMessage(data, origin) { sent.push({ data: structuredClone(data), origin }); } },
    addEventListener(type, fn) { listeners.set(fn, type); }, removeEventListener(_type, fn) { listeners.delete(fn); },
    emit(event, type = "message") { for (const [fn, kind] of listeners) if (kind === type) fn(event); } };
  const client = spec.create(host, 1000);
  const reply = (index, result = null, fields = {}, event = {}) => host.emit({ source: host.parent, origin: "tauri://localhost",
    data: { type: `${spec.protocol}:result`, id: sent[index].data.id, session: token, ok: true, result, ...fields }, ...event });
  return { host, sent, client, reply, listeners };
}
for (const spec of cases) {
  for (const transition of ["pagehide", "restore", "dispose"]) {
    test(`${spec.name} ${transition} fences an acknowledged reservation before promise delivery`, async () => {
      const f = fixture(spec);
      let delivered = false;
      try {
        const result = f.client.reserve(spec.intent).then(value => { delivered = true; return value; });
        const rejected = assert.rejects(result, /journal_unavailable/);
        f.reply(0); await settle();
        f.reply(1, spec.result(spec.intent));
        if (transition === "dispose") f.client.dispose();
        else {
          f.host.emit({}, "pagehide");
          if (transition === "restore") f.host.emit({}, "pageshow");
        }
        await rejected;
        assert.equal(delivered, false);
        assert.deepEqual(f.sent.map(m => m.data.action), ["connect", "reserve"], "no rollback or replay of a possibly committed write");
      } finally { f.client.dispose(); }
    });
  }

  test(`${spec.name} cleanup rechecks caller ownership after acknowledgement`, async () => {
    const f = fixture(spec);
    let current = true;
    try {
      const check = () => { if (!current) throw new Error(`${spec.name}:authority_changed`); };
      const result = assert.rejects(spec.name === "direct" ? f.client.remove(spec.intent, check)
        : f.client.remove(spec.intent.key, spec.intent.requestId, check), /authority_changed/);
      f.reply(0); await settle();
      f.reply(1); current = false;
      await result;
      assert.deepEqual(f.sent.map(m => m.data.action), ["connect", "remove"]);
    } finally { f.client.dispose(); }
  });

  test(`${spec.name} session expiry rejects without replay; fresh inspection reconnects`, async () => {
    const f = fixture(spec);
    try {
      const failed = assert.rejects(f.client.reserve(spec.intent), /journal_unavailable/);
      f.reply(0); await settle();
      f.reply(1, null, { ok: false, expired: true, error: `${spec.name}:journal_unavailable` });
      await failed; await settle();
      assert.deepEqual(f.sent.map(m => m.data.action), ["connect", "reserve"]);
      const read = f.client.list();
      assert.equal(f.sent[2].data.action, "connect");
      assert.equal(f.sent[2].origin, "tauri://localhost");
      f.reply(2, null, { session: fresh }); await settle();
      assert.equal(f.sent[3].data.action, "list");
      assert.equal(f.sent[3].data.session, fresh);
      f.reply(3, [], { session: fresh });
      assert.deepEqual(await read, []);
    } finally { f.client.dispose(); }
  });

  for (const phase of ["handshake", "native reply"]) test(`${spec.name} pagehide retires pending ${phase} across pageshow`, async () => {
    const f = fixture(spec);
    try {
      const old = assert.rejects(f.client.reserve(spec.intent), /journal_unavailable/);
      if (phase === "native reply") { f.reply(0); await settle(); }
      const last = f.sent.length - 1;
      f.host.emit({}, "pagehide");
      await old;
      await assert.rejects(f.client.list(), /journal_unavailable/);
      f.host.emit({}, "pageshow");
      const read = f.client.list(), handshake = f.sent.length - 1;
      f.reply(last, phase === "handshake" ? null : spec.result(spec.intent)); await settle();
      assert.equal(f.sent.length, handshake + 1, "late old reply cannot start work in a restored page");
      f.reply(handshake, null, { session: fresh }); await settle();
      f.reply(handshake + 1, [], { session: fresh });
      assert.deepEqual(await read, []);
    } finally { f.client.dispose(); assert.equal(f.listeners.size, 0); }
  });

  test(`${spec.name} handshake waiters share one probe and stay within eight logical operations`, async () => {
    const f = fixture(spec);
    try {
      const pending = Promise.all(Array.from({ length: 8 }, () => f.client.reserve(spec.intent)));
      const outcome = pending.then(value => value, error => { throw error; });
      await assert.rejects(f.client.reserve(spec.intent), /journal_unavailable/);
      assert.equal(f.sent.length, 1);
      assert.deepEqual(Object.keys(f.sent[0].data).sort(), ["action", "id", "type"]);
      f.reply(0); await settle();
      assert.equal(f.sent.length, 9);
      for (let index = 1; index <= 8; index++) f.reply(index, spec.result(spec.intent));
      assert.equal((await outcome).length, 8);
    } finally { f.client.dispose(); }
  });

  test(`${spec.name} rejects malformed handshakes and ignores foreign or wrong-session results`, async () => {
    const f = fixture(spec);
    try {
      const bad = assert.rejects(f.client.list(), /journal_corrupt/);
      f.reply(0, null, { session: "bad" }); await bad;
      let resolved = false;
      const read = f.client.list().then(value => { resolved = true; return value; });
      f.reply(1, null, {}, { source: {} });
      f.reply(1, null, {}, { origin: "https://foreign.invalid" });
      await settle(); assert.equal(f.sent.length, 2);
      f.reply(1); await settle();
      for (const fields of [{ session: fresh }, { session: undefined }, { type: spec.protocol.replace("v2", "v1") + ":result" }]) f.reply(2, [], fields);
      await settle(); assert.equal(resolved, false);
      f.reply(2, []); assert.deepEqual(await read, []);
    } finally { f.client.dispose(); }
  });

  test(`${spec.name} reserve snapshots metadata before session handshake`, async () => {
    const f = fixture(spec);
    try {
      const value = structuredClone(spec.intent), pending = f.client.reserve(value);
      if (spec.name === "direct") value.execution_id = "c".repeat(32);
      else value.requestId = "request-replacement";
      f.reply(0); await settle();
      assert.deepEqual(f.sent[1].data.intent, spec.intent);
      f.reply(1, spec.result(spec.intent));
      assert.deepEqual(await pending, spec.result(spec.intent));
    } finally { f.client.dispose(); }
  });
}
