import test from "node:test";
import assert from "node:assert/strict";
import { bindInstallerActionHandlers } from "../ui/installer-event-bindings.js";

function fixture(t, handlers, onEvent = () => {}) {
  const previous = {
    document: globalThis.document,
    window: globalThis.window,
    CustomEvent: globalThis.CustomEvent,
  };
  t.after(() => Object.assign(globalThis, previous));
  const listeners = new Map();
  const events = [];
  globalThis.window = {};
  globalThis.CustomEvent = class {
    constructor(type, options = {}) { this.type = type; this.detail = options.detail; }
  };
  globalThis.document = {
    addEventListener: (type, listener) => listeners.set(type, listener),
    dispatchEvent: (event) => { events.push(event.detail); onEvent(event.detail); },
  };
  bindInstallerActionHandlers(handlers);
  const button = (action, disabled = false) => {
    const attributes = new Map();
    return {
      disabled, dataset: { action },
      getAttribute: (name) => attributes.get(name) ?? null,
      setAttribute: (name, value) => attributes.set(name, value),
      removeAttribute: (name) => attributes.delete(name),
    };
  };
  const click = (target) => listeners.get("click")({ target: { closest: () => target } });
  return { button, click, events };
}

function deferred() {
  let resolve, reject;
  const promise = new Promise((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

test("installer failure settles without replacing the last successful action", async (t) => {
  const { button, click, events } = fixture(t, {
    success: async () => {},
    failure: async () => { throw new Error("forced installer failure"); },
  });
  await click(button("success"));
  const completedAt = window.__kyuubikiInstallerActionCompletedAt;
  await click(button("failure"));
  assert.equal(window.__kyuubikiInstallerActionStatus, "failed");
  assert.equal(window.__kyuubikiInstallerLastCompletedAction, "success");
  assert.equal(window.__kyuubikiInstallerActionCompletedAt, completedAt);
  assert.ok(window.__kyuubikiInstallerActionSettledAt >= completedAt);
  assert.equal(events.at(-1).error, "forced installer failure");
});

test("installer blocks duplicate and competing actions without queueing mutations", async (t) => {
  const pending = deferred();
  const calls = [];
  const { button, click, events } = fixture(t, {
    bootstrap: () => { calls.push("bootstrap"); return pending.promise; },
    install: () => { calls.push("install"); },
  });
  const activeButton = button("bootstrap");
  const first = click(activeButton);
  const startedAt = window.__kyuubikiInstallerActionStartedAt;
  // A second Bootstrap control must share the same owner as the first one.
  const attempts = [click(button("bootstrap")), click(button("install"))];
  pending.resolve();
  await Promise.all([first, ...attempts]);
  assert.deepEqual(calls, ["bootstrap"]);
  assert.deepEqual(events.map(({ action, status }) => [action, status]), [
    ["bootstrap", "running"], ["bootstrap", "blocked"],
    ["install", "blocked"], ["bootstrap", "completed"],
  ]);
  assert.equal(events[2].activeAction, "bootstrap");
  assert.equal(events.at(-1).activeAction, null);
  assert.equal(window.__kyuubikiInstallerActionStartedAt, startedAt);
  assert.equal(window.__kyuubikiInstallerActiveAction, null);
  assert.equal(activeButton.getAttribute("aria-busy"), null);
  await click(button("install"));
  assert.deepEqual(calls, ["bootstrap", "install"]);
});

test("installer failed mutation releases ownership for an explicit retry", async (t) => {
  const pending = deferred();
  let calls = 0;
  const { button, click } = fixture(t, {
    success: () => {},
    install: () => { calls += 1; return calls === 1 ? pending.promise : "installed"; },
  });
  await click(button("success"));
  const completedAt = window.__kyuubikiInstallerActionCompletedAt;
  const target = button("install");
  const first = click(target);
  const busy = target.getAttribute("aria-busy");
  const blocked = click(button("install"));
  pending.reject(new Error("transport unavailable"));
  await Promise.all([first, blocked]);
  assert.equal(busy, "true");
  assert.equal(calls, 1);
  assert.equal(window.__kyuubikiInstallerActionStatus, "failed");
  assert.equal(window.__kyuubikiInstallerLastCompletedAction, "success");
  assert.equal(window.__kyuubikiInstallerActionCompletedAt, completedAt);
  assert.equal(window.__kyuubikiInstallerActiveAction, null);
  assert.equal(target.getAttribute("aria-busy"), null);
  await click(target);
  assert.equal(calls, 2);
  assert.equal(window.__kyuubikiInstallerLastCompletedAction, "install");
});

test("installer synchronous failures release ownership without enabling guarded controls", async (t) => {
  const { button, click } = fixture(t, {
    failure: () => { throw new Error("invalid form"); },
    disable: () => { target.disabled = true; },
  });
  await click(button("failure"));
  assert.equal(window.__kyuubikiInstallerActiveAction, null);
  const target = button("disable");
  target.setAttribute("aria-busy", "false");
  await click(target);
  assert.equal(target.disabled, true);
  assert.equal(target.getAttribute("aria-busy"), "false");
  assert.equal(window.__kyuubikiInstallerLastCompletedAction, "disable");
});

test("installer rejects missing, non-callable and inherited handlers", async (t) => {
  let calls = 0;
  const { button, click, events } = fixture(t, { install: () => { calls += 1; }, invalid: "install" });
  await click(button("install", true));
  await click(null);
  assert.equal(events.length, 0);
  for (const name of ["missing", "invalid", "toString", "constructor"]) {
    await click(button(name));
    assert.equal(events.at(-1).status, "missing", name);
  }
  assert.equal(calls, 0);
  assert.equal(window.__kyuubikiInstallerLastCompletedAction, undefined);
});

test("installer acquires ownership before publishing running and releases before completion", async (t) => {
  const calls = [];
  const attempts = [];
  const { button, click, events } = fixture(t, {
    install: () => { calls.push("install"); },
    next: () => { calls.push("next"); },
  }, ({ action, status }) => {
    if (action === "install" && status === "running") attempts.push(click(button("next")));
    if (action === "install" && status === "completed") attempts.push(click(button("next")));
  });
  await click(button("install"));
  await Promise.all(attempts);
  assert.deepEqual(calls, ["install", "next"]);
  assert.equal(events[1].status, "blocked");
  assert.equal(window.__kyuubikiInstallerLastCompletedAction, "next");
});
