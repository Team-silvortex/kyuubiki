import test from "node:test";
import assert from "node:assert/strict";
import {
  assertMatches,
  createFixtureReader,
  createFixtureRoot,
} from "../../desktop-shared/test/smoke-test-helpers.mjs";

const ROOT = createFixtureRoot(import.meta.url);
const read = createFixtureReader(ROOT);

test("desktop shell defines a least-privilege main-window capability", () => {
  const tauriConfig = JSON.parse(read("src-tauri/tauri.conf.json"));
  const capability = JSON.parse(read("src-tauri/capabilities/main.json"));
  const permissions = read("src-tauri/permissions/workbench.toml");

  assert.equal(tauriConfig.app.windows[0]?.label, "main");
  assert.equal(capability.identifier, "main");
  assert.deepEqual(capability.windows, ["main"]);
  assert.ok(capability.permissions.includes("core:default"));
  assert.ok(capability.permissions.includes("allow-guarded-mutation-action"));
  assert.ok(capability.permissions.includes("allow-read-runtime-log"));
  assert.ok(capability.permissions.includes("allow-workbench-environment"));
  assert.match(permissions, /identifier = "allow-guarded-mutation-action"/);
  assert.match(permissions, /commands\.allow = \["guarded_mutation_action"\]/);
});

test("desktop shell exposes runtime and log panels", () => {
  const html = read("ui/index.html");

  assertMatches(html, [
    /data-console-tab="status"/,
    /data-console-tab="logs"/,
    /data-log-service="frontend"/,
    /data-log-service="orchestrator"/,
    /data-shell-page="control"/,
    /data-shell-page="workbench"/,
    /data-shell-pane="control"/,
    /data-shell-pane="workbench"/,
    /id="workbench-frame"/,
  ]);
});

test("desktop shell registers local runtime actions and shortcuts", () => {
  const html = read("ui/index.html");
  const js = read("ui/app.js");
  const bridge = read("ui/shared/tauri-bridge.js");
  const platform = read("ui/shared/platform.js");

  assertMatches(html, [/shortcut-list/, /reload embedded workbench/]);
  assertMatches(js, [
    /guarded_mutation_action/,
    /invokeGuardedMutation/,
    /read_runtime_log/,
    /watchDesktopLanguagePreference/,
    /setShellPage/,
    /renderShellPages/,
    /workbenchShellLanguageOptions/,
    /"pt-BR", "Português \(Brasil\)"/,
    /"zh-TW", "繁體中文 · Traditional Chinese"/,
    /normalizeDesktopPlatform/,
    /keydown/,
  ]);
  assert.match(bridge, /export async function invokeTauri/);
  assert.match(bridge, /export function applyDesktopState/);
  assert.doesNotMatch(bridge, /desktop-shared\/ui\/tauri-bridge\.js/);
  assert.match(platform, /export function normalizeDesktopPlatform/);
  assert.doesNotMatch(platform, /desktop-shared\/ui\/platform\.js/);
});

test("tauri backend exposes workbench runtime commands", () => {
  const rust = read("src-tauri/src/main.rs");

  assertMatches(rust, [
    /service_status/,
    /guarded_mutation_action/,
    /read_runtime_log/,
    /workbench_environment/,
  ]);
});
