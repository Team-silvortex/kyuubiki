import test from "node:test";
import assert from "node:assert/strict";
import { createWorkbenchCheckpointClient, installWorkbenchCheckpointHost, validateCheckpointIntent } from "../ui/checkpoint-recovery-bridge.js";

const protocol = "kyuubiki:checkpoint-recovery/v1";
const intent = { key: "a".repeat(64), scope: "b".repeat(64), operation: "model", parentId: "p", requestId: "test-request-id-0001", createdAt: 1 };
const settle = () => new Promise(resolve => setImmediate(resolve));
function eventHost() {
  const listeners = new Set();
  return { addEventListener: (_type, listener) => listeners.add(listener), removeEventListener: (_type, listener) => listeners.delete(listener),
    emit: event => { for (const listener of listeners) listener(event); }, listeners };
}

test("checkpoint bridge validates metadata and cannot accept arbitrary native commands or fields", async () => {
  const host = eventHost();
  const replies = [];
  const frame = { contentWindow: { postMessage: (data, origin) => replies.push({ data, origin }) } };
  const calls = [];
  const stop = installWorkbenchCheckpointHost(frame, async (command, payload) => {
    calls.push({ command, payload });
    return command === "list_checkpoint_intents" ? [intent] : command === "reserve_checkpoint_intent" ? { intent, isNew: false } : null;
  }, host);
  const data = { type: protocol, id: "one", action: "list" };
  const event = { source: frame.contentWindow, origin: "http://127.0.0.1:3000", data };
  for (const bad of [{ ...event, source: {} }, { ...event, origin: "https://untrusted.test" },
    { ...event, data: { ...data, action: "service_stop" } }, { ...event, data: { ...data, action: { toString: null } } },
    { ...event, data: { ...data, path: "elsewhere" } }, { ...event, data: { ...data, action: "reserve", intent: { ...intent, token: "secret" } } }]) host.emit(bad);
  await settle();
  assert.equal(calls.length, 0);
  host.emit(event);
  host.emit({ ...event, data: { ...data, id: "two", action: "reserve", intent } });
  host.emit({ ...event, data: { ...data, id: "three", action: "remove", key: intent.key, requestId: intent.requestId } });
  await settle();
  assert.deepEqual(calls.map(value => value.command), ["list_checkpoint_intents", "reserve_checkpoint_intent", "remove_checkpoint_intent"]);
  assert.ok(replies.every(value => value.origin === event.origin && value.data.ok));
  stop();
  assert.equal(host.listeners.size, 0);
  for (const bad of [{ ...intent, createdAt: Number.MAX_SAFE_INTEGER }, { ...intent, operation: "delete" }, { ...intent, payload: {} }]) {
    assert.throws(() => validateCheckpointIntent(bad));
  }
});

test("host bounds its queue and redacts native errors", async () => {
  const host = eventHost();
  const replies = [];
  const frame = { contentWindow: { postMessage: data => replies.push(data) } };
  let calls = 0;
  const stop = installWorkbenchCheckpointHost(frame, async () => { calls++; throw new Error("private filesystem path"); }, host);
  for (let i = 0; i < 9; i++) host.emit({ source: frame.contentWindow, origin: "http://127.0.0.1:3000", data: { type: protocol, id: `r${i}`, action: "list" } });
  await settle();
  assert.equal(calls, 8);
  assert.ok(replies.some(reply => reply.error === "checkpoint:journal_busy"));
  assert.doesNotMatch(JSON.stringify(replies), /private filesystem/);
  stop();
});

function clientFixture(timeout = 1000) {
  const host = eventHost();
  const sent = [];
  host.parent = { postMessage: (data, origin) => sent.push({ data, origin }) };
  host.location = { search: "?desktopLayout=1" };
  host.crypto = globalThis.crypto;
  host.setTimeout = setTimeout;
  host.clearTimeout = clearTimeout;
  const client = createWorkbenchCheckpointClient(host, timeout);
  const reply = (index, result = [], extra = {}) => host.emit({ source: host.parent, origin: "tauri://localhost",
    data: { type: `${protocol}:result`, id: sent[index].data.id, ok: true, result }, ...extra });
  return { host, sent, client, reply };
}

test("client pins the parent before sending identities and rejects spoofed or unrelated responses", async () => {
  const f = clientFixture();
  const reserved = f.client.reserve(intent);
  assert.equal(f.sent[0].data.action, "list");
  assert.equal(f.sent[0].origin, "*");
  assert.equal(f.sent[0].data.intent, undefined);
  f.reply(0, [], { source: {} });
  f.reply(0, [], { origin: "null" });
  await settle();
  assert.equal(f.sent.length, 1);
  f.reply(0);
  await settle();
  assert.equal(f.sent[1].origin, "tauri://localhost");
  assert.deepEqual(f.sent[1].data.intent, intent);
  f.reply(1, { intent, isNew: true });
  assert.deepEqual(await reserved, { intent, isNew: true });
  const bad = assert.rejects(f.client.reserve(intent), /journal_corrupt/);
  await settle();
  f.reply(2, { intent: { ...intent, scope: "c".repeat(64) }, isNew: false });
  await bad;
  f.client.dispose();
});

test("desktop host absence fails closed, and disposal rejects outstanding operations", async () => {
  const f = clientFixture(10);
  await assert.rejects(f.client.reserve(intent), /journal_unavailable/);
  assert.ok(f.sent.every(value => value.data.action === "list"), "no mutation sent without discovery");
  const read = assert.rejects(f.client.list(), /journal_unavailable/);
  f.client.dispose();
  await read;
  assert.equal(f.host.listeners.size, 0);
  const host = eventHost(); host.parent = host;
  assert.equal(createWorkbenchCheckpointClient(host), null);
});
