import test from "node:test";
import assert from "node:assert/strict";

import {
  buildWorkflowPackageMaintenanceLogEntryId,
  listStoredWorkflowPackageMaintenanceHistory,
  saveStoredWorkflowPackageMaintenanceHistory,
  WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY,
} from "../../src/components/workbench/workflow/workbench-workflow-package-maintenance-log.ts";

function createMemoryStorage(): Storage {
  const records = new Map<string, string>();
  return {
    get length() {
      return records.size;
    },
    clear: () => records.clear(),
    getItem: (key) => records.get(key) ?? null,
    key: (index) => [...records.keys()][index] ?? null,
    removeItem: (key) => records.delete(key),
    setItem: (key, value) => records.set(key, String(value)),
  };
}

const originalWindowDescriptor = Object.getOwnPropertyDescriptor(globalThis, "window");
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: { localStorage: createMemoryStorage() } as unknown as Window,
});

test.after(() => {
  if (originalWindowDescriptor) Object.defineProperty(globalThis, "window", originalWindowDescriptor);
  else Reflect.deleteProperty(globalThis, "window");
});

test("maintenance log IDs remain unique inside one millisecond", () => {
  const originalNow = Date.now;
  Date.now = () => 1_800_000_000_000;
  try {
    assert.notEqual(
      buildWorkflowPackageMaintenanceLogEntryId("scan"),
      buildWorkflowPackageMaintenanceLogEntryId("scan"),
    );
  } finally {
    Date.now = originalNow;
  }
});

test("maintenance history recovery deduplicates and enforces the per-workflow limit", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const storage = createMemoryStorage();
  const workflowId = "workflow.maintenance";
  const entries = Array.from({ length: 15 }, (_, index) => ({
    id: `scan:${index}`,
    workflowId,
    at: new Date(Date.UTC(2026, 0, 1, 0, index)).toISOString(),
    kind: "scan",
    lines: [`line ${index}`],
  }));
  storage.setItem(
    WORKBENCH_WORKFLOW_PACKAGE_MAINTENANCE_LOG_KEY,
    JSON.stringify([
      ...entries,
      { ...entries[14], lines: ["duplicate"] },
      { ...entries[0], id: "invalid-date", at: "not-a-date" },
    ]),
  );
  browserWindow.localStorage = storage;

  try {
    const recovered = listStoredWorkflowPackageMaintenanceHistory(workflowId);
    assert.equal(recovered.length, 12);
    assert.equal(new Set(recovered.map((entry) => entry.id)).size, 12);
    assert.equal(recovered[0]?.id, "scan:14");
    assert.equal(recovered.at(-1)?.id, "scan:3");
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});

test("maintenance history remains usable when persistence is read-only", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  browserWindow.localStorage = {
    ...createMemoryStorage(),
    setItem: () => {
      throw new Error("quota exceeded");
    },
  } as Storage;

  try {
    assert.equal(saveStoredWorkflowPackageMaintenanceHistory("workflow.read-only", [{
      id: "scan:read-only",
      at: "2026-08-28T00:00:00.000Z",
      kind: "scan",
      lines: ["scan completed"],
    }]), false);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});

test("maintenance history does not overwrite an unreadable store", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  let writeCount = 0;
  browserWindow.localStorage = {
    ...createMemoryStorage(),
    getItem: () => {
      throw new Error("storage unavailable");
    },
    setItem: () => {
      writeCount += 1;
    },
  } as Storage;

  try {
    assert.equal(saveStoredWorkflowPackageMaintenanceHistory("workflow-a", [{
      id: "scan:new",
      at: "2026-08-28T00:00:00.000Z",
      kind: "scan",
      lines: ["new receipt"],
    }]), false);
    assert.equal(writeCount, 0);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});
