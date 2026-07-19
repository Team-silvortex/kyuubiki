import assert from "node:assert/strict";
import test from "node:test";
import { assertNoUnmanagedLocalRuntime } from "./support/local-runtime-lock.mjs";

test("local runtime isolation rejects unmanaged services", () => {
  assert.throws(
    () => assertNoUnmanagedLocalRuntime("orchestrator: running on http://127.0.0.1:4000 (unmanaged pid)"),
    /requires an isolated local runtime/,
  );
  assert.doesNotThrow(() => assertNoUnmanagedLocalRuntime("orchestrator: stopped"));
});
