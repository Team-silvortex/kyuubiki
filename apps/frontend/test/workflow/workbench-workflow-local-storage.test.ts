import test from "node:test";
import assert from "node:assert/strict";

import type { WorkflowGraphDefinition } from "../../src/lib/api/workflow-types.ts";
import { KYUUBIKI_PRODUCT_VERSION_LABEL } from "../../src/lib/product-version.ts";
import {
  duplicateStoredLocalWorkflow,
  listStoredLocalWorkflows,
  removeStoredLocalWorkflow,
  renameStoredLocalWorkflow,
  saveStoredLocalWorkflow,
  updateStoredLocalWorkflowMetadata,
  WORKBENCH_LOCAL_WORKFLOWS_KEY,
} from "../../src/components/workbench/workflow/workbench-workflow-local-storage.ts";

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

function graph(id: string): WorkflowGraphDefinition {
  return {
    schema_version: "kyuubiki.workflow/v1",
    id,
    name: id,
    nodes: [],
    edges: [],
  };
}

test("local workflow IDs remain unique and use the current product version", () => {
  const originalNow = Date.now;
  Date.now = () => 1_800_000_000_000;
  const created: string[] = [];
  try {
    for (const suffix of ["a", "b"]) {
      const saved = saveStoredLocalWorkflow({
        sourceWorkflowId: `source-${suffix}`,
        workflowName: `Local ${suffix}`,
        graph: graph(`local-${suffix}`),
      });
      assert.ok(saved);
      created.push(saved.id);
      assert.equal(saved.graph.version, `${KYUUBIKI_PRODUCT_VERSION_LABEL} local`);
    }
    assert.notEqual(created[0], created[1]);
  } finally {
    Date.now = originalNow;
    created.forEach(removeStoredLocalWorkflow);
  }
});

test("legacy local workflows remain visible when sanitized migration cannot write back", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const legacyRecord = {
    id: "workflow.local.legacy",
    sourceWorkflowId: "workflow.source",
    name: "Legacy local workflow",
    summary: "Legacy record",
    version: "local",
    promotedAt: "2026-08-27T00:00:00.000Z",
    graph: graph("workflow.local.legacy"),
    inputArtifactTexts: { model: "legacy-sensitive-input" },
  };
  browserWindow.localStorage = {
    ...createMemoryStorage(),
    getItem: (key) => key === WORKBENCH_LOCAL_WORKFLOWS_KEY ? JSON.stringify([legacyRecord]) : null,
    setItem: () => {
      throw new Error("storage is read-only");
    },
  } as Storage;

  try {
    const records = listStoredLocalWorkflows();
    assert.equal(records.length, 1);
    assert.equal(records[0]?.id, legacyRecord.id);
    assert.equal(records[0]?.inputArtifactTexts, undefined);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});

test("local workflow recovery keeps the newest unique valid records within the storage limit", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const storage = createMemoryStorage();
  const records = Array.from({ length: 45 }, (_, index) => {
    const id = `workflow.local.recovery-${index}`;
    return {
      id,
      sourceWorkflowId: `workflow.source-${index}`,
      name: `Recovery ${index}`,
      summary: "Recovery fixture",
      version: "local",
      promotedAt: new Date(Date.UTC(2026, 0, 1, 0, index)).toISOString(),
      graph: graph(id),
      inputArtifactTexts: index === 44 ? { model: "legacy-sensitive-input" } : undefined,
    };
  });
  storage.setItem(
    WORKBENCH_LOCAL_WORKFLOWS_KEY,
    JSON.stringify([
      ...records,
      { ...records[44], name: "Duplicate older identity" },
      {
        ...records[0],
        id: "workflow.local.invalid-date",
        promotedAt: "not-a-date",
        graph: graph("workflow.local.invalid-date"),
      },
      {
        ...records[0],
        id: "workflow.local.graph-mismatch",
        graph: graph("workflow.local.other"),
      },
      { ...records[0], id: "", graph: graph("") },
    ]),
  );
  browserWindow.localStorage = storage;

  try {
    const recovered = listStoredLocalWorkflows();
    assert.equal(recovered.length, 40);
    assert.equal(new Set(recovered.map((entry) => entry.id)).size, 40);
    assert.equal(recovered[0]?.id, "workflow.local.recovery-44");
    assert.equal(recovered.at(-1)?.id, "workflow.local.recovery-5");
    assert.equal(recovered.some((entry) => entry.id === "workflow.local.invalid-date"), false);
    assert.equal(recovered.some((entry) => entry.id === "workflow.local.graph-mismatch"), false);
    assert.equal(recovered[0]?.inputArtifactTexts, undefined);

    const persisted = JSON.parse(storage.getItem(WORKBENCH_LOCAL_WORKFLOWS_KEY) ?? "[]") as unknown[];
    assert.equal(persisted.length, 40);
    assert.equal(
      persisted.some(
        (entry) => typeof entry === "object" && entry !== null && "inputArtifactTexts" in entry,
      ),
      false,
    );
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});

test("local workflow mutations report write failures without throwing", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const record = {
    id: "workflow.local.read-only",
    sourceWorkflowId: "workflow.source",
    name: "Read-only workflow",
    summary: "Read-only fixture",
    version: "local",
    promotedAt: "2026-08-27T00:00:00.000Z",
    graph: graph("workflow.local.read-only"),
  };
  browserWindow.localStorage = {
    ...createMemoryStorage(),
    getItem: (key) => key === WORKBENCH_LOCAL_WORKFLOWS_KEY ? JSON.stringify([record]) : null,
    setItem: () => {
      throw new Error("quota exceeded");
    },
  } as Storage;

  try {
    assert.equal(
      saveStoredLocalWorkflow({
        sourceWorkflowId: "workflow.source",
        workflowName: "Unsaved workflow",
        graph: graph("workflow.unsaved"),
      }),
      null,
    );
    assert.equal(renameStoredLocalWorkflow(record.id, "Renamed"), false);
    assert.equal(updateStoredLocalWorkflowMetadata(record.id, { notes: "note", summary: "summary" }), false);
    assert.equal(duplicateStoredLocalWorkflow(record.id), null);
    assert.equal(removeStoredLocalWorkflow(record.id), false);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});
