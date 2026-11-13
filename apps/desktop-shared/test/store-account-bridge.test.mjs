import assert from "node:assert/strict";
import test from "node:test";
import { storeAccountPortalUrl, installStoreAccountHost, requestStoreAccountPortal } from "../ui/store-account-bridge.js";

const protocol = "kyuubiki:store-account/v1";
const settle = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
function host() {
  const groups = new Map();
  return { addEventListener: (type, fn) => { if (!groups.has(type)) groups.set(type, new Set()); groups.get(type).add(fn); },
    removeEventListener: (type, fn) => groups.get(type)?.delete(fn),
    emit: (event, type = "message") => groups.get(type)?.forEach(fn => fn(event)),
    get listeners() { return new Set([...groups.values()].flatMap(group => [...group])); }, crypto: globalThis.crypto,
    setTimeout, clearTimeout };
}

test("account links contain only the explicit HTTPS origin and supported language", () => {
  assert.equal(storeAccountPortalUrl("https://id.silvortex.test:3443/", "zh-cn"), "https://id.silvortex.test:3443/account?lang=zh");
  assert.equal(storeAccountPortalUrl("https://example.test", "ja"), "https://example.test/account?lang=en");
  for (const origin of ["", "http://example.test", "javascript:alert(1)", "file:///tmp", "https://u:p@example.test",
    "https://example.test/account", "https://example.test?token=secret", "https://example.test#x", "https://example.test\\evil",
    "https://example.test\n", "https://example.test:0", "https://example.test/%2f", "https://example.test/.",
    "https://example.test:", "https://example.test:99999", "https://example.test/foo/..", "https:example.test",
    "https://:443", "https://[invalid]:443"]) {
    assert.throws(() => storeAccountPortalUrl(origin, "en"), undefined, origin);
  }
});

test("native account bridge pins the local iframe and only invokes the dedicated opener", async () => {
  const shell = host();
  const replies = [], calls = [];
  const frame = { ...host(), contentWindow: { postMessage: (data, origin) => replies.push({ data, origin }) } };
  const stop = installStoreAccountHost(frame, async (...args) => calls.push(args), shell);
  const event = { origin: "http://127.0.0.1:3000", source: frame.contentWindow,
    data: { type: protocol, id: "r1", origin: "https://example.test", language: "zh" } };
  shell.emit({ ...event, source: {} });
  shell.emit({ ...event, origin: "https://evil.test" });
  shell.emit({ ...event, data: { ...event.data, token: "never-forward" } });
  shell.emit({ ...event, data: { ...event.data, origin: "file:///tmp" } });
  await settle();
  assert.equal(calls.length, 0);
  shell.emit(event);
  shell.emit(event);
  await settle();
  assert.equal(calls.length, 1, "duplicate clicks cannot open multiple native windows");
  assert.deepEqual(calls[0], ["open_store_account_portal", { origin: "https://example.test", language: "zh" }]);
  assert.ok(replies.every(reply => reply.origin === event.origin));
  stop();
  assert.equal(shell.listeners.size, 0);
});

test("client accepts only the native parent response and cleans up on timeout", async () => {
  const child = host(), sent = [];
  child.parent = { postMessage: (data, origin) => sent.push({ data, origin }) };
  child.location = { search: "?desktopLayout=1" };
  const request = requestStoreAccountPortal("https://example.test", "en", child, 1000);
  const response = { source: child.parent, origin: "tauri://localhost", data: {
    type: `${protocol}:result`, id: sent[0].data.id, ok: true } };
  child.emit({ ...response, origin: "https://evil.test" });
  assert.equal(child.listeners.size, 1);
  child.emit(response);
  await request;
  assert.equal(child.listeners.size, 0);
  await assert.rejects(requestStoreAccountPortal("https://example.test", "en", child, 5), /timed out/);
  assert.equal(child.listeners.size, 0);
  child.parent = child;
  await assert.rejects(requestStoreAccountPortal("https://example.test", "en", child), /desktop/);
});

test("native failures are sanitized and a later request can recover", async () => {
  const shell = host(), replies = [];
  let failed = true;
  const frame = { ...host(), contentWindow: { postMessage: data => replies.push(data) } };
  const stop = installStoreAccountHost(frame, async () => {
    if (failed) throw new Error("private native path and credentials");
  }, shell);
  const event = { origin: "http://127.0.0.1:3000", source: frame.contentWindow,
    data: { type: protocol, id: "r1", origin: "https://example.test", language: "en" } };
  shell.emit(event);
  await settle();
  assert.equal(replies[0].ok, false);
  assert.ok(!JSON.stringify(replies).includes("private"));
  failed = false;
  shell.emit({ ...event, data: { ...event.data, id: "r2" } });
  await settle();
  assert.equal(replies.at(-1).ok, true);
  stop();
});

test("account host fences browser opens and late replies across frame reload or shell close", async () => {
  for (const kind of ["reload", "pagehide", "dispose"]) {
    const shell = host(), replies = [], calls = [];
    const frame = Object.assign(host(), { contentWindow: { postMessage: data => replies.push(data) } });
    const stop = installStoreAccountHost(frame, async (...args) => calls.push(args), shell);
    const event = { origin: "http://127.0.0.1:3000", source: frame.contentWindow,
      data: { type: protocol, id: "old-page", origin: "https://example.test", language: "en" } };
    shell.emit(event);
    if (kind === "reload") frame.emit({}, "load");
    else if (kind === "pagehide") shell.emit({}, "pagehide");
    else stop();
    await settle();
    assert.equal(calls.length, 0, kind);
    assert.deepEqual(replies, []);
    stop();
    assert.equal(shell.listeners.size, 0);
    assert.equal(frame.listeners.size, 0);
  }
  const shell = host(), replies = [];
  const frame = Object.assign(host(), { contentWindow: { postMessage: data => replies.push(data) } });
  let finish;
  let calls = 0;
  const stop = installStoreAccountHost(frame, () => { calls++; return new Promise(resolve => { finish = resolve; }); }, shell);
  const event = { origin: "http://127.0.0.1:3000", source: frame.contentWindow,
    data: { type: protocol, id: "old-page", origin: "https://example.test", language: "en" } };
  shell.emit(event);
  await settle();
  frame.emit({}, "load");
  shell.emit({ ...event, data: { ...event.data, id: "busy-new-page" } });
  await settle();
  assert.equal(calls, 1, "reload must not bypass occupied native capacity");
  finish();
  await settle();
  assert.ok(replies.every(reply => reply.id !== "old-page"), "old native result cannot claim success in a new page");
  shell.emit({ ...event, data: { ...event.data, id: "retry-new-page" } });
  await settle();
  finish();
  await settle();
  assert.equal(calls, 2);
  assert.equal(replies.at(-1).ok, true);
  stop();
});
