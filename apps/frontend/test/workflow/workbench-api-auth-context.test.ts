import test from "node:test";
import assert from "node:assert/strict";

import { buildWorkbenchApiAuthHeaders } from "@/lib/api/auth-context";
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
