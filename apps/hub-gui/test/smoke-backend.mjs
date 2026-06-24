import test from "node:test";
import assert from "node:assert/strict";
import { assertMatches } from "../../desktop-shared/test/smoke-test-helpers.mjs";
import { HUB_BACKEND_PATTERNS, read } from "./smoke-fixtures.mjs";

test("tauri backend exposes hub runtime commands", () => {
  const rust = read("src-tauri/src/main.rs");
  const commands = read("src-tauri/src/hub_commands.rs");
  const desktopLaunch = read("src-tauri/src/hub_desktop_launch.rs");
  const desktopStatus = read("src-tauri/src/hub_desktop_status.rs");
  const runtimeRust = read("../../workers/rust/crates/desktop-runtime/src/lib.rs");
  const backendSource = `${rust}\n${commands}\n${desktopLaunch}\n${desktopStatus}`;

  assertMatches(backendSource, HUB_BACKEND_PATTERNS);
  assert.match(backendSource, /hub_regression_gate_report/);
  assert.match(runtimeRust, /failed to read .* log:/);
});
