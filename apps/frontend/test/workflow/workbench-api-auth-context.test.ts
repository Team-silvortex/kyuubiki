import test from "node:test";
import assert from "node:assert/strict";

import { buildWorkbenchApiAuthHeaders, captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import { writeInMemoryWorkbenchSecrets } from "@/lib/workbench/workbench-secrets";

function installWindow(settings: Record<string, unknown> | null) {
  const previousWindow = globalThis.window;
  globalThis.window = {
    localStorage: {
      getItem: (key: string) =>
        key === "kyuubiki-workbench-settings" && settings ? JSON.stringify(settings) : null,
      setItem: () => undefined,
      removeItem: () => undefined,
    },
  } as unknown as Window & typeof globalThis;
  return () => {
    globalThis.window = previousWindow;
  };
}

test("API auth context stays empty outside browser GUI state", () => {
  const previousWindow = globalThis.window;
  globalThis.window = undefined as unknown as Window & typeof globalThis;
  writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "control" });

  try {
    assert.deepEqual(buildWorkbenchApiAuthHeaders("/api/health"), {});
  } finally {
    writeInMemoryWorkbenchSecrets({});
    globalThis.window = previousWindow;
  }
});

test("API auth context attaches control-plane token for orchestrated routes", () => {
  const restoreWindow = installWindow({ frontendRuntimeMode: "orchestrated_gui" });
  writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "control", directMeshApiToken: "mesh" });

  try {
    assert.deepEqual(buildWorkbenchApiAuthHeaders("/api/health"), { "x-kyuubiki-token": "control" });
    assert.deepEqual(buildWorkbenchApiAuthHeaders("/api/direct-mesh/agents"), {});
  } finally {
    writeInMemoryWorkbenchSecrets({});
    restoreWindow();
  }
});

test("API auth context attaches direct-mesh token only for direct mesh GUI routes", () => {
  const restoreWindow = installWindow({ frontendRuntimeMode: "direct_mesh_gui" });
  writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "control", directMeshApiToken: "mesh" });

  try {
    assert.deepEqual(buildWorkbenchApiAuthHeaders("/api/direct-mesh/agents"), { "x-kyuubiki-token": "mesh" });
    assert.deepEqual(buildWorkbenchApiAuthHeaders("/api/health"), {});
  } finally {
    writeInMemoryWorkbenchSecrets({});
    restoreWindow();
  }
});

test("API authority compares the effective URL rather than an unrelated setting", () => {
  const settings = { backendApiBaseUrl: "https://backend-a.invalid", frontendRuntimeMode: "orchestrated_gui", theme: "graphite" };
  const restore = installWindow(settings);
  try {
    const current = captureWorkbenchApiAuthority("/api/v1/jobs", "/api/v1/results");
    settings.theme = "paper";
    assert.equal(current(), true);
    settings.backendApiBaseUrl = "https://backend-b.invalid";
    assert.equal(current(), false);
  } finally { restore(); }
});

test("API authority rejects rotation of the relevant in-memory credential", () => {
  const restore = installWindow({ frontendRuntimeMode: "orchestrated_gui" });
  writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "old-test-token", directMeshApiToken: "unused-token" });
  try {
    const current = captureWorkbenchApiAuthority("/api/v1/jobs");
    writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "old-test-token", directMeshApiToken: "other-unused-token" });
    assert.equal(current(), true);
    writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "new-test-token" });
    assert.equal(current(), false);
  } finally { writeInMemoryWorkbenchSecrets({}); restore(); }
});

test("API authority follows query precedence and detects governed header removal", () => {
  const settings = { backendApiBaseUrl: "https://stored.invalid", frontendRuntimeMode: "orchestrated_gui" };
  const restore = installWindow(settings);
  const location = { search: "?kyuubikiApiBaseUrl=https%3A%2F%2Fquery.invalid" };
  Object.assign(globalThis.window, { location });
  writeInMemoryWorkbenchSecrets({ controlPlaneApiToken: "test-token" });
  try {
    const current = captureWorkbenchApiAuthority("/api/v1/results");
    settings.backendApiBaseUrl = "https://ignored.invalid";
    assert.equal(current(), true);
    settings.frontendRuntimeMode = "direct_mesh_gui";
    assert.equal(current(), false);
    settings.frontendRuntimeMode = "orchestrated_gui";
    const next = captureWorkbenchApiAuthority("/api/v1/results");
    location.search = "?kyuubikiApiBaseUrl=https%3A%2F%2Fother-query.invalid";
    assert.equal(next(), false);
  } finally { writeInMemoryWorkbenchSecrets({}); restore(); }
});

for (const initial of ["direct_mesh_gui", "orchestrated_gui"]) {
  test(`API authority fences a token-free mode switch from ${initial}`, () => {
    const settings = { frontendRuntimeMode: initial };
    const restore = installWindow(settings);
    writeInMemoryWorkbenchSecrets({});
    try {
      const current = captureWorkbenchApiAuthority("/api/direct-mesh/tasks/inspect", "/api/v1/jobs");
      assert.equal(current(), true);
      settings.frontendRuntimeMode = initial === "direct_mesh_gui" ? "orchestrated_gui" : "direct_mesh_gui";
      assert.equal(current(), false, "mode ownership must not depend on configured tokens");
    } finally { restore(); }
  });
}

test("an observed authority change cannot revive an old response after restoring the gateway", () => {
  const settings = { frontendRuntimeMode: "direct_mesh_gui", backendApiBaseUrl: "https://original.invalid" };
  const restore = installWindow(settings);
  try {
    const current = captureWorkbenchApiAuthority("/api/direct-mesh/tasks/inspect");
    settings.backendApiBaseUrl = "https://replacement.invalid";
    assert.equal(current(), false);
    settings.backendApiBaseUrl = "https://original.invalid";
    assert.equal(current(), false);
    assert.equal(captureWorkbenchApiAuthority("/api/direct-mesh/tasks/inspect")(), true);
  } finally { restore(); }
});

test("unreadable authority revokes the old request even after storage recovers", () => {
  const restore = installWindow({ frontendRuntimeMode: "direct_mesh_gui" });
  try {
    const current = captureWorkbenchApiAuthority("/api/direct-mesh/tasks/inspect");
    const read = window.localStorage.getItem;
    window.localStorage.getItem = () => { throw new Error("storage unavailable"); };
    assert.throws(current, /BACKEND_TARGET_UNAVAILABLE/);
    window.localStorage.getItem = read;
    assert.equal(current(), false);
    assert.equal(captureWorkbenchApiAuthority("/api/direct-mesh/tasks/inspect")(), true);
  } finally { restore(); }
});
