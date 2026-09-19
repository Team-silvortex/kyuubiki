import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRuntimeLogController } from "../ui/runtime-log-panel.js";
import { builtinInstallerSetupCopy } from "../ui/installer-setup-copy.js";
import { loadInstallerShellTranslation } from "../ui/installer-shell-translations.js";

const locales = JSON.parse(readFileSync(new URL(
  "../../../config/localization/mainstream-language-pack-locales.json", import.meta.url,
), "utf8")).locales;
const format = (copy, key, service = "orchestrator") => copy[key].replaceAll("{service}", () => service);

function control(value) {
  const listeners = new Map();
  return {
    value, checked: false,
    addEventListener: (type, callback) => listeners.set(type, callback),
    change() { return listeners.get("change")({ target: this }); },
  };
}

function fixture(t, options = {}) {
  const previousWindow = globalThis.window;
  const timers = new Map();
  let nextTimer = 0;
  globalThis.window = {
    setInterval(callback, delay) {
      assert.equal(delay, 3000);
      timers.set(++nextTimer, callback);
      return nextTimer;
    },
  };
  t.mock.method(globalThis, "clearInterval", (id) => timers.delete(id));
  t.after(() => { globalThis.window = previousWindow; });
  const calls = [], rendered = [], completed = [], listeners = new Set();
  const logServiceSelect = control("orchestrator");
  const liveTailToggle = control();
  let copy = options.copy || builtinInstallerSetupCopy.zh;
  const controller = createRuntimeLogController({
    invoke: async (command, payload) => {
      calls.push({ command, payload });
      if (command === options.fail) throw new Error("native diagnostic remains untranslated");
      return command === "read_runtime_log" ? { service: payload.service, rendered: options.raw || "" } : "ok";
    },
    listen: async (name, callback) => {
      assert.equal(name, "runtime-log-update");
      listeners.add(callback);
      return () => listeners.delete(callback);
    },
    getCopy: () => copy,
    logServiceSelect, liveTailToggle,
    renderRuntimeLog: (value) => rendered.push(value),
    showCompletion: (value) => completed.push(value),
  });
  t.after(() => controller.stopRuntimeLogStream());
  return {
    ...controller, calls, rendered, completed, timers, listeners, logServiceSelect, liveTailToggle,
    setCopy: (value) => { copy = value; },
    emit: (service, value) => {
      for (const callback of listeners) callback({ payload: { service, rendered: value } });
    },
  };
}

test("runtime log messages use all 34 dictionaries and retain service identifiers", async (t) => {
  const f = fixture(t);
  for (const language of ["en", "zh", "ja", "es", ...locales.map(({ language }) => language)]) {
    const copy = builtinInstallerSetupCopy[language] || (await loadInstallerShellTranslation(language)).setup;
    f.setCopy(copy);
    const service = "agent-5001";
    f.logServiceSelect.value = service;
    assert.equal(await f.refreshRuntimeLog(), format(copy, "logLoaded", service), language);
    assert.equal(f.rendered.at(-1), format(copy, "logEmpty", service), language);
    assert.deepEqual(f.calls.at(-1), { command: "read_runtime_log", payload: { service } });
    assert.equal(await f.startRuntimeLogStream(), format(copy, "logAttached", service), language);
    assert.equal(f.completed.at(-1), format(copy, "logAttached", service), language);
    await f.stopRuntimeLogStream();
    assert.equal(f.listeners.size, 0);
    assert.equal(f.timers.size, 0);
  }
});

test("runtime log language changes do not restart the stream or rewrite backend content", async (t) => {
  const raw = "[solver] <mesh> iteration=42 status=ready";
  const f = fixture(t, { raw });
  await f.startRuntimeLogStream();
  assert.equal(f.rendered.at(-1), raw);
  const before = f.calls.length;
  f.setCopy(builtinInstallerSetupCopy.es);
  f.emit("frontend", "wrong service");
  assert.equal(f.rendered.at(-1), raw);
  f.emit("orchestrator", "");
  assert.equal(f.rendered.at(-1), format(builtinInstallerSetupCopy.es, "logEmpty"));
  f.emit("orchestrator", raw);
  assert.equal(f.rendered.at(-1), raw);
  assert.equal(f.calls.length, before, "changing language needs no service restart");
  assert.deepEqual(f.completed, [format(builtinInstallerSetupCopy.zh, "logAttached")]);
});

test("runtime log fallback reports polling rather than claiming a live connection", async (t) => {
  const f = fixture(t, { fail: "start_log_stream" });
  assert.equal(await f.startRuntimeLogStream(), format(builtinInstallerSetupCopy.zh, "logPolling"));
  assert.deepEqual(f.completed, [format(builtinInstallerSetupCopy.zh, "logPolling")]);
  assert.equal(f.listeners.size, 0);
  assert.equal(f.timers.size, 1);
  f.setCopy(builtinInstallerSetupCopy.es);
  f.timers.values().next().value();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(f.rendered.at(-1), format(builtinInstallerSetupCopy.es, "logEmpty"));
  await f.stopRuntimeLogStream();
  assert.equal(f.timers.size, 0);
});

test("a failed initial log read releases the started native stream before polling", async (t) => {
  const f = fixture(t, { fail: "read_runtime_log" });
  await assert.rejects(f.refreshRuntimeLog(), /native diagnostic remains untranslated/u);
  f.calls.length = 0;
  assert.equal(await f.startRuntimeLogStream(), format(builtinInstallerSetupCopy.zh, "logPolling"));
  assert.deepEqual(f.calls.map(({ command }) => command), ["start_log_stream", "read_runtime_log", "stop_log_stream"]);
  assert.equal(f.listeners.size, 0);
  assert.equal(f.timers.size, 1);
  assert.deepEqual(f.completed, [format(builtinInstallerSetupCopy.zh, "logPolling")]);
});

test("runtime log selector and live toggle preserve native targets and stop listeners", async (t) => {
  const f = fixture(t);
  f.liveTailToggle.checked = true;
  await f.liveTailToggle.change();
  f.calls.length = 0;
  f.logServiceSelect.value = "frontend";
  await f.logServiceSelect.change();
  assert.deepEqual(f.calls, [
    { command: "stop_log_stream", payload: { service: "orchestrator" } },
    { command: "start_log_stream", payload: { service: "frontend" } },
    { command: "read_runtime_log", payload: { service: "frontend" } },
  ]);
  assert.equal(f.listeners.size, 1);
  f.liveTailToggle.checked = false;
  await f.liveTailToggle.change();
  assert.equal(f.listeners.size, 0);
  assert.equal(f.timers.size, 0);
  assert.equal(f.calls.at(-2).command, "stop_log_stream");
  assert.equal(f.calls.at(-1).command, "read_runtime_log");
});

test("service identifiers are interpolated literally, including replacement metacharacters", async (t) => {
  const f = fixture(t);
  f.logServiceSelect.value = "agent-$&-<test>";
  assert.equal(await f.refreshRuntimeLog(), "已加载 agent-$&-<test> 日志。");
  assert.equal(f.rendered.at(-1), "agent-$&-<test> 暂无日志。");
});
