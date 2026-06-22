import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWorkbenchStorageManifest,
  clearWorkbenchSafeStorage,
  inspectWorkbenchStorage,
  listWorkbenchStorageRules,
} from "../../src/components/workbench/system/workbench-system-storage.ts";

class FakeLocalStorage {
  private values = new Map<string, string>();

  get length() {
    return this.values.size;
  }

  key(index: number) {
    return [...this.values.keys()][index] ?? null;
  }

  getItem(key: string) {
    return this.values.get(key) ?? null;
  }

  setItem(key: string, value: string) {
    this.values.set(key, value);
  }

  removeItem(key: string) {
    this.values.delete(key);
  }
}

function installStorageFixture() {
  const localStorage = new FakeLocalStorage();
  const windowFixture = { localStorage, TextEncoder };
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: windowFixture,
  });
  Object.defineProperty(globalThis, "navigator", {
    configurable: true,
    value: {
      storage: {
        estimate: async () => ({ quota: 100_000, usage: 2_048 }),
      },
    },
  });
  return localStorage;
}

test("workbench storage manifest classifies persisted buckets", async () => {
  const localStorage = installStorageFixture();
  localStorage.setItem("kyuubiki.workbench.workflowLibrary.v1", JSON.stringify([{ id: "workflow-a" }]));
  localStorage.setItem("kyuubiki.workbench.workflowSnapshots.index.v1", JSON.stringify([{ id: "snap-a" }]));
  localStorage.setItem("kyuubiki.unregistered.debug", "leftover");

  const snapshot = await inspectWorkbenchStorage();
  assert.equal(snapshot.unknownKeys, 1);
  assert(snapshot.unknownBytes > 0);

  const manifest = await buildWorkbenchStorageManifest();
  const localWorkflows = manifest.find((entry) => entry.id === "local_workflows");
  const snapshots = manifest.find((entry) => entry.id === "workflow_snapshots");

  assert.equal(localWorkflows?.authority, "workbench");
  assert.equal(localWorkflows?.dataClass, "source_of_truth");
  assert.equal(localWorkflows?.portable, true);
  assert.equal(localWorkflows?.mode, "careful");
  assert.equal(localWorkflows?.entries, 1);
  assert.equal(snapshots?.dataClass, "cache");
  assert.equal(snapshots?.mode, "safe");
});

test("safe storage cleanup preserves careful source-of-truth buckets", async () => {
  const localStorage = installStorageFixture();
  localStorage.setItem("kyuubiki.workbench.workflowLibrary.v1", "authoritative");
  localStorage.setItem("kyuubiki.workbench.workflowSnapshots.index.v1", "cache");
  localStorage.setItem("kyuubiki.workbench.workflowPackageMaintenanceLog.v1", "receipt");

  clearWorkbenchSafeStorage();

  assert.equal(localStorage.getItem("kyuubiki.workbench.workflowLibrary.v1"), "authoritative");
  assert.equal(localStorage.getItem("kyuubiki.workbench.workflowSnapshots.index.v1"), null);
  assert.equal(localStorage.getItem("kyuubiki.workbench.workflowPackageMaintenanceLog.v1"), null);

  const carefulRules = listWorkbenchStorageRules().filter((rule) => rule.mode === "careful");
  assert(carefulRules.some((rule) => rule.dataClass === "source_of_truth"));
});
