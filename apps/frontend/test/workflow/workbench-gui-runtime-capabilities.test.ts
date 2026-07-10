import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

import {
  explainWorkbenchGuiRuntimeBoundary,
  type GuiRuntimeCapabilityManifest,
  hasGuiRuntimeManifestCapability,
  inferWorkbenchGuiHostKind,
  isWorkbenchBackendTargetAllowedForGuiCapability,
  listGuiRuntimeManifestCapabilities,
  resolveWorkbenchGuiRuntimeCapability,
  resolveWorkbenchGuiRuntimeCapabilityFromManifest,
  selectGuiRuntimeManifestBindings,
} from "../../src/lib/api/gui-runtime-capabilities.ts";

const repoRoot = path.resolve(new URL("../../../../", import.meta.url).pathname);

function readManifest(name: string): GuiRuntimeCapabilityManifest {
  return JSON.parse(
    fs.readFileSync(path.join(repoRoot, "config/gui-runtime-capabilities", `${name}.json`), "utf8"),
  ) as GuiRuntimeCapabilityManifest;
}

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

test("Workbench capability can be derived from the product manifest", () => {
  const manifest = readManifest("workbench");
  const capability = resolveWorkbenchGuiRuntimeCapabilityFromManifest("desktop_webview", manifest);

  assert.equal(capability.canUseRemoteBackend, true);
  assert.equal(capability.canHostOrchestra, false);
  assert.equal(capability.canHostAgent, false);
  assert.equal(capability.canInstallRuntime, false);
  assert.equal(capability.canUseLocalhostBackend, true);
  assert.equal(capability.posture, "local_workstation");
});

test("Installer manifest grants install planning without making GUI a solver host", () => {
  const manifest = readManifest("installer");
  const capability = resolveWorkbenchGuiRuntimeCapabilityFromManifest("desktop_webview", manifest);

  assert.equal(capability.canInstallRuntime, true);
  assert.equal(capability.canHostOrchestra, false);
  assert.equal(capability.canHostAgent, false);
  assert.equal(capability.canUseLocalhostBackend, true);
});

test("manifest capability helpers expose required bindings without leaking runtime topology", () => {
  const manifest = readManifest("workbench");
  const workflowBindings = selectGuiRuntimeManifestBindings(manifest, "workflow.submit");

  assert.deepEqual(workflowBindings.map((binding) => binding.binding_id), ["orchestrated-workflow"]);
  assert.equal(hasGuiRuntimeManifestCapability(manifest, "workflow.submit"), true);
  assert.equal(hasGuiRuntimeManifestCapability(manifest, "operator.catalog.search"), false);
  assert.equal(
    hasGuiRuntimeManifestCapability(manifest, "operator.catalog.search", { includeOptional: true }),
    true,
  );
  assert.ok(listGuiRuntimeManifestCapabilities(manifest).includes("workflow.submit"));
  assert.equal(listGuiRuntimeManifestCapabilities(manifest).includes("operator.catalog.search"), false);
});

test("mobile binding selection hides desktop-only direct mesh capabilities", () => {
  const manifest = readManifest("workbench");

  assert.deepEqual(
    selectGuiRuntimeManifestBindings(manifest, "solver.submit").map((binding) => binding.binding_id),
    ["direct-mesh-solver"],
  );
  assert.deepEqual(
    selectGuiRuntimeManifestBindings(manifest, "solver.submit", { hostKind: "mobile_webview" }),
    [],
  );
});

test("mobile manifest cannot become a local runtime host through frontend capability resolution", () => {
  const manifest = readManifest("mobile-webview");
  const capability = resolveWorkbenchGuiRuntimeCapabilityFromManifest("mobile_webview", manifest);

  assert.equal(capability.canHostOrchestra, false);
  assert.equal(capability.canHostAgent, false);
  assert.equal(capability.canInstallRuntime, false);
  assert.equal(capability.canUseLocalhostBackend, false);
  assert.equal(capability.posture, "remote_control");
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
