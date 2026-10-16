import test from "node:test";
import assert from "node:assert/strict";
import { createDirectExecutionJournalClient, installDirectExecutionJournalHost, validateDirectExecutionIntent } from "../ui/direct-execution-journal-bridge.js";
const protocol = "kyuubiki:direct-execution-journal/v2";
const session = "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa";
const intent = { scope: "a".repeat(64), endpoint: "agent:5001", study_kind: "axial_bar_1d", execution_id: "b".repeat(32), created_at: 1 };
function events() {
  const listeners = new Map();
  return { crypto: globalThis.crypto, addEventListener: (type, fn) => listeners.set(fn, type), removeEventListener: (_, fn) => listeners.delete(fn),
    emit: value => { for (const [fn, type] of listeners) if (type === "message") fn(value); }, listeners };
}
const settle = () => new Promise(resolve => setImmediate(resolve));
test("native bridge only accepts bounded identities from its known frame", async () => {
  const host = events(), calls = [], replies = [];
  const frame = { ...events(), isConnected: true, contentWindow: { postMessage: (value, origin) => replies.push({ value, origin }) } };
  const stop = installDirectExecutionJournalHost(frame, async (command, payload) => { calls.push({ command, payload }); return []; }, host);
  const event = { source: frame.contentWindow, origin: "http://127.0.0.1:3000", data: { type: protocol, id: "one", action: "list" } };
  host.emit({ ...event, data: { type: protocol, id: "connect", action: "connect" } });
  event.data.session = replies.pop().value.session;
  for (const bad of [{ ...event, source: {} }, { ...event, origin: "https://elsewhere.test" },
    { ...event, data: { ...event.data, path: "elsewhere" } }, { ...event, data: { ...event.data, action: "exec" } },
    { ...event, data: { ...event.data, action: "reserve", intent: { ...intent, token: "secret" } } }]) host.emit(bad);
  await settle(); assert.equal(calls.length, 0);
  host.emit(event); await settle();
  assert.deepEqual(calls, [{ command: "direct_execution_journal", payload: { payload: { action: "list" } } }]);
  assert.equal(replies[0].origin, event.origin); assert.equal(replies[0].value.ok, true);
  stop(); assert.equal(host.listeners.size, 0);
});
test("strict journal metadata rejects payloads, unknown studies and noncanonical endpoints", () => {
  for (const value of [{ ...intent, model: {} }, { ...intent, endpoint: "agent:05001" }, { ...intent, study_kind: "new" }, { ...intent, created_at: Infinity }]) {
    assert.throws(() => validateDirectExecutionIntent(value));
  }
});
test("desktop client discovers only with metadata-free handshakes; writes target the verified shell", async () => {
  const host = events(), messages = [];
  host.parent = { postMessage(data, origin) { messages.push({ data, origin }); } };
  Object.assign(host, { crypto: globalThis.crypto, location: { search: "?desktopLayout=1" }, setTimeout, clearTimeout });
  const client = createDirectExecutionJournalClient(host, 1000);
  const pending = client.reserve(intent);
  assert.equal(messages[0].data.action, "connect"); assert.equal(messages[0].origin, "*"); assert.equal(messages[0].data.intent, undefined);
  host.emit({ source: host.parent, origin: "https://untrusted.test", data: { type: `${protocol}:result`, id: messages[0].data.id, session, ok: true, result: null } });
  assert.equal(messages.length, 1);
  host.emit({ source: host.parent, origin: "tauri://localhost", data: { type: `${protocol}:result`, id: messages[0].data.id, session, ok: true, result: null } });
  await settle();
  assert.equal(messages[1].origin, "tauri://localhost"); assert.deepEqual(messages[1].data.intent, intent);
  host.emit({ source: host.parent, origin: "tauri://localhost", data: { type: `${protocol}:result`, id: messages[1].data.id, session, ok: true, result: { is_new: true, intent } } });
  assert.deepEqual(await pending, { is_new: true, intent }); client.dispose();
});
test("missing desktop host fails closed rather than using a memory journal", async () => {
  const host = events(); Object.assign(host, { parent: { postMessage() {} }, crypto: globalThis.crypto,
    location: { search: "?desktopLayout=1" }, setTimeout, clearTimeout });
  const client = createDirectExecutionJournalClient(host, 5);
  await assert.rejects(client.reserve(intent), /journal_unavailable/); client.dispose();
});

test("desktop cleanup rechecks caller ownership after its initial shell handshake", async () => {
  const host = events(), messages = [];
  Object.assign(host, { parent: { postMessage: (data, origin) => messages.push({ data, origin }) },
    crypto: globalThis.crypto, location: { search: "?desktopLayout=1" }, setTimeout, clearTimeout });
  const client = createDirectExecutionJournalClient(host, 1000);
  let current = true;
  const pending = client.remove(intent, () => { if (!current) throw new Error("caller retired"); });
  const outcome = pending.then(() => null, error => error);
  try {
    assert.equal(messages.length, 1); assert.equal(messages[0].data.action, "connect");
    current = false;
    host.emit({ source: host.parent, origin: "tauri://localhost", data: {
      type: `${protocol}:result`, id: messages[0].data.id, session, ok: true, result: null,
    } });
    await settle();
    assert.equal(messages.length, 1, "no native remove may be sent after ownership was lost during handshake");
    assert.match((await outcome).message, /caller retired/);
  } finally { client.dispose(); await outcome; }
});

test("desktop cleanup checks ownership before discovery and never serializes the guard", async () => {
  const host = events(), messages = [];
  Object.assign(host, { parent: { postMessage: (data, origin) => messages.push({ data, origin }) },
    crypto: globalThis.crypto, location: { search: "?desktopLayout=1" }, setTimeout, clearTimeout });
  const client = createDirectExecutionJournalClient(host, 1000);
  try {
    await assert.rejects(client.remove(intent, () => { throw new Error("already retired"); }), /already retired/);
    assert.deepEqual(messages, []);
    const pending = client.remove(intent, () => {});
    host.emit({ source: host.parent, origin: "tauri://localhost", data: {
      type: `${protocol}:result`, id: messages[0].data.id, session, ok: true, result: null,
    } });
    await settle();
    const sent = messages[1];
    assert.equal(sent.origin, "tauri://localhost");
    assert.deepEqual(Object.keys(sent.data).sort(), ["action", "id", "intent", "session", "type"]);
    assert.deepEqual(sent.data.intent, intent);
    host.emit({ source: host.parent, origin: sent.origin, data: {
      type: `${protocol}:result`, id: sent.data.id, session, ok: true, result: null,
    } });
    await pending;
    await assert.rejects(client.remove(intent, () => { throw new Error("retired with warm host"); }), /retired with warm host/);
    assert.equal(messages.length, 2, "a verified shell must not bypass the local deletion guard");
  } finally { client.dispose(); }
});
