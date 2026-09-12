import assert from "node:assert/strict";
import { test } from "node:test";
import { createWorkbenchModelBatchController } from "../../src/components/workbench/workbench-model-batch-controller";
import { getModelSelectionCopy } from "../../src/components/workbench/model/workbench-model-selection-copy";
import { buildWorkbenchLanguageOptions } from "../../src/components/workbench/workbench-language-options";
import { modelingFixture } from "../support/modeling-fixtures";

function selectionController(count = 6, studyKind = "truss_3d") {
  const model = modelingFixture(count), events: unknown[] = [];
  Object.defineProperty(model, "elements", { get() { assert.fail("selecting nodes must not scan topology"); } });
  const mutation = () => assert.fail("selection must not change models, history or solver results");
  const controller = createWorkbenchModelBatchController({
    studyKind, truss3dModel: model, trussModel: model, frameModel: null!, selectedNode: 1,
    selectedTruss3dNodes: [1, 2], setTruss3dModel: mutation, setTrussModel: mutation, setFrameModel: mutation,
    recordHistory: mutation, resetActiveResult: mutation, historyLabel: "Batch",
    setSelectedTruss3dNodes: (indices) => events.push(["nodes", indices]),
    setSelectedNode: (index) => events.push(["anchor", index]),
    setSelectedElement: (index) => events.push(["element", index]),
    setMemberDraftNodes: (indices) => events.push(["draft", indices]),
  });
  return { controller, events };
}

test("query selection updates actual 3D nodes without touching models, topology, results or history", () => {
  const { controller, events } = selectionController();
  assert.deepEqual(controller.select3d({ kind: "range", axis: "x", min: 1, max: 3 }), { selectedNodes: 3 });
  assert.deepEqual(events, [["nodes", [1, 2, 3]], ["anchor", 1], ["element", null], ["draft", []]]);
  assert.throws(() => controller.select3d({ kind: "all" }), /stale_model/);
  assert.throws(() => controller.apply({ query: { kind: "all" }, operation: { kind: "delete", confirmDelete: true } }), /stale_model/);
});

test("query selection clears safely, inverts current nodes, and rejects invalid queries atomically", () => {
  const cleared = selectionController();
  assert.deepEqual(cleared.controller.select3d({ kind: "indices", indices: [] }), { selectedNodes: 0 });
  assert.deepEqual(cleared.events, [["nodes", []], ["anchor", null], ["element", null], ["draft", []]]);
  const invalid = selectionController();
  assert.throws(() => invalid.controller.select3d({ kind: "indices", indices: [999] }), /invalid_indices/);
  assert.throws(() => invalid.controller.select3d({ kind: "range", axis: "z", min: Infinity, max: 5 }), /invalid_number/);
  assert.deepEqual(invalid.events, []);
  assert.deepEqual(invalid.controller.select3d({ kind: "current", invert: true }), { selectedNodes: 4 });
  assert.deepEqual(invalid.events[0], ["nodes", [0, 3, 4, 5]]);
  for (const kind of ["truss_2d", "frame_2d", "heat_2d"]) {
    const unsupported = selectionController(6, kind);
    assert.throws(() => unsupported.controller.select3d({ kind: "all" }), /unsupported_study/);
    assert.deepEqual(unsupported.events, []);
  }
});

test("query selection handles 200k actual nodes independently of render sampling and connectivity", () => {
  const { controller, events } = selectionController(200_000);
  assert.deepEqual(controller.select3d({ kind: "all" }), { selectedNodes: 200_000 });
  const indices = (events[0] as [string, number[]])[1];
  assert.equal(indices.length, 200_000);
  assert.equal(indices[199_999], 199_999);
});

test("query selection labels cover every shipped locale", () => {
  const english = getModelSelectionCopy("en");
  for (const { value } of buildWorkbenchLanguageOptions({ copy: {}, languagePacks: [], currentLanguage: "en" })) {
    const copy = getModelSelectionCopy(value);
    assert.deepEqual(Object.keys(copy), Object.keys(english));
    assert.ok(Object.values(copy).every((label) => label.trim()));
    if (value !== "en") assert.notEqual(copy.select, english.select, value);
  }
});
