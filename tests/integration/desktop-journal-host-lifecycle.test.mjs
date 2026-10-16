import test, { before, after } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

const require = createRequire(new URL("../../apps/frontend/package.json", import.meta.url));
const { chromium } = require("playwright");
let browser;
before(async () => { browser = await launchIntegrationBrowser(chromium); });
after(async () => { await browser?.close(); });

const variants = [
  { name: "direct execution", file: "direct-execution-journal-bridge.js", install: "installDirectExecutionJournalHost",
    client: "createDirectExecutionJournalClient", protocol: "kyuubiki:direct-execution-journal/v2", intent: { scope: "a".repeat(64), endpoint: "agent:5001",
      study_kind: "axial_bar_1d", execution_id: "b".repeat(32), created_at: 1 } },
  { name: "checkpoint", file: "checkpoint-recovery-bridge.js", install: "installWorkbenchCheckpointHost",
    client: "createWorkbenchCheckpointClient", protocol: "kyuubiki:checkpoint-recovery/v2", intent: { key: "a".repeat(64), scope: "b".repeat(64),
      operation: "model", parentId: "p", requestId: "original-request-0001", createdAt: 1 } },
];

async function routeFixture(context, spec) {
  const source = await readFile(new URL(`../../apps/desktop-shared/ui/${spec.file}`, import.meta.url), "utf8");
  const sessionClient = await readFile(new URL("../../apps/desktop-shared/ui/journal-session-client.js", import.meta.url), "utf8");
  await context.route("**/*", async route => {
    const url = new URL(route.request().url());
    if (url.pathname === "/bridge.js") return route.fulfill({ contentType: "text/javascript", body: source });
    if (url.pathname === "/journal-session-client.js") return route.fulfill({ contentType: "text/javascript", body: sessionClient });
    if (url.hostname === "tauri.localhost") return route.fulfill({ contentType: "text/html", body:
      '<!doctype html><iframe id="workbench" src="http://127.0.0.1:3000/frame?desktopLayout=1&document=first"></iframe>' });
    if (url.origin === "http://127.0.0.1:3000") return route.fulfill({ contentType: "text/html", body:
      '<!doctype html><script>window.received=[];addEventListener("message",event=>{if(event.source===parent&&event.origin==="http://tauri.localhost")received.push(event.data);});</script>' });
    return route.abort();
  });
}
const connectFrame = (frame, protocol) => frame.evaluate(protocol => new Promise(resolve => {
  const id = crypto.randomUUID();
  const receive = event => {
    if (event.source !== parent || event.origin !== "http://tauri.localhost" || event.data.id !== id) return;
    removeEventListener("message", receive); window.session = event.data.session;
    window.received = []; resolve();
  };
  addEventListener("message", receive);
  parent.postMessage({ type: protocol, id, action: "connect" }, "http://tauri.localhost");
}), protocol);

for (const spec of variants) for (const firstAction of ["list", "reserve"]) for (const reload of [false, true]) {
  test(`${spec.name} real iframe ${reload ? "reload retires" : "unchanged document drains"} the queue behind ${firstAction}`, { timeout: 30_000 }, async () => {
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on("pageerror", error => errors.push(error.message));
    try {
      // Synthetic network origins, real cross-origin iframe/WindowProxy/message/load behavior.
      await routeFixture(context, spec);
      await page.goto("http://tauri.localhost/", { waitUntil: "load" });
      await page.evaluate(async ({ spec, firstAction }) => {
        const module = await import("/bridge.js"), frame = document.querySelector("iframe");
        let release;
        const gate = new Promise(resolve => { release = resolve; });
        const h = window.harness = { calls: [], ledger: firstAction === "reserve" ? [] : [spec.intent],
          paused: true, release, firstProxy: frame.contentWindow, loads: 0 };
        h.stop = module[spec.install](frame, async (command, payload) => {
          const action = spec.name === "direct execution" ? payload.payload.action : command.split("_")[0];
          h.calls.push(action);
          if (h.paused) await gate;
          if (action === "list") return structuredClone(h.ledger);
          if (action === "remove") { h.ledger = []; return null; }
          h.ledger = [spec.intent];
          return spec.name === "direct execution" ? { intent: spec.intent, is_new: true } : { intent: spec.intent, isNew: true };
        });
        frame.addEventListener("load", () => { h.loads++; });
      }, { spec, firstAction });
      let frame = page.frames().find(frame => frame.url().startsWith("http://127.0.0.1:3000/"));
      await connectFrame(frame, spec.protocol);
      const fields = action => action === "list" ? { action }
        : action === "reserve" || spec.name === "direct execution" ? { action, intent: spec.intent }
          : { action, key: spec.intent.key, requestId: spec.intent.requestId };
      const send = (id, action) => frame.evaluate(message => parent.postMessage({ ...message, session: window.session }, "http://tauri.localhost"),
        { type: spec.protocol, id, ...fields(action) });
      await send("old-in-flight", firstAction);
      await page.waitForFunction(() => window.harness.calls.length === 1);
      await send("old-queued-reserve", "reserve");
      await send("old-queued-remove", "remove");
      // A read-only marker from the same child confirms its previous messages were delivered.
      await page.evaluate(() => { window.marker = new Promise(resolve => {
        const receive = event => { if (event.source === document.querySelector("iframe").contentWindow && event.data === "queued") {
          removeEventListener("message", receive); resolve();
        } }; addEventListener("message", receive);
      }); });
      await frame.evaluate(() => parent.postMessage("queued", "http://tauri.localhost"));
      await page.evaluate(() => window.marker);
      if (reload) {
        await page.evaluate(() => { document.querySelector("iframe").src = "http://127.0.0.1:3000/frame?document=second"; });
        await page.waitForFunction(() => window.harness.loads === 1);
        assert.equal(await page.evaluate(() => window.harness.firstProxy === document.querySelector("iframe").contentWindow), true,
          "this regression requires a real stable WindowProxy across document navigation");
        frame = page.frames().find(frame => frame.url().includes("document=second"));
        await connectFrame(frame, spec.protocol);
      }
      await page.evaluate(() => { window.harness.paused = false; window.harness.release(); });
      await send("fresh-read", "list");
      await frame.waitForFunction(() => window.received.some(reply => reply.id === "fresh-read"));
      const replies = await frame.evaluate(() => window.received);
      const expected = reload ? [spec.intent] : [];
      assert.deepEqual(replies.find(reply => reply.id === "fresh-read").result, expected);
      assert.deepEqual(await page.evaluate(() => window.harness.calls), reload
        ? [firstAction, "list"] : [firstAction, "reserve", "remove", "list"]);
      if (reload) assert.deepEqual(replies.map(reply => reply.id), ["fresh-read"], "no old result leaks into the new document");
      await send("explicit-fresh-remove", "remove");
      await frame.waitForFunction(() => window.received.some(reply => reply.id === "explicit-fresh-remove"));
      assert.deepEqual(await page.evaluate(() => window.harness.ledger), []);
      assert.deepEqual(errors, []);
    } finally {
      await page.evaluate(() => { window.harness?.release(); window.harness?.stop(); }).catch(() => {});
      await context.close();
    }
  });
}

for (const spec of variants) for (const delayedAction of ["connect", "list", "reserve", "remove"]) {
  test(`${spec.name} real navigation ${delayedAction === "connect" ? "isolates" : "rejects"} delayed old ${delayedAction} before native dispatch`, { timeout: 30_000 }, async () => {
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on("pageerror", error => errors.push(error.message));
    try {
      await routeFixture(context, spec);
      await page.goto("http://tauri.localhost/", { waitUntil: "load" });
      await page.evaluate(async spec => {
        const module = await import("/bridge.js"), frame = document.querySelector("iframe");
        const h = window.harness = { calls: [], ledger: [spec.intent], firstProxy: frame.contentWindow, loads: 0, held: null };
        const wrappers = new Map();
        // Hold a real browser MessageEvent before host receipt to model delayed task delivery.
        const host = { crypto, addEventListener(type, fn) {
          const wrapped = type === "message" ? event => {
            if (event.data?.id === "delayed-old") h.held = () => fn(event); else fn(event);
          } : fn;
          wrappers.set(fn, wrapped); addEventListener(type, wrapped);
        }, removeEventListener(type, fn) { removeEventListener(type, wrappers.get(fn)); wrappers.delete(fn); } };
        h.stop = module[spec.install](frame, async (command, payload) => {
          const action = spec.name === "direct execution" ? payload.payload.action : command.split("_")[0];
          h.calls.push(action);
          if (action === "list") return structuredClone(h.ledger);
          if (action === "remove") { h.ledger = []; return null; }
          return spec.name === "direct execution" ? { intent: spec.intent, is_new: false } : { intent: spec.intent, isNew: false };
        }, host);
        frame.addEventListener("load", () => { h.loads++; });
      }, spec);
      let frame = page.frames().find(frame => frame.url().includes("document=first"));
      await connectFrame(frame, spec.protocol);
      await frame.evaluate(({ spec, action }) => {
        const payload = action === "connect" ? {} : { session: window.session };
        if (action === "reserve" || (action === "remove" && spec.name === "direct execution")) payload.intent = spec.intent;
        else if (action === "remove") { payload.key = spec.intent.key; payload.requestId = spec.intent.requestId; }
        parent.postMessage({ type: spec.protocol, id: "delayed-old", action, ...payload }, "http://tauri.localhost");
      }, { spec, action: delayedAction });
      await page.waitForFunction(() => window.harness.held !== null);
      await page.evaluate(() => { document.querySelector("iframe").src = "http://127.0.0.1:3000/frame?desktopLayout=1&document=second"; });
      await page.waitForFunction(() => window.harness.loads === 1);
      assert.equal(await page.evaluate(() => window.harness.firstProxy === document.querySelector("iframe").contentWindow), true);
      frame = page.frames().find(frame => frame.url().includes("document=second"));
      await frame.evaluate(async spec => { const module = await import("/bridge.js"); window.client = module[spec.client](); }, spec);
      await page.evaluate(() => window.harness.held());
      await frame.waitForFunction(() => window.received.some(reply => reply.id === "delayed-old"));
      const oldReply = await frame.evaluate(() => window.received.find(reply => reply.id === "delayed-old"));
      assert.equal(oldReply.ok, delayedAction === "connect");
      if (delayedAction !== "connect") assert.equal(oldReply.expired, true);
      assert.deepEqual(await page.evaluate(() => window.harness.calls), [], "old messages never read or mutate the ledger");
      assert.deepEqual(await frame.evaluate(() => window.client.list()), [spec.intent]);
      await frame.evaluate(async spec => {
        if (spec.name === "direct execution") await window.client.remove(spec.intent);
        else await window.client.remove(spec.intent.key, spec.intent.requestId);
        window.client.dispose();
      }, spec);
      assert.deepEqual(await page.evaluate(() => window.harness.calls), ["list", "remove"]);
      assert.deepEqual(await page.evaluate(() => window.harness.ledger), []);
      assert.deepEqual(errors, []);
    } finally {
      await page.evaluate(() => window.harness?.stop()).catch(() => {});
      await context.close();
    }
  });
}
