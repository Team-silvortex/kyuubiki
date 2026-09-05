import test from "node:test";
import assert from "node:assert/strict";

import type { WorkflowGraphDefinition } from "../../src/lib/api/workflow-types.ts";
import {
  removeStoredWorkflowSnapshotsByWorkflowId,
  saveStoredWorkflowSnapshot,
} from "../../src/components/workbench/workflow/workbench-workflow-snapshot-storage.ts";

const originalWindowDescriptor = Object.getOwnPropertyDescriptor(globalThis, "window");
let writeCount = 0;
let readMode: "empty" | "throw" = "empty";
Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: {
    localStorage: {
      getItem: () => {
        if (readMode === "empty") return "";
        throw new Error("snapshot index unavailable");
      },
      setItem: () => {
        writeCount += 1;
      },
    },
    sessionStorage: { getItem: () => null, setItem: () => undefined },
    setTimeout: globalThis.setTimeout.bind(globalThis),
    clearTimeout: globalThis.clearTimeout.bind(globalThis),
  } as unknown as Window,
});

test.after(() => {
  if (originalWindowDescriptor) Object.defineProperty(globalThis, "window", originalWindowDescriptor);
  else Reflect.deleteProperty(globalThis, "window");
});

const graph: WorkflowGraphDefinition = {
  schema_version: "kyuubiki.workflow/v1",
  id: "snapshot-unreadable",
  nodes: [{ id: "solve", kind: "solver" }],
  edges: [],
};

test("snapshot mutations do not overwrite an unreadable index", () => {
  assert.equal(saveStoredWorkflowSnapshot({
    workflowId: graph.id,
    workflowName: graph.id,
    reason: "unreadable index test",
    graph,
    summary: [],
  }), null);
  readMode = "throw";
  assert.equal(removeStoredWorkflowSnapshotsByWorkflowId(graph.id), false);
  assert.equal(writeCount, 0);
});
