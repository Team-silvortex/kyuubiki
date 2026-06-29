import test from "node:test";
import assert from "node:assert/strict";
import {
  readInMemoryWorkbenchSecrets,
  scrubPersistedWorkbenchSecrets,
  WORKBENCH_SECRETS_KEY,
  writeInMemoryWorkbenchSecrets,
} from "@/lib/workbench/workbench-secrets";

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
