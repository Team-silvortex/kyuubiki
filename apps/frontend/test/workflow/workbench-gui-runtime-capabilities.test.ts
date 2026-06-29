import test from "node:test";
import assert from "node:assert/strict";

import {
  explainWorkbenchGuiRuntimeBoundary,
  inferWorkbenchGuiHostKind,
  isWorkbenchBackendTargetAllowedForGuiCapability,
  resolveWorkbenchGuiRuntimeCapability,
} from "../../src/lib/api/gui-runtime-capabilities.ts";

test("mobile WebView is a remote-control GUI, not a runtime host", () => {
  const capability = resolveWorkbenchGuiRuntimeCapability("mobile_webview");

  assert.equal(capability.canUseRemoteBackend, true);
  assert.equal(capability.canHostOrchestra, false);
  assert.equal(capability.canHostAgent, false);
  assert.equal(capability.canInstallRuntime, false);
  assert.equal(capability.canUseLocalhostBackend, false);
  assert.equal(capability.posture, "remote_control");
});

test("desktop WebView can expose local workstation runtime management", () => {
  const capability = resolveWorkbenchGuiRuntimeCapability("desktop_webview");

  assert.equal(capability.canHostOrchestra, true);
  assert.equal(capability.canHostAgent, true);
  assert.equal(capability.canInstallRuntime, true);
  assert.equal(capability.canUseLocalhostBackend, true);
  assert.equal(capability.posture, "local_workstation");
});

test("host inference separates mobile WebView from desktop tauri shells", () => {
  assert.equal(
    inferWorkbenchGuiHostKind({
      userAgent: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) Mobile/15E148",
    }),
    "mobile_webview",
  );
  assert.equal(inferWorkbenchGuiHostKind({ userAgent: "Mozilla/5.0", isTauri: true }), "desktop_webview");
  assert.equal(inferWorkbenchGuiHostKind({ userAgent: "Mozilla/5.0 (X11; Linux x86_64)" }), "browser");
});

test("mobile boundary explanation forbids local runtime assumptions", () => {
  const capability = resolveWorkbenchGuiRuntimeCapability("mobile_webview");
  const explanation = explainWorkbenchGuiRuntimeBoundary(capability);

  assert.equal(explanation.title, "Mobile GUI remote-control boundary");
  assert.ok(explanation.forbidden.includes("host_agent"));
  assert.ok(explanation.forbidden.includes("localhost_runtime_assumption"));
  assert.ok(explanation.allowed.includes("remote_job_submission"));
});

test("mobile GUI rejects localhost backend targets but accepts remote HTTP services", () => {
  const capability = resolveWorkbenchGuiRuntimeCapability("mobile_webview");

  assert.deepEqual(
    isWorkbenchBackendTargetAllowedForGuiCapability("http://localhost:4000", capability),
    { ok: false, reason: "localhost_forbidden" },
  );
  assert.deepEqual(
    isWorkbenchBackendTargetAllowedForGuiCapability("http://127.0.0.1:4000", capability),
    { ok: false, reason: "localhost_forbidden" },
  );
  assert.deepEqual(
    isWorkbenchBackendTargetAllowedForGuiCapability("http://127.0.1.1:4000", capability),
    { ok: false, reason: "localhost_forbidden" },
  );
  assert.deepEqual(
    isWorkbenchBackendTargetAllowedForGuiCapability("http://[::1]:4000", capability),
    { ok: false, reason: "localhost_forbidden" },
  );
  assert.deepEqual(
    isWorkbenchBackendTargetAllowedForGuiCapability("https://orch.example.local", capability),
    { ok: true },
  );
});
