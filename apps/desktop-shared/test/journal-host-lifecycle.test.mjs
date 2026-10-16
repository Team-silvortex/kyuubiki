import test from "node:test";
import assert from "node:assert/strict";
import { installDirectExecutionJournalHost } from "../ui/direct-execution-journal-bridge.js";
import { installWorkbenchCheckpointHost } from "../ui/checkpoint-recovery-bridge.js";

const origin = "http://127.0.0.1:3000";
const settle = () => new Promise(resolve => setImmediate(resolve));
function events() {
  const listeners = new Map();
  return { crypto: globalThis.crypto, listeners, addEventListener: (type, fn) => listeners.set(fn, type),
    removeEventListener: (_type, fn) => listeners.delete(fn),
    emit(event, type = "message") { for (const [fn, kind] of listeners) if (type === kind) fn(event); } };
}
const cases = [
  { name: "direct execution", protocol: "kyuubiki:direct-execution-journal/v2", install: installDirectExecutionJournalHost,
    intent: { scope: "a".repeat(64), endpoint: "agent:5001", study_kind: "axial_bar_1d", execution_id: "b".repeat(32), created_at: 1 },
    reserve(intent) { return { action: "reserve", intent }; }, remove(intent) { return { action: "remove", intent }; },
    action(_command, payload) { return payload.payload.action; },
    reserved(intent) { return { intent, is_new: true }; } },
  { name: "checkpoint", protocol: "kyuubiki:checkpoint-recovery/v2", install: installWorkbenchCheckpointHost,
    intent: { key: "a".repeat(64), scope: "b".repeat(64), operation: "model", parentId: "p", requestId: "request-original-0001", createdAt: 1 },
    reserve(intent) { return { action: "reserve", intent }; }, remove(intent) { return { action: "remove", key: intent.key, requestId: intent.requestId }; },
    action(command) { return command.split("_")[0]; }, reserved(intent) { return { intent, isNew: true }; } },
];

function fixture(spec) {
  const host = events(), replies = [], calls = [];
  const frame = { ...events(), isConnected: true, contentWindow: { postMessage: value => replies.push(value) } };
  const state = { pause: true, throwAfterWait: false, entries: [structuredClone(spec.intent)] };
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const stop = spec.install(frame, async (command, payload) => {
    const action = spec.action(command, payload);
    calls.push({ command, payload: structuredClone(payload), action });
    if (state.pause) await gate;
    if (state.throwAfterWait) throw new Error("private storage path");
    if (action === "list") return structuredClone(state.entries);
    if (action === "remove") { state.entries = []; return null; }
    state.entries = [structuredClone(spec.intent)]; return spec.reserved(spec.intent);
  }, host);
  let session;
  const connect = () => {
    host.emit({ source: frame.contentWindow, origin, data: { type: spec.protocol, id: "handshake", action: "connect" } });
    session = replies.pop().session;
    assert.equal(typeof session, "string");
  };
  connect();
  const message = (id, fields = { action: "list" }) => ({ type: spec.protocol, id, ...fields, session });
  const send = (id, fields = { action: "list" }) => {
    const data = message(id, fields);
    host.emit({ source: frame.contentWindow, origin, data });
    return data;
  };
  return { host, frame, replies, calls, state, connect, message, send, stop, release };
}

for (const spec of cases) {
  test(`${spec.name} handshake is metadata-free and never falls back to v1 or unfenced journal calls`, async () => {
    const f = fixture(spec);
    try {
      const native = f.message("unfenced", spec.remove(spec.intent));
      delete native.session;
      for (const data of [native, { ...f.message("v1", spec.remove(spec.intent)), type: spec.protocol.replace("v2", "v1") },
        { type: spec.protocol, id: "bad-connect", action: "connect", intent: spec.intent }]) {
        f.host.emit({ source: f.frame.contentWindow, origin, data });
      }
      f.release(); await settle();
      assert.deepEqual(f.calls, []); assert.deepEqual(f.replies, []);
      f.connect(); assert.deepEqual(f.calls, []);
    } finally { f.release(); f.stop(); }
  });
  for (const action of ["list", "reserve", "remove"]) test(`${spec.name} rejects old ${action} first delivered after replacement load`, async () => {
    const f = fixture(spec);
    try {
      const delayed = f.message("old-delayed", action === "list" ? { action } : spec[action](spec.intent));
      f.frame.emit({}, "load");
      f.host.emit({ source: f.frame.contentWindow, origin, data: delayed });
      f.release(); await settle();
      assert.deepEqual(f.calls, [], "late messages must not acquire the replacement document's authority");
      assert.deepEqual(f.state.entries, [spec.intent]);
    } finally { f.release(); f.stop(); }
  });
  for (const boundary of ["frame replacement", "same WindowProxy load", "detached frame", "host pagehide", "dispose"]) {
    test(`${spec.name} host drops queued mutations and late replies after ${boundary}`, async () => {
      const f = fixture(spec);
      try {
        f.send("slow-read"); await settle();
        assert.equal(f.calls.length, 1);
        f.send("old-reserve", spec.reserve(structuredClone(spec.intent)));
        f.send("old-remove", spec.remove(structuredClone(spec.intent)));
        if (boundary === "frame replacement") f.frame.contentWindow = { postMessage: value => f.replies.push(value) };
        if (boundary === "same WindowProxy load") f.frame.emit({}, "load");
        if (boundary === "detached frame") f.frame.isConnected = false;
        if (boundary === "host pagehide") f.host.emit({}, "pagehide");
        if (boundary === "dispose") f.stop();
        f.state.pause = false; f.release(); await settle();
        assert.deepEqual(f.calls.map(call => call.action), ["list"], "obsolete queued writes must never reach native storage");
        assert.deepEqual(f.replies, [], "obsolete replies must not reach a replacement document");
        assert.deepEqual(f.state.entries, [spec.intent]);
        if (boundary === "dispose") {
          assert.equal(f.host.listeners.size, 0); assert.equal(f.frame.listeners.size, 0);
          return;
        }
        if (boundary === "host pagehide") {
          f.send("hidden-remove", spec.remove(spec.intent)); await settle();
          assert.equal(f.calls.length, 1);
          f.host.emit({}, "pageshow");
        }
        f.frame.isConnected = true;
        f.connect();
        f.send("fresh-read"); await settle();
        assert.deepEqual(f.replies.map(reply => reply.id), ["fresh-read"]);
        assert.deepEqual(f.replies[0].result, [spec.intent]);
        f.send("explicit-fresh-remove", spec.remove(spec.intent)); await settle();
        assert.deepEqual(f.state.entries, []);
        assert.deepEqual(f.calls.map(call => call.action), ["list", "list", "remove"]);
      } finally { f.release(); f.stop(); }
    });
  }

  for (const failed of [false, true]) {
    test(`${spec.name} native mutation already in flight is not rolled back or replied across reload (failed=${failed})`, async () => {
      const f = fixture(spec); f.state.entries = [];
      try {
        f.send("in-flight-reserve", spec.reserve(spec.intent)); await settle();
        f.send("queued-remove", spec.remove(spec.intent));
        f.frame.emit({}, "load");
        f.state.throwAfterWait = failed; f.release(); await settle();
        assert.deepEqual(f.calls.map(call => call.action), ["reserve"]);
        assert.deepEqual(f.replies, []);
        assert.deepEqual(f.state.entries, failed ? [] : [spec.intent]);
      } finally { f.release(); f.stop(); }
    });
  }

  for (const action of ["reserve", "remove"]) test(`${spec.name} queued ${action} identity and metadata are snapshots, not mutable event objects`, async () => {
    const f = fixture(spec);
    try {
      f.send("slow-read"); await settle();
      const data = f.send("original-request", spec[action](structuredClone(spec.intent)));
      data.id = "changed-id";
      if (data.intent) {
        if (spec.name === "direct execution") data.intent.execution_id = "c".repeat(32);
        else data.intent.requestId = "request-replacement";
      }
      else { data.key = "c".repeat(64); data.requestId = "request-replacement"; }
      f.release(); await settle();
      assert.equal(f.replies[1].id, "original-request");
      assert.equal(f.replies[1].ok, true);
      const payload = f.calls[1].payload;
      if (spec.name === "direct execution") assert.deepEqual(payload.payload.intent, spec.intent);
      else if (action === "reserve") assert.deepEqual(payload, { payload: spec.intent });
      else assert.deepEqual(payload, { key: spec.intent.key, requestId: spec.intent.requestId });
    } finally { f.release(); f.stop(); }
  });

  test(`${spec.name} reload does not reset occupied queue capacity behind native work`, async () => {
    const f = fixture(spec);
    try {
      f.send("slow-read"); await settle();
      for (let index = 0; index < 7; index++) f.send(`old-${index}`);
      f.frame.emit({}, "load");
      f.connect();
      f.send("fresh-but-full");
      assert.equal(f.replies.length, 1);
      assert.equal(f.replies[0].id, "fresh-but-full");
      assert.equal(f.replies[0].ok, false);
      assert.match(f.replies[0].error, /:journal_busy$/);
      f.release(); await settle();
      assert.equal(f.calls.length, 1);
      assert.equal(f.replies.length, 1, "retired queue entries cannot reply after draining");
      f.send("fresh-after-drain"); await settle();
      assert.equal(f.calls.length, 2);
      assert.equal(f.replies[1].id, "fresh-after-drain");
      assert.equal(f.replies[1].ok, true);
    } finally { f.release(); f.stop(); }
  });
}
