import test from "node:test";
import assert from "node:assert/strict";
import { assertMatches } from "../../desktop-shared/test/smoke-test-helpers.mjs";
import {
  HUB_INFORMATION_ARCHITECTURE_PATTERNS,
  HUB_PLATFORM_HELPER_PATTERNS,
  read,
} from "./smoke-fixtures.mjs";

test("hub shell defines a least-privilege main-window capability", () => {
  const tauriConfig = JSON.parse(read("src-tauri/tauri.conf.json"));
  const capability = JSON.parse(read("src-tauri/capabilities/main.json"));
  const permissions = read("src-tauri/permissions/hub.toml");

  assert.equal(tauriConfig.app.windows[0]?.label, "main");
  assert.equal(capability.identifier, "main");
  assert.deepEqual(capability.windows, ["main"]);
  assert.ok(Array.isArray(capability.permissions));
  assert.ok(capability.permissions.includes("core:default"));
  assert.ok(capability.permissions.includes("allow-guarded-mutation-action"));
  assert.ok(capability.permissions.includes("allow-service-status"));
  assert.ok(capability.permissions.includes("allow-project-bundle-inspect"));
  assert.ok(capability.permissions.includes("allow-hub-environment"));
  assert.match(permissions, /identifier = "allow-service-status"/);
  assert.match(permissions, /commands\.allow = \["service_status"\]/);
  assert.match(permissions, /identifier = "allow-guarded-mutation-action"/);
});

test("hub shell exposes the desktop information architecture", () => {
  const html = read("ui/index.html");
  assertMatches(html, HUB_INFORMATION_ARCHITECTURE_PATTERNS);
});

test("hub shell normalizes host platform through shared desktop helpers", () => {
  const js = read("ui/app.js");
  const platform = read("ui/shared/platform.js");

  assertMatches(js, HUB_PLATFORM_HELPER_PATTERNS);
  assert.doesNotMatch(js, /hostPlatform:\s*"macos"/);
  assert.match(platform, /desktop-shared\/ui\/platform\.js/);
});
