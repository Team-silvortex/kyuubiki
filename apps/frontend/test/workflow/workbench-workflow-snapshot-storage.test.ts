import test from "node:test";
import assert from "node:assert/strict";

import type { WorkflowGraphDefinition } from "../../src/lib/api/workflow-types.ts";
import {
  loadStoredWorkflowSnapshot,
  removeStoredWorkflowSnapshotsByWorkflowId,
  saveStoredWorkflowSnapshot,
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
