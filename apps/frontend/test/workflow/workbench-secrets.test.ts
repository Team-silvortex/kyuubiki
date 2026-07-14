import test from "node:test";
import assert from "node:assert/strict";
import {
  readInMemoryWorkbenchSecrets,
  scrubPersistedWorkbenchSecrets,
  WORKBENCH_SECRETS_KEY,
  writeInMemoryWorkbenchSecrets,
} from "@/lib/workbench/workbench-secrets";
import { persistWorkbenchSettings, sanitizeWorkbenchSettings, type WorkbenchSettingsInput } from "@/lib/workbench/helpers";

function buildSettingsInput(overrides: Partial<WorkbenchSettingsInput> = {}): WorkbenchSettingsInput {
  return {
    theme: "graphite",
    language: "en",
    showShortcutHints: true,
    immersiveGuardrails: true,
    frontendRuntimeMode: "orchestrated_gui",
    directMeshEndpointsText: "solver-a:5001",
    directMeshSelectionMode: "healthiest",
    controlPlaneApiToken: "control-token",
    clusterApiToken: "cluster-token",
    directMeshApiToken: "mesh-token",
    assistantMode: "llm",
    assistantApiBaseUrl: "https://assistant.example.test",
    assistantApiKey: "assistant-key",
    assistantModel: "model-a",
    ...overrides,
  };
}

test("workbench in-memory secrets trim tokens and avoid persisted residue", () => {
  writeInMemoryWorkbenchSecrets({
    controlPlaneApiToken: " control ",
    clusterApiToken: "",
    directMeshApiToken: " mesh ",
    assistantApiKey: " assistant ",
  });

  assert.deepEqual(readInMemoryWorkbenchSecrets(), {
    controlPlaneApiToken: "control",
    directMeshApiToken: "mesh",
    assistantApiKey: "assistant",
  });

  writeInMemoryWorkbenchSecrets({});
  assert.deepEqual(readInMemoryWorkbenchSecrets(), {});
});

test("scrubPersistedWorkbenchSecrets removes legacy session storage secrets", () => {
  const storage = new Map<string, string>([[WORKBENCH_SECRETS_KEY, "{\"controlPlaneApiToken\":\"legacy\"}"]]);
  const previousWindow = globalThis.window;

  globalThis.window = {
    sessionStorage: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
      removeItem: (key: string) => storage.delete(key),
    },
  } as unknown as Window & typeof globalThis;

  try {
    scrubPersistedWorkbenchSecrets();
    assert.equal(storage.has(WORKBENCH_SECRETS_KEY), false);
  } finally {
    globalThis.window = previousWindow;
  }
});

test("sanitizeWorkbenchSettings never persists secret-shaped fields", () => {
  const sanitized = sanitizeWorkbenchSettings(buildSettingsInput());

  assert.equal("controlPlaneApiToken" in sanitized, false);
  assert.equal("clusterApiToken" in sanitized, false);
  assert.equal("directMeshApiToken" in sanitized, false);
  assert.equal("assistantApiKey" in sanitized, false);
});

test("persistWorkbenchSettings stores preferences but keeps secrets memory-only", () => {
  const localStorage = new Map<string, string>();
  const sessionStorage = new Map<string, string>([[WORKBENCH_SECRETS_KEY, "{\"assistantApiKey\":\"legacy\"}"]]);
  const previousWindow = globalThis.window;

  globalThis.window = {
    localStorage: {
      getItem: (key: string) => localStorage.get(key) ?? null,
      setItem: (key: string, value: string) => localStorage.set(key, value),
      removeItem: (key: string) => localStorage.delete(key),
    },
    sessionStorage: {
      getItem: (key: string) => sessionStorage.get(key) ?? null,
      setItem: (key: string, value: string) => sessionStorage.set(key, value),
      removeItem: (key: string) => sessionStorage.delete(key),
    },
  } as unknown as Window & typeof globalThis;

  try {
    persistWorkbenchSettings(buildSettingsInput());
    const persisted = JSON.parse(localStorage.get("kyuubiki-workbench-settings") ?? "{}") as Record<string, unknown>;

    assert.equal("controlPlaneApiToken" in persisted, false);
    assert.equal("clusterApiToken" in persisted, false);
    assert.equal("directMeshApiToken" in persisted, false);
    assert.equal("assistantApiKey" in persisted, false);
    assert.deepEqual(readInMemoryWorkbenchSecrets(), {
      controlPlaneApiToken: "control-token",
      clusterApiToken: "cluster-token",
      directMeshApiToken: "mesh-token",
      assistantApiKey: "assistant-key",
    });
    assert.equal(sessionStorage.has(WORKBENCH_SECRETS_KEY), false);
  } finally {
    writeInMemoryWorkbenchSecrets({});
    globalThis.window = previousWindow;
  }
});
