import assert from "node:assert/strict";
import test from "node:test";
import { createWorkbenchPanelLayoutClient, installWorkbenchPanelLayoutHost, isPanelLayoutRecord } from "../ui/workbench-panel-layout-bridge.js";

const protocol = "kyuubiki:workbench-panel-layout/v1";
const layout = { version: 1, sizes: { sidebar: 327, inspector: 267, report: 274 } };
const settle = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };
function eventHost() {
  const listeners = new Set();
  return { addEventListener: (_type, listener) => listeners.add(listener), removeEventListener: (_type, listener) => listeners.delete(listener),
    emit: event => { for (const listener of listeners) listener(event); }, listeners };
}

test("layout schema permits only bounded dimensions, not arbitrary preference keys", () => {
  assert.equal(isPanelLayoutRecord(layout), true);
  assert.equal(isPanelLayoutRecord({ version: 1, sizes: {} }), true);
  for (const invalid of [null, [], {}, { ...layout, version: 2 }, { ...layout, path: "secrets" },
    { version: 1, sizes: { sidebar: Infinity } }, { version: 1, sizes: { report: 5001 } },
    { version: 1, sizes: { report: -10 } }, { version: 1, sizes: { report: "300" } },
    { version: 1, sizes: { other: 300 } }]) assert.equal(isPanelLayoutRecord(invalid), false);
});

test("host requires both the pinned origin and exact iframe source and serializes only dedicated IPC", async () => {
  const host = eventHost();
  const replies = [];
  const frame = { contentWindow: { postMessage: (data, origin) => replies.push({ data, origin }) } };
  const calls = [];
  const stop = installWorkbenchPanelLayoutHost(frame, async (command, payload) => { calls.push({ command, payload }); return layout; }, host);
  const data = { type: protocol, id: "test-read", action: "read" };
  const event = { source: frame.contentWindow, origin: "http://127.0.0.1:3000", data };
  host.emit({ ...event, source: {} });
  host.emit({ ...event, origin: "https://untrusted.test" });
  host.emit({ ...event, data: { ...data, action: "service_stop" } });
  host.emit({ ...event, data: { ...data, path: "elsewhere" } });
  host.emit({ ...event, data: { ...data, action: "write", layout: { version: 1, sizes: { path: 5 } } } });
  await settle();
  assert.equal(calls.length, 0);
  host.emit(event);
  host.emit({ ...event, data: { ...data, id: "test-write", action: "write", layout } });
  await settle();
  assert.deepEqual(calls, [{ command: "get_workbench_panel_layout", payload: {} }, { command: "set_workbench_panel_layout", payload: { payload: layout } }]);
  assert.ok(replies.every(reply => reply.origin === event.origin && reply.data.ok === true));
  stop();
  assert.equal(host.listeners.size, 0);
});

test("host bounds queued writes and recovers after a failed or closed-frame response", async () => {
  const host = eventHost();
  const replies = [];
  let failed = false;
  let calls = 0;
  const frame = { contentWindow: { postMessage: data => { if (data.ok && !failed) { failed = true; throw new Error("frame closed"); } replies.push(data); } } };
  const stop = installWorkbenchPanelLayoutHost(frame, async () => { calls++; return layout; }, host);
  for (let i = 0; i < 9; i++) host.emit({ source: frame.contentWindow, origin: "http://127.0.0.1:3000", data: { type: protocol, action: "read", id: `r${i}` } });
  await settle();
  await settle();
  assert.equal(calls, 8);
  assert.ok(replies.some(reply => reply.id === "r8" && reply.ok === false));
  assert.ok(replies.some(reply => reply.id === "r7" && reply.ok));
  stop();
});

function clientFixture(timeout = 4000) {
  const host = eventHost();
  const sent = [];
  host.parent = { postMessage: (data, origin) => sent.push({ data, origin }) };
  host.location = { search: "?desktopLayout=1" };
  host.crypto = globalThis.crypto;
  host.setTimeout = setTimeout;
  host.clearTimeout = clearTimeout;
  const client = createWorkbenchPanelLayoutClient(host, timeout);
  const reply = (index, value = layout, extra = {}) => host.emit({ source: host.parent, origin: "tauri://localhost",
    data: { type: `${protocol}:result`, id: sent[index].data.id, ok: true, layout: value }, ...extra });
  return { host, client, sent, reply };
}

test("client pins the native parent before writing and rejects spoofed, foreign and mismatched replies", async () => {
  const f = clientFixture();
  await assert.rejects(f.client.write(layout));
  const read = f.client.read();
  let resolved = false;
  void read.then(() => { resolved = true; });
  f.reply(0, layout, { source: {} });
  f.reply(0, layout, { origin: "null" });
  f.reply(0, layout, { origin: "https://evil.test" });
  await settle();
  assert.equal(resolved, false);
  f.reply(0);
  assert.deepEqual(await read, layout);
  const write = f.client.write(layout);
  assert.equal(f.sent[1].origin, "tauri://localhost");
  f.reply(1);
  await write;
  f.client.dispose();
  assert.equal(f.host.listeners.size, 0);
});

test("client rejects invalid native data, times out cleanly and disposes pending requests", async () => {
  const f = clientFixture(10);
  const invalid = assert.rejects(f.client.read());
  f.reply(0, { ...layout, version: 99 });
  await invalid;
  await assert.rejects(f.client.read(), /timed out/);
  const closed = assert.rejects(f.client.read(), /closed/);
  f.client.dispose();
  await closed;
  await assert.rejects(f.client.read(), /not ready/);
});

test("normal browser windows and unmarked embeds do not open native persistence IPC", () => {
  const host = eventHost();
  host.parent = host;
  assert.equal(createWorkbenchPanelLayoutClient(host), null);
  host.parent = {};
  host.location = { search: "" };
  assert.equal(createWorkbenchPanelLayoutClient(host), null);
  assert.equal(host.listeners.size, 0);
});
