import test from "node:test";
import assert from "node:assert/strict";

import type { WorkflowGraphDefinition } from "../../src/lib/api/workflow-types.ts";
import { KYUUBIKI_PRODUCT_VERSION_LABEL } from "../../src/lib/product-version.ts";
import {
  removeStoredLocalWorkflow,
  saveStoredLocalWorkflow,
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
      created.push(saved.id);
      assert.equal(saved.graph.version, `${KYUUBIKI_PRODUCT_VERSION_LABEL} local`);
    }
    assert.notEqual(created[0], created[1]);
  } finally {
    Date.now = originalNow;
    created.forEach(removeStoredLocalWorkflow);
  }
});
