import test from "node:test";
import assert from "node:assert/strict";
import { modelingFixture } from "../support/modeling-fixtures";

import {
  buildWorkbenchSnapshot,
  canRetainWorkbenchSnapshotBinding,
  createAssistantTransactionEntry,
  pushHistoryEntry,
  restoreWorkbenchSnapshot,
  type WorkbenchSnapshot,
} from "../../src/lib/workbench/history.ts";

function snapshot(marker: number): WorkbenchSnapshot {
  return {
    memberDraftNodes: [marker],
  } as unknown as WorkbenchSnapshot;
}

test("history capacity one retains only the newest entry", () => {
  const initial = [
    { label: "first", snapshot: snapshot(1) },
    { label: "second", snapshot: snapshot(2) },
  ];

  const next = pushHistoryEntry(initial, "third", snapshot(3), 1);

  assert.equal(next.length, 1);
  assert.equal(next[0]?.label, "third");
});

test("assistant transaction IDs remain unique inside one millisecond", () => {
  const originalNow = Date.now;
  Date.now = () => 1_700_000_000_000;
  try {
    const first = createAssistantTransactionEntry("first", ["study/run"], snapshot(1));
    const second = createAssistantTransactionEntry("second", ["study/run"], snapshot(2));

    assert.notEqual(first.id, second.id);
  } finally {
    Date.now = originalNow;
  }
});

test("assistant transaction audit actions are isolated from caller mutation", () => {
  const actions = ["study/run"];
  const entry = createAssistantTransactionEntry("run", actions, snapshot(1));

  actions.push("project/delete");

  assert.deepEqual(entry.executedActions, ["study/run"]);
});

test("history snapshot owns its save provenance without deep-copying immutable model geometry", () => {
  const source = { ...snapshot(1), savedModelContext: { projectId: "project", modelId: "model" },
    heatBarModel: { nodes: [], elements: [] } };
  const captured = buildWorkbenchSnapshot(source);
  source.savedModelContext.modelId = "different-model";
  assert.deepEqual(captured.savedModelContext, { projectId: "project", modelId: "model" });
  assert.equal(captured.heatBarModel, source.heatBarModel);
});

test("history restores independent 3D multi-selection and drops stale indices", () => {
  const source: WorkbenchSnapshot = { ...snapshot(1), selectedTruss3dNodes: [1, 2], truss3dModel: modelingFixture(3) };
  const captured = buildWorkbenchSnapshot(source);
  source.selectedTruss3dNodes![0] = 0;
  assert.deepEqual(captured.selectedTruss3dNodes, [1, 2]);
  assert.equal(captured.truss3dModel, source.truss3dModel);
  let selection: number[] = [];
  const setters = new Proxy({ setSelectedTruss3dNodes: (value: number[]) => { selection = value; } }, {
    get: (target, key) => key === "setSelectedTruss3dNodes" ? target.setSelectedTruss3dNodes : () => {},
  }) as unknown as Parameters<typeof restoreWorkbenchSnapshot>[1];
  restoreWorkbenchSnapshot(captured, setters);
  assert.deepEqual(selection, [1, 2]);
  restoreWorkbenchSnapshot({ ...captured, selectedTruss3dNodes: [1, -1, 90, NaN] }, setters);
  assert.deepEqual(selection, [1]);
  restoreWorkbenchSnapshot({ ...captured, selectedTruss3dNodes: undefined }, setters);
  assert.deepEqual(selection, []);
});

for (const scenario of ["same", "project", "model", "study", "unsaved", "legacy", "null-context"]) {
  test(`history save binding handles ${scenario} provenance without adopting another model`, () => {
    const captured: WorkbenchSnapshot = { ...snapshot(1), studyKind: "heat_bar_1d",
      savedModelContext: { projectId: "project", modelId: "model" } };
    const current = { projectId: "project", modelId: "model", studyKind: "heat_bar_1d" as const };
    if (scenario === "project") captured.savedModelContext!.projectId = "other-project";
    if (scenario === "model") captured.savedModelContext!.modelId = "other-model";
    if (scenario === "study") captured.studyKind = "thermal_bar_1d";
    if (scenario === "unsaved") captured.savedModelContext!.modelId = null;
    if (scenario === "legacy") delete captured.savedModelContext;
    if (scenario === "null-context") (captured as any).savedModelContext = null;
    assert.equal(canRetainWorkbenchSnapshotBinding(captured, current), scenario === "same");
  });
}
