import test from "node:test";
import assert from "node:assert/strict";
import { assertMatches } from "../../desktop-shared/test/smoke-test-helpers.mjs";
import { HUB_BACKEND_PATTERNS, read } from "./smoke-fixtures.mjs";

test("tauri backend exposes hub runtime commands", () => {
  const rust = read("src-tauri/src/main.rs");
  const runtimeRust = read("../../workers/rust/crates/desktop-runtime/src/lib.rs");

  assertMatches(rust, HUB_BACKEND_PATTERNS);
  assert.match(runtimeRust, /failed to read .* log:/);
});
