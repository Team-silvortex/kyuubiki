import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function read(relativePath) {
  return fs.readFileSync(path.join(ROOT, relativePath), "utf8");
}

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
  assert.match(js, /bootstrap/);
  assert.match(js, /write_env_file/);
  assert.match(js, /service_start/);
  assert.match(js, /service_restart/);
  assert.match(js, /service_stop/);
  assert.match(js, /remote_bootstrap/);
  assert.match(js, /remote_start_agent/);
  assert.match(js, /build_installer_bundle/);
  assert.match(bridge, /invokeTauri/);
  assert.match(bridge, /listenTauri/);
  assert.match(bridge, /loadDesktopBrand/);
});

test("tauri backend exposes installer command surface", () => {
  const rust = read("src-tauri/src/main.rs");

  assert.match(rust, /doctor_report/);
  assert.match(rust, /validate_env/);
  assert.match(rust, /prepare_layout/);
  assert.match(rust, /bootstrap/);
  assert.match(rust, /service_status/);
  assert.match(rust, /start_log_stream/);
  assert.match(rust, /remote_bootstrap/);
  assert.match(rust, /remote_start_agent/);
  assert.match(rust, /build_installer_bundle/);
});
