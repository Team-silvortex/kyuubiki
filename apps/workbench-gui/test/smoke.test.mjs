import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const ROOT = "/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui";

function read(relativePath) {
  return fs.readFileSync(path.join(ROOT, relativePath), "utf8");
}

test("desktop shell exposes runtime and log panels", () => {
  const html = read("ui/index.html");

  assert.match(html, /data-console-tab="status"/);
  assert.match(html, /data-console-tab="logs"/);
  assert.match(html, /data-log-service="frontend"/);
  assert.match(html, /data-log-service="orchestrator"/);
  assert.match(html, /id="workbench-frame"/);
});

test("desktop shell registers local runtime actions and shortcuts", () => {
  const html = read("ui/index.html");
  const js = read("ui/app.js");

  assert.match(html, /shortcut-list/);
  assert.match(html, /reload embedded workbench/);
  assert.match(js, /service_start/);
  assert.match(js, /service_restart/);
  assert.match(js, /service_stop/);
  assert.match(js, /read_runtime_log/);
  assert.match(js, /keydown/);
});

test("tauri backend exposes workbench runtime commands", () => {
  const rust = read("src-tauri/src/main.rs");

  assert.match(rust, /service_status/);
  assert.match(rust, /service_start/);
  assert.match(rust, /service_restart/);
  assert.match(rust, /service_stop/);
  assert.match(rust, /read_runtime_log/);
  assert.match(rust, /workbench_environment/);
});
