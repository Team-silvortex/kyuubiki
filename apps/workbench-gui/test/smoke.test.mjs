import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function read(relativePath) {
  return fs.readFileSync(path.join(ROOT, relativePath), "utf8");
}

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

  assert.match(html, /data-console-tab="status"/);
  assert.match(html, /data-console-tab="logs"/);
  assert.match(html, /data-log-service="frontend"/);
  assert.match(html, /data-log-service="orchestrator"/);
  assert.match(html, /data-shell-page="control"/);
  assert.match(html, /data-shell-page="workbench"/);
  assert.match(html, /data-shell-pane="control"/);
  assert.match(html, /data-shell-pane="workbench"/);
  assert.match(html, /id="workbench-frame"/);
});

test("desktop shell registers local runtime actions and shortcuts", () => {
  const html = read("ui/index.html");
  const js = read("ui/app.js");
  const bridge = read("ui/shared/tauri-bridge.js");

  assert.match(html, /shortcut-list/);
  assert.match(html, /reload embedded workbench/);
  assert.match(js, /guarded_mutation_action/);
  assert.match(js, /invokeGuardedMutation/);
  assert.match(js, /read_runtime_log/);
  assert.match(js, /setShellPage/);
  assert.match(js, /renderShellPages/);
  assert.match(js, /keydown/);
  assert.match(bridge, /invokeTauri/);
  assert.match(bridge, /loadDesktopBrand/);
});

test("tauri backend exposes workbench runtime commands", () => {
  const rust = read("src-tauri/src/main.rs");

  assert.match(rust, /service_status/);
  assert.match(rust, /guarded_mutation_action/);
  assert.match(rust, /read_runtime_log/);
  assert.match(rust, /workbench_environment/);
});
