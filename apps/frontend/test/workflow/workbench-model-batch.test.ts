import assert from "node:assert/strict";
import { test } from "node:test";
import { applyModelBatch, type ModelBatchOperation } from "../../src/lib/workbench/model-batch-commands";
import { inspectBatchSelection, parseBatchIndices, selectBatchNodes, type BatchModel } from "../../src/lib/workbench/model-batch-selection";
import { createWorkbenchModelBatchController } from "../../src/components/workbench/workbench-model-batch-controller";
import { modelingFixture } from "../support/modeling-fixtures";
import { buildWorkbenchLanguageOptions } from "../../src/components/workbench/workbench-language-options";
import { getModelBatchCopy } from "../../src/components/workbench/model/workbench-model-batch-copy";

const all = { kind: "all" as const };
const run = (model: BatchModel, operation: ModelBatchOperation, indices?: number[]) =>
  applyModelBatch(model, { query: indices ? { kind: "indices", indices } : all, operation });

test("batch selection supports ranges, deduplicated indices, inversion and bounded parsing", () => {
  const model = modelingFixture(8);
  assert.deepEqual(parseBatchIndices("0-3, 2, 7", 8), [0, 1, 2, 3, 7]);
  assert.deepEqual(selectBatchNodes(model, { kind: "range", axis: "x", min: 1, max: 3 }), [1, 2, 3]);
  assert.deepEqual(selectBatchNodes(model, { kind: "indices", indices: [7, 7, 0], invert: true }), [1, 2, 3, 4, 5, 6]);
  assert.deepEqual(selectBatchNodes(model, { kind: "current" }, [6, 6, -1, 999]), [6]);
  assert.deepEqual(inspectBatchSelection(model, { kind: "indices", indices: [2, 3] }), {
    nodes: 2, internalMembers: 1, touchingMembers: 3,
  });
  for (const text of ["", "2-1", "0-999999999999", "-1", "1.5", "NaN", "9"]) {
    assert.throws(() => parseBatchIndices(text, 8), /invalid_indices/);
  }
  assert.throws(() => selectBatchNodes(model, { kind: "indices", indices: [99] }), /invalid_indices/);
  assert.throws(() => selectBatchNodes(model, { kind: "range", axis: "x", min: NaN, max: 3 }), /invalid_number/);
});

test("batch geometry edits preserve undo snapshots, stable IDs and unchanged references", () => {
  const model = modelingFixture(8), before = structuredClone(model);
  const result = run(model, { kind: "translate", offset: { x: 2, y: -1, z: 0.5 } }, [1, 2, 2]);
  assert.equal(result.model.nodes[1].x, 3);
  assert.equal(result.model.nodes[2].z, 2.5);
  assert.equal(result.model.nodes[0], model.nodes[0]);
  assert.equal(result.model.elements, model.elements);
  assert.deepEqual(result.nextSelection, [1, 2]);
  assert.deepEqual(model, before);
  assert.equal(run(model, { kind: "translate", offset: { x: 0, y: 0, z: 0 } }).model, model);
  assert.equal(run(model, { kind: "align", axis: "y", value: 0 }).model, model);
});

test("linear arrays preserve internal connectivity without copying external members or boundary conditions", () => {
  const model = modelingFixture(4);
  model.nodes[0].id = "n4";
  model.elements[0].id = "e3";
  model.nodes[0].load_y = -12;
  const before = structuredClone(model);
  const result = run(model, { kind: "array", copies: 3, offset: { x: 0, y: 2, z: 0 } }, [0, 1]);
  assert.equal(result.model.nodes.length, 10);
  assert.equal(result.model.elements.length, 6);
  assert.equal(new Set(result.model.nodes.map((n) => n.id)).size, 10);
  assert.equal(new Set(result.model.elements.map((e) => e.id)).size, 6);
  assert.deepEqual(result.model.elements.slice(3).map((e) => [e.node_i, e.node_j]), [[4, 5], [6, 7], [8, 9]]);
  assert.equal(result.model.nodes[8].y, 6);
  assert.equal(result.model.nodes[4].fix_x, false);
  assert.equal(result.model.nodes[4].load_y, 0);
  assert.equal(result.model.nodes[0], model.nodes[0]);
  assert.equal(result.model.elements[0], model.elements[0]);
  const copied = run(model, { kind: "array", copies: 1, offset: { x: 0, y: 2 }, copyBoundaryConditions: true }, [0, 1]);
  assert.equal(copied.model.nodes[4].load_y, -12);
  assert.equal(copied.model.nodes[4].fix_x, true);
  assert.deepEqual(model, before);
});

test("batch loads distinguish total load from per-node load and keep supports unchanged", () => {
  const model = modelingFixture(6);
  const total = run(model, { kind: "loads", loads: { x: 12, y: -18, z: 6 }, distribution: "total" });
  assert.deepEqual(total.model.nodes.map((node) => node.load_y), Array(6).fill(-3));
  assert.equal(total.model.nodes.reduce((sum, node) => sum + node.load_x, 0), 12);
  assert.equal(total.model.nodes[0].fix_x, true);
  const perNode = run(model, { kind: "loads", loads: { x: 12, y: -18 }, distribution: "per_node" });
  assert.equal(perNode.model.nodes[5].load_y, -18);
  const supported = run(model, { kind: "supports", axes: ["x", "z"], fixed: true }, [3, 4]);
  assert.equal(supported.model.nodes[3].fix_z, true);
  assert.equal(supported.model.nodes[3].fix_y, false);
});

test("batch member edits use explicit internal/touching scopes and material modulus", () => {
  const model = modelingFixture(6);
  model.materials = [{ id: "steel", name: "Steel", youngs_modulus: 210e9 }];
  const internal = run(model, { kind: "members", scope: "internal", area: 0.2, materialId: "steel" }, [2, 3]);
  assert.equal(internal.summary.affectedMembers, 1);
  assert.equal(internal.model.elements[2].youngs_modulus, 210e9);
  assert.equal(internal.model.elements[2].material_id, "steel");
  assert.equal(internal.model.elements[1], model.elements[1]);
  assert.equal(internal.model.nodes, model.nodes);
  assert.equal(run(model, { kind: "members", scope: "touching", area: 0.2 }, [2, 3]).summary.affectedMembers, 3);
  assert.throws(() => run(model, { kind: "members", scope: "internal", materialId: "missing" }), /missing_material/);
});

test("batch deletion removes incident members and remaps surviving endpoints without renaming IDs", () => {
  const model = modelingFixture(7), before = structuredClone(model);
  const result = run(model, { kind: "delete", confirmDelete: true }, [1, 4]);
  assert.deepEqual(result.model.nodes.map((n) => n.id), ["n0", "n2", "n3", "n5", "n6"]);
  assert.deepEqual(result.model.elements.map((e) => [e.id, e.node_i, e.node_j]), [["e2", 1, 2], ["e5", 3, 4]]);
  assert.equal(result.summary.removedMembers, 4);
  assert.deepEqual(result.nextSelection, []);
  assert.deepEqual(model, before);
  assert.throws(() => run(model, { kind: "delete", confirmDelete: false }), /confirmation_required/);
  assert.equal(run(model, { kind: "delete", confirmDelete: true }).model.nodes.length, 0);
});

test("batch invalid input, excessive arrays and collapsed members fail without mutation", () => {
  const model = modelingFixture(4), before = structuredClone(model);
  for (const operation of [
    { kind: "translate", offset: { x: Infinity, y: 0 } },
    { kind: "members", scope: "internal", area: -1 },
    { kind: "array", offset: { x: 0, y: 0 }, copies: 2 },
    { kind: "array", offset: { x: 1, y: 0 }, copies: 1001 },
    { kind: "array", offset: { x: 1, y: 0 }, copies: 1.5 },
    { kind: "supports", axes: [], fixed: true },
    { kind: "translate", offset: { x: Number.MAX_VALUE, y: Number.MAX_VALUE, z: Number.MAX_VALUE } },
  ] as ModelBatchOperation[]) assert.throws(() => run(model, operation), /model_batch:/);
  const large = modelingFixture(101);
  assert.throws(() => run(large, { kind: "array", copies: 1000, offset: { x: 1, y: 0 } }), /array_limit/);
  assert.throws(() => run(model, { kind: "translate", offset: { x: 1, y: 0, z: 1 } }, [0]), /degenerate_member/);
  assert.deepEqual(model, before);
});

test("same batch commands support 2D trusses and frame rotation constraints without introducing Z", () => {
  const model = modelingFixture(3);
  const planar = { ...model, nodes: model.nodes.map(({ z, fix_z, load_z, ...node }) => node) };
  const moved = run(planar, { kind: "translate", offset: { x: 0, y: 1 } });
  assert.equal("z" in moved.model.nodes[0], false);
  assert.throws(() => run(planar, { kind: "supports", axes: ["z"], fixed: true }), /unsupported_axis/);
  assert.throws(() => run(planar, { kind: "align", axis: "z", value: 0 }), /unsupported_axis/);
  assert.throws(() => run(planar, { kind: "align", axis: "x", value: 0 }), /degenerate_member/);
  const frame = { ...planar, nodes: planar.nodes.map((node) => ({ ...node, fix_rz: false, moment_z: 12 })) };
  const loadedFrame = { ...frame, elements: frame.elements.map((element) => ({ ...element, distributed_load_y: -50 })) };
  assert.ok(run(frame, { kind: "supports", axes: ["rz"], fixed: true }).model.nodes.every((n) => n.fix_rz));
  const array = run(frame, { kind: "array", copies: 1, offset: { x: 0, y: 1 } });
  assert.equal(array.model.nodes[3].moment_z, 0);
  assert.equal(array.model.nodes[0].moment_z, 12);
  const unloaded = run(loadedFrame, { kind: "array", copies: 1, offset: { x: 0, y: 1 } });
  assert.equal(unloaded.model.elements[0].distributed_load_y, -50);
  assert.equal(unloaded.model.elements[2].distributed_load_y, 0);
  const retained = run(loadedFrame, { kind: "array", copies: 1, offset: { x: 0, y: 1 }, copyBoundaryConditions: true });
  assert.equal(retained.model.elements[2].distributed_load_y, -50);
});

test("batch controller records one history entry and resets results only after successful edits", () => {
  const model = modelingFixture(4);
  const events: string[] = [];
  let current = model;
  const controller = createWorkbenchModelBatchController({
    studyKind: "truss_3d", truss3dModel: model, trussModel: model, frameModel: null!, selectedNode: 1,
    selectedTruss3dNodes: [1, 2], setTruss3dModel: (next) => { current = next; events.push("model"); },
    setTrussModel: () => assert.fail("wrong model"), setFrameModel: () => assert.fail("wrong model"),
    recordHistory: () => events.push("history"), resetActiveResult: () => events.push("reset"),
    setSelectedNode: () => {}, setSelectedElement: () => {}, setSelectedTruss3dNodes: () => {},
    setMemberDraftNodes: () => {}, historyLabel: "Batch",
  });
  assert.equal(controller.inspect(all).nodes, 4);
  controller.apply({ query: all, operation: { kind: "translate", offset: { x: 0, y: 0 } } });
  assert.throws(() => controller.apply({ query: all, operation: { kind: "delete", confirmDelete: false } }));
  assert.deepEqual(events, []);
  controller.apply({ query: { kind: "current" }, operation: { kind: "translate", offset: { x: 0, y: 2 } } });
  assert.deepEqual(events, ["history", "reset", "model"]);
  assert.equal(current.nodes[2].y, 2);
  assert.equal(model.nodes[2].y, 0);
  assert.throws(() => controller.apply({ query: all, operation: { kind: "translate", offset: { x: 1, y: 0 } } }), /stale_model/);
  assert.throws(() => controller.inspect(all), /stale_model/);
  assert.deepEqual(events, ["history", "reset", "model"]);
});

test("batch selection and editing stay bounded on a 100k-node input", () => {
  const model = modelingFixture(100_000);
  let reads = 0;
  model.elements = model.elements.map((member) => ({ ...member, get node_i() { reads++; return member.node_i; } }));
  const result = applyModelBatch(model, {
    query: { kind: "range", axis: "x", min: 25, max: 74 },
    operation: { kind: "members", scope: "internal", area: 0.05 },
  });
  assert.equal(result.summary.selectedNodes, 50_000);
  assert.equal(result.summary.affectedMembers, 49_000);
  assert.ok(reads < model.elements.length * 4, `unexpected nested scan: ${reads}`);
});

test("batch array rejects precision collapse rather than producing zero-length copied members", () => {
  const model = modelingFixture(2);
  model.nodes[1].z = 0;
  const original = structuredClone(model);
  assert.throws(() => run(model, { kind: "array", copies: 1, offset: { x: 1e20, y: 0, z: 0 } }), /degenerate_member/);
  assert.deepEqual(model, original);
});

test("batch modeling labels cover every shipped language option", () => {
  const english = getModelBatchCopy("en");
  const languages = buildWorkbenchLanguageOptions({ copy: {}, languagePacks: [], currentLanguage: "en" });
  for (const { value } of languages) {
    const translated = getModelBatchCopy(value);
    assert.deepEqual(Object.keys(translated), Object.keys(english));
    assert.ok(Object.values(translated).every((entry) => typeof entry === "string" && entry.trim().length > 0), value);
    if (value !== "en") assert.notEqual(translated.title, english.title, `${value} fell back to English`);
  }
});
