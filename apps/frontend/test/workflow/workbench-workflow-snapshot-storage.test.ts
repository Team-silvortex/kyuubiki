import test from "node:test";
import assert from "node:assert/strict";

import type { WorkflowGraphDefinition } from "../../src/lib/api/workflow-types.ts";
import {
  listStoredWorkflowSnapshots,
  loadStoredWorkflowSnapshot,
  removeStoredWorkflowSnapshot,
  removeStoredWorkflowSnapshotsByWorkflowId,
  saveStoredWorkflowSnapshot,
  WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX,
} from "../../src/components/workbench/workflow/workbench-workflow-snapshot-storage.ts";

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
  value: {
    localStorage: createMemoryStorage(),
    sessionStorage: createMemoryStorage(),
    setTimeout: globalThis.setTimeout.bind(globalThis),
    clearTimeout: globalThis.clearTimeout.bind(globalThis),
  } as unknown as Window,
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
    dispatch_policy: "orchestrated",
    nodes: [{ id: "solve", kind: "solver", config: { gain: 1 } }],
    edges: [],
  };
}

function save(workflowId: string, value: WorkflowGraphDefinition) {
  return saveStoredWorkflowSnapshot({
    workflowId,
    workflowName: workflowId,
    reason: "test",
    graph: value,
    summary: [],
  });
}

test("pending snapshots retain the graph captured at save time", () => {
  const workflowId = "snapshot-immutable";
  try {
    const source = graph(workflowId);
    const saved = save(workflowId, source);
    assert.ok(saved);

    source.name = "mutated after save";
    source.nodes[0]!.config = { gain: 99 };
    const loaded = loadStoredWorkflowSnapshot(saved.id);

    assert.equal(loaded?.graph.name, workflowId);
    assert.deepEqual(loaded?.graph.nodes[0]?.config, { gain: 1 });
  } finally {
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
  }
});

test("snapshot IDs stay unique when multiple captures share one millisecond", () => {
  const workflowId = "snapshot-same-millisecond";
  const originalNow = Date.now;
  Date.now = () => 1_800_000_000_000;
  try {
    const first = save(workflowId, graph(`${workflowId}-a`));
    const second = save(workflowId, graph(`${workflowId}-b`));
    assert.ok(first && second);
    assert.notEqual(first.id, second.id);
  } finally {
    Date.now = originalNow;
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
  }
});

test("snapshot deduplication includes dispatch policy changes", () => {
  const workflowId = "snapshot-dispatch-policy";
  const originalNow = Date.now;
  let now = originalNow();
  Date.now = () => now++;
  try {
    const firstGraph = graph(workflowId);
    const first = save(workflowId, firstGraph);
    const nextGraph = graph(workflowId);
    nextGraph.dispatch_policy = "direct_mesh";
    const second = save(workflowId, nextGraph);
    assert.ok(first && second);
    assert.notEqual(first.id, second.id);
  } finally {
    Date.now = originalNow;
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
  }
});

test("snapshot payload limits use UTF-8 bytes instead of JavaScript characters", () => {
  const workflowId = "snapshot-utf8-size";
  try {
    const largeGraph = graph(workflowId);
    largeGraph.name = "界".repeat(70_000);
    const saved = save(workflowId, largeGraph);

    assert.equal(saved?.payloadState, "summary_only");
  } finally {
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
  }
});

test("summary-only snapshots still deduplicate identical graphs during the cooldown", () => {
  const workflowId = "snapshot-summary-only-deduplication";
  try {
    const largeGraph = graph(workflowId);
    largeGraph.name = "large-snapshot".repeat(20_000);

    const first = save(workflowId, largeGraph);
    const second = save(workflowId, largeGraph);

    assert.equal(first?.payloadState, "summary_only");
    assert.equal(second?.id, first?.id);
  } finally {
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
  }
});

test("failed deferred payload writes downgrade snapshots without throwing", () => {
  const workflowId = "snapshot-deferred-write-failure";
  const browserWindow = window as unknown as {
    localStorage: Storage;
    setTimeout: typeof globalThis.setTimeout;
  };
  const originalStorage = browserWindow.localStorage;
  const originalSetTimeout = browserWindow.setTimeout;
  const backingStorage = createMemoryStorage();
  let deferredWrite: (() => void) | undefined;
  browserWindow.localStorage = {
    ...backingStorage,
    setItem: (key, value) => {
      if (key.startsWith(WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX)) {
        throw new Error("snapshot quota exhausted");
      }
      backingStorage.setItem(key, value);
    },
  } as Storage;
  browserWindow.setTimeout = ((callback: () => void) => {
    deferredWrite = callback;
    return 1;
  }) as unknown as typeof globalThis.setTimeout;

  try {
    const saved = save(workflowId, graph(workflowId));
    assert.equal(saved?.payloadState, "full");
    assert(deferredWrite);
    assert.doesNotThrow(() => deferredWrite?.());

    const [summary] = listStoredWorkflowSnapshots(workflowId);
    assert.equal(summary?.payloadState, "summary_only");
    assert.equal(loadStoredWorkflowSnapshot(saved!.id), null);
  } finally {
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
    browserWindow.localStorage = originalStorage;
    browserWindow.setTimeout = originalSetTimeout;
  }
});

test("failed index commits do not block edits or prune retained snapshots", () => {
  const workflowId = "snapshot-index-write-failure";
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const created = Array.from({ length: 20 }, (_, index) =>
    save(workflowId, graph(`${workflowId}-${index}`)),
  );
  const oldest = created[0];
  assert(oldest);
  browserWindow.localStorage = {
    get length() {
      return originalStorage.length;
    },
    clear: () => originalStorage.clear(),
    getItem: (key) => originalStorage.getItem(key),
    key: (index) => originalStorage.key(index),
    removeItem: (key) => originalStorage.removeItem(key),
    setItem: (key, value) => {
      if (key === "kyuubiki.workbench.workflowSnapshots.index.v1") {
        throw new Error("snapshot index is read-only");
      }
      originalStorage.setItem(key, value);
    },
  };

  try {
    let failedSave: ReturnType<typeof save> | "not-run" = "not-run";
    assert.doesNotThrow(() => {
      failedSave = save(workflowId, graph(`${workflowId}-rejected`));
    });
    assert.equal(failedSave, null);
    assert.equal(listStoredWorkflowSnapshots(workflowId).length, 20);
    assert.equal(loadStoredWorkflowSnapshot(oldest.id)?.graph.id, `${workflowId}-0`);
  } finally {
    browserWindow.localStorage = originalStorage;
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
  }
});

test("failed snapshot deletion index commits keep the snapshot restorable", () => {
  const workflowId = "snapshot-delete-index-failure";
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const saved = save(workflowId, graph(workflowId));
  assert(saved);
  let payloadRemovalCount = 0;
  browserWindow.localStorage = {
    get length() {
      return originalStorage.length;
    },
    clear: () => originalStorage.clear(),
    getItem: (key) => originalStorage.getItem(key),
    key: (index) => originalStorage.key(index),
    removeItem: (key) => {
      if (key.startsWith(WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX)) {
        payloadRemovalCount += 1;
      }
      originalStorage.removeItem(key);
    },
    setItem: (key, value) => {
      if (key === "kyuubiki.workbench.workflowSnapshots.index.v1") {
        throw new Error("snapshot index is read-only");
      }
      originalStorage.setItem(key, value);
    },
  };

  try {
    assert.equal(removeStoredWorkflowSnapshot(saved.id), false);
    assert.equal(payloadRemovalCount, 0);
    assert.equal(listStoredWorkflowSnapshots(workflowId)[0]?.id, saved.id);
    assert.equal(loadStoredWorkflowSnapshot(saved.id)?.graph.id, workflowId);
  } finally {
    browserWindow.localStorage = originalStorage;
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
  }
});

test("snapshot deletion commits the index even when stale payload cleanup fails", () => {
  const workflowId = "snapshot-delete-payload-cleanup-failure";
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const saved = save(workflowId, graph(workflowId));
  assert(saved);
  browserWindow.localStorage = {
    get length() {
      return originalStorage.length;
    },
    clear: () => originalStorage.clear(),
    getItem: (key) => originalStorage.getItem(key),
    key: (index) => originalStorage.key(index),
    removeItem: (key) => {
      if (key.startsWith(WORKBENCH_WORKFLOW_SNAPSHOT_PAYLOAD_PREFIX)) {
        throw new Error("payload cleanup denied");
      }
      originalStorage.removeItem(key);
    },
    setItem: (key, value) => originalStorage.setItem(key, value),
  };

  try {
    assert.equal(removeStoredWorkflowSnapshot(saved.id), true);
    assert.equal(listStoredWorkflowSnapshots(workflowId).length, 0);
    assert.equal(loadStoredWorkflowSnapshot(saved.id), null);
  } finally {
    browserWindow.localStorage = originalStorage;
    removeStoredWorkflowSnapshotsByWorkflowId(workflowId);
  }
});
