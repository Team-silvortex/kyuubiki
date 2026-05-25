import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function read(relativePath) {
  return fs.readFileSync(path.join(ROOT, relativePath), "utf8");
}

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

  assert.match(html, /data-tab="setup"/);
  assert.match(html, /data-tab="services"/);
  assert.match(html, /data-tab="remote"/);
  assert.match(html, /data-tab="release"/);
  assert.match(html, /Run doctor/);
  assert.match(html, /Bootstrap workspace/);
});

test("installer shell wires core install and runtime actions", () => {
  const js = read("ui/app.js");
  const bridge = read("ui/shared/tauri-bridge.js");

  assert.match(js, /doctor_report/);
  assert.match(js, /guarded_mutation_action/);
  assert.match(js, /invokeGuardedMutation/);
  assert.match(bridge, /invokeTauri/);
  assert.match(bridge, /listenTauri/);
  assert.match(bridge, /loadDesktopBrand/);
});

test("tauri backend exposes installer command surface", () => {
  const rust = read("src-tauri/src/main.rs");

  assert.match(rust, /doctor_report/);
  assert.match(rust, /guarded_mutation_action/);
  assert.match(rust, /service_status/);
  assert.match(rust, /start_log_stream/);
  assert.match(rust, /read_env_file/);
});
