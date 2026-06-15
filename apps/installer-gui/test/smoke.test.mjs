import test from "node:test";
import assert from "node:assert/strict";
import {
  assertMatches,
  createFixtureReader,
  createFixtureRoot,
} from "../../desktop-shared/test/smoke-test-helpers.mjs";

const ROOT = createFixtureRoot(import.meta.url);
const read = createFixtureReader(ROOT);

test("installer shell defines a least-privilege main-window capability", () => {
  const tauriConfig = JSON.parse(read("src-tauri/tauri.conf.json"));
  const capability = JSON.parse(read("src-tauri/capabilities/main.json"));
  const permissions = read("src-tauri/permissions/installer.toml");

  assert.equal(tauriConfig.app.windows[0]?.label, "main");
  assert.equal(capability.identifier, "main");
  assert.deepEqual(capability.windows, ["main"]);
  assert.ok(capability.permissions.includes("core:default"));
  assert.ok(capability.permissions.includes("allow-guarded-mutation-action"));
  assert.ok(capability.permissions.includes("allow-service-status"));
  assert.ok(capability.permissions.includes("allow-read-env-file"));
  assert.match(permissions, /identifier = "allow-guarded-mutation-action"/);
  assert.match(permissions, /commands\.allow = \["guarded_mutation_action"\]/);
});

test("installer shell exposes setup, services, remote, and release surfaces", () => {
  const html = read("ui/index.html");

  assertMatches(html, [
    /data-tab="setup"/,
    /data-tab="services"/,
    /data-tab="remote"/,
    /data-tab="release"/,
    /Run doctor/,
    /Bootstrap workspace/,
    /placeholder="dist\/\{platform\}"/,
  ]);
});

test("installer shell wires core install and runtime actions", () => {
  const js = read("ui/app.js");
  const bridge = read("ui/shared/tauri-bridge.js");
  const platform = read("ui/shared/platform.js");

  assertMatches(js, [
    /doctor_report/,
    /guarded_mutation_action/,
    /invokeGuardedMutation/,
    /populateDesktopPlatformSelect/,
    /syncDesktopReleaseTargetInput/,
  ]);
  assert.match(bridge, /desktop-shared\/ui\/tauri-bridge\.js/);
  assert.match(platform, /desktop-shared\/ui\/platform\.js/);
});

test("tauri backend exposes installer command surface", () => {
  const rust = read("src-tauri/src/main.rs");

  assertMatches(rust, [
    /doctor_report/,
    /guarded_mutation_action/,
    /service_status/,
    /start_log_stream/,
    /read_env_file/,
  ]);
});
