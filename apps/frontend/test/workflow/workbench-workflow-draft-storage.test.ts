import test from "node:test";
import assert from "node:assert/strict";

import type { WorkflowGraphDefinition } from "../../src/lib/api/workflow-types.ts";
import {
  listStoredWorkflowDrafts,
  removeStoredWorkflowDraft,
  saveStoredWorkflowDraft,
} from "../../src/components/workbench/workflow/workbench-workflow-draft-storage.ts";

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
    schema_version: "kyuubiki.workflow-graph/v1",
    id,
    name: id,
    nodes: [{ id: "solve", kind: "solve", config: { gain: 1 } }],
    edges: [],
  };
}

test("draft IDs stay unique when consecutive saves share one millisecond", () => {
  const workflowId = "workflow.draft-id";
  const originalNow = Date.now;
  Date.now = () => 1_800_000_000_000;
  try {
    const first = saveStoredWorkflowDraft({ workflowId, workflowName: "Draft", graph: graph("draft-a") });
    const second = saveStoredWorkflowDraft({ workflowId, workflowName: "Draft", graph: graph("draft-b") });

    assert.notEqual(first.id, second.id);
    assert.equal(listStoredWorkflowDrafts(workflowId).length, 2);
  } finally {
    Date.now = originalNow;
    listStoredWorkflowDrafts(workflowId).forEach((draft) => removeStoredWorkflowDraft(draft.id));
  }
});

test("saved drafts own their graph and template preference state", () => {
  const workflowId = "workflow.draft-ownership";
  const source = graph("draft-owned");
  const preferences = {
    favoriteChainIds: ["chain.a"],
    favoriteChainAliases: { "chain.a": "Primary" },
  };
  try {
    const saved = saveStoredWorkflowDraft({
      workflowId,
      workflowName: "Draft",
      graph: source,
      templateChainPreferences: preferences,
    });
    source.nodes[0]!.config = { gain: 99 };
    preferences.favoriteChainIds.push("chain.b");
    preferences.favoriteChainAliases["chain.a"] = "Mutated";

    assert.deepEqual(saved.graph.nodes[0]?.config, { gain: 1 });
    assert.deepEqual(saved.templateChainPreferences, {
      favoriteChainIds: ["chain.a"],
      favoriteChainAliases: { "chain.a": "Primary" },
    });
  } finally {
    listStoredWorkflowDrafts(workflowId).forEach((draft) => removeStoredWorkflowDraft(draft.id));
  }
});

test("legacy drafts remain visible when sanitized migration cannot write back", () => {
  const browserWindow = window as unknown as { localStorage: Storage };
  const originalStorage = browserWindow.localStorage;
  const workflowId = "workflow.legacy-draft";
  const legacyRecord = {
    id: "draft_legacy",
    workflowId,
    name: "Legacy draft",
    savedAt: "2026-08-27T00:00:00.000Z",
    graph: graph(workflowId),
    inputArtifactTexts: { model: "legacy-sensitive-input" },
  };
  browserWindow.localStorage = {
    ...createMemoryStorage(),
    getItem: (key) => key === "kyuubiki.workbench.workflowDrafts.v1" ? JSON.stringify([legacyRecord]) : null,
    setItem: () => {
      throw new Error("storage is read-only");
    },
  } as Storage;

  try {
    const records = listStoredWorkflowDrafts(workflowId);
    assert.equal(records.length, 1);
    assert.equal(records[0]?.id, legacyRecord.id);
    assert.equal(records[0]?.inputArtifactTexts, undefined);
  } finally {
    browserWindow.localStorage = originalStorage;
  }
});
