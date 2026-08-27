import test from "node:test";
import assert from "node:assert/strict";

import {
  limitWorkflowCatalogGroups,
  scheduleWorkflowDeferredRender,
  WORKFLOW_CATALOG_RENDER_LIMIT,
} from "../../src/components/workbench/workflow/workbench-workflow-render-budget.ts";
import type { WorkflowCatalogEntry } from "../../src/lib/api/workflow-types.ts";

function catalogEntry(id: string): WorkflowCatalogEntry {
  return {
    id,
    name: id,
    version: "test",
    summary: id,
    entry_inputs: [],
    output_artifacts: [],
  };
}

test("workflow catalog groups share one render budget with pinned entries", () => {
  const pinned = Array.from({ length: 10 }, (_, index) => catalogEntry(`pinned-${index}`));
  const groups = [
    {
      key: "first",
      label: "First",
      entries: Array.from({ length: 50 }, (_, index) => catalogEntry(`first-${index}`)),
    },
    {
      key: "second",
      label: "Second",
      entries: Array.from({ length: 50 }, (_, index) => catalogEntry(`second-${index}`)),
    },
  ];

  const rendered = limitWorkflowCatalogGroups(groups, pinned);

  assert.equal(rendered[0]?.entries.length, 50);
  assert.equal(rendered[1]?.entries.length, WORKFLOW_CATALOG_RENDER_LIMIT - 60);
  assert.equal(groups[1]?.entries.length, 50);
});

test("disposed deferred workflow renders stay suppressed without idle cancellation support", () => {
  let delayedCallback: (() => void) | undefined;
  let idleCallback: (() => void) | undefined;
  let renderCount = 0;
  const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  const originalDocument = Object.getOwnPropertyDescriptor(globalThis, "document");
  Object.defineProperty(globalThis, "document", {
    configurable: true,
    value: { activeElement: null },
  });
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: {
      setTimeout(callback: () => void) {
        delayedCallback = callback;
        return 1;
      },
      clearTimeout() {},
      requestIdleCallback(callback: () => void) {
        idleCallback = callback;
        return 2;
      },
      requestAnimationFrame() {
        return 3;
      },
      cancelAnimationFrame() {},
    },
  });

  try {
    const dispose = scheduleWorkflowDeferredRender(() => {
      renderCount += 1;
    }, 10);
    assert(delayedCallback);
    delayedCallback();
    assert(idleCallback);
    dispose();
    idleCallback();
    assert.equal(renderCount, 0);
  } finally {
    if (originalWindow) Object.defineProperty(globalThis, "window", originalWindow);
    else Reflect.deleteProperty(globalThis, "window");
    if (originalDocument) Object.defineProperty(globalThis, "document", originalDocument);
    else Reflect.deleteProperty(globalThis, "document");
  }
});
