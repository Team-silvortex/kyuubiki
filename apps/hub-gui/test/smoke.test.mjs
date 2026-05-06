import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function read(relativePath) {
  return fs.readFileSync(path.join(ROOT, relativePath), "utf8");
}

test("hub shell exposes the desktop information architecture", () => {
  const html = read("ui/index.html");

  assert.match(html, /data-target="projects"/);
  assert.match(html, /data-target="runtimes"/);
  assert.match(html, /data-target="deploy"/);
  assert.match(html, /data-target="observe"/);
  assert.match(html, /data-target="tools"/);
  assert.match(html, /Open workbench/);
  assert.match(html, /Start local stack/);
});

test("hub shell registers section switching behavior", () => {
  const js = read("ui/app.js");
  const bridge = read("ui/shared/tauri-bridge.js");

  assert.match(js, /sectionModel/);
  assert.match(js, /setSection/);
  assert.match(js, /hub-nav__item--active/);
  assert.match(js, /addEventListener\("click"/);
  assert.match(bridge, /invokeTauri/);
  assert.match(bridge, /loadDesktopBrand/);
});

test("tauri backend exposes hub runtime commands", () => {
  const rust = read("src-tauri/src/main.rs");

  assert.match(rust, /service_status/);
  assert.match(rust, /service_start/);
  assert.match(rust, /service_restart/);
  assert.match(rust, /service_stop/);
  assert.match(rust, /read_runtime_log/);
  assert.match(rust, /hub_environment/);
});
