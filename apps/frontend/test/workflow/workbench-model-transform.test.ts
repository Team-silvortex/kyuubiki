import assert from "node:assert/strict";
import { test } from "node:test";
import { applyModelBatch, type ModelBatchOperation } from "../../src/lib/workbench/model-batch-commands";
import { cloneTruss3dSelectedNodes } from "../../src/lib/workbench/truss3d-commands";
import type { BatchAxis, BatchModel } from "../../src/lib/workbench/model-batch-selection";
import type { BatchPivot } from "../../src/lib/workbench/model-batch-geometry";
import { createWorkbenchModelBatchController } from "../../src/components/workbench/workbench-model-batch-controller";
import { createWorkbenchStructureEditController } from "../../src/components/workbench/workbench-structure-edit-controller";
import { buildWorkbenchLanguageOptions } from "../../src/components/workbench/workbench-language-options";
import { getModelTransformCopy } from "../../src/components/workbench/model/workbench-model-transform-copy";
import { modelingFixture } from "../support/modeling-fixtures";

const origin = { kind: "origin" as const }, all = { kind: "all" as const };
const run = (model: BatchModel, operation: ModelBatchOperation, indices?: number[]) =>
  applyModelBatch(model, { query: indices ? { kind: "indices", indices } : all, operation });
const xyz = (node: BatchModel["nodes"][number]) => [node.x, node.y, node.z];
const close = (actual: number, expected: number, tolerance = 1e-10) =>
  assert.ok(Math.abs(actual - expected) <= tolerance * Math.max(1, Math.abs(expected)), `${actual} != ${expected}`);

test("geometry rotation follows right-hand axes with exact quarter turns and stable identities", () => {
  const model = modelingFixture(1);
  Object.assign(model.nodes[0], { x: 1, y: 2, z: 3, load_x: 100, fix_z: true });
  for (const [axis, expected] of [["x", [1, -3, 2]], ["y", [3, 2, -1]], ["z", [-2, 1, 3]]] as const) {
    const result = run(model, { kind: "rotate", axis, angleDegrees: 90, pivot: origin });
    assert.deepEqual(xyz(result.model.nodes[0]), expected);
    assert.equal(result.model.nodes[0].load_x, 100);
    assert.equal(result.model.nodes[0].fix_z, true);
    assert.equal(result.model.nodes[0].id, model.nodes[0].id);
    assert.equal(result.model.elements, model.elements);
    const restored = run(result.model, { kind: "rotate", axis, angleDegrees: -90, pivot: origin });
    assert.deepEqual(restored.model, model);
  }
  assert.equal(run(model, { kind: "rotate", axis: "z", angleDegrees: 360, pivot: origin }).model, model);
  const tiny = modelingFixture(1);
  tiny.nodes[0].x = 1e15;
  assert.ok(run(tiny, { kind: "rotate", axis: "z", angleDegrees: 1e-14, pivot: origin }).model.nodes[0].y > 0.1);
  assert.ok(run(tiny, { kind: "rotate", axis: "z", angleDegrees: -1e-14, pivot: origin }).model.nodes[0].y < -0.1);
});

test("geometry transforms distinguish selection bounds and explicit pivots without scaling sections", () => {
  const model = modelingFixture(3);
  model.nodes.forEach((node, index) => { node.x = [1, 2, 9][index]; node.y = 2; node.z = 3; });
  const before = structuredClone(model);
  const mirrored = run(model, { kind: "mirror", axis: "x", pivot: { kind: "selection" } });
  assert.deepEqual(mirrored.model.nodes.map((node) => node.x), [9, 8, 1], "bounds center is 5, not the centroid 4");
  const pivot: BatchPivot = { kind: "point", point: { x: 1, y: 1, z: 1 } };
  const scaled = run(model, { kind: "scale", factors: { x: 2, y: 3, z: 4 }, pivot });
  assert.deepEqual(xyz(scaled.model.nodes[2]), [17, 4, 9]);
  assert.equal(scaled.model.elements, model.elements, "section, material and topology must not scale implicitly");
  const rotated = run(model, { kind: "rotate", axis: "z", angleDegrees: 90, pivot });
  assert.deepEqual(xyz(rotated.model.nodes[0]), [0, 1, 3]);
  assert.deepEqual(model, before);
  assert.equal(run(model, { kind: "scale", factors: { x: 1, y: 1, z: 1 }, pivot }).model, model);
  const huge = modelingFixture(1);
  huge.nodes[0].x = 1.5e308;
  assert.equal(run(huge, { kind: "mirror", axis: "x", pivot: { kind: "selection" } }).model, huge);
});

test("transform copies retain internal connectivity, collision-free IDs and explicit boundary policy", () => {
  const model = modelingFixture(4);
  model.nodes[0].id = "n4"; model.nodes[0].load_y = -50; model.elements[0].id = "e3";
  for (const operation of [
    { kind: "rotate", axis: "y", angleDegrees: 45, pivot: origin },
    { kind: "scale", factors: { x: 2, y: 2, z: 2 }, pivot: origin },
    { kind: "mirror", axis: "x", pivot: origin },
  ] as const) {
    const result = run(model, { ...operation, copy: true }, [0, 1, 1]);
    assert.deepEqual(result.nextSelection, [4, 5]);
    assert.equal(result.summary.addedMembers, 1);
    assert.equal(new Set(result.model.nodes.map((node) => node.id)).size, 6);
    assert.equal(new Set(result.model.elements.map((member) => member.id)).size, 4);
    assert.equal(result.model.elements[0], model.elements[0]);
    assert.equal(result.model.nodes[0], model.nodes[0]);
    assert.deepEqual([result.model.elements[3].node_i, result.model.elements[3].node_j], [4, 5]);
    assert.equal(result.model.nodes[4].fix_x, false);
    assert.equal(result.model.nodes[4].load_y, 0);
    const retained = run(model, { ...operation, copy: true, copyBoundaryConditions: true }, [0, 1]);
    assert.equal(retained.model.nodes[4].fix_x, true);
    assert.equal(retained.model.nodes[4].load_y, -50);
  }
  assert.throws(() => run(model, { kind: "mirror", axis: "x", pivot: origin, copyBoundaryConditions: true }), /copy_required/);
  const large = modelingFixture(100_001);
  assert.throws(() => run(large, { kind: "mirror", axis: "x", pivot: origin, copy: true }), /array_limit/);
});

test("grid snap is symmetric, axis-scoped, idempotent and rejects collapsed members", () => {
  const model = modelingFixture(4);
  model.nodes.forEach((node, i) => { node.x = [-1.5, -0.5, 0.5, 1.5][i]; node.y = 0.2; node.z = 0.3; });
  const snapped = run(model, { kind: "snap", axes: ["x", "x"], spacing: 1 });
  assert.deepEqual(snapped.model.nodes.map((node) => node.x), [-2, -1, 1, 2]);
  assert.equal(snapped.model.nodes[1].y, 0.2);
  assert.equal(snapped.model.nodes[1].z, 0.3);
  assert.equal(run(snapped.model, { kind: "snap", axes: ["x"], spacing: 1 }).model, snapped.model);
  const line = modelingFixture(2);
  line.nodes[1].z = 0; line.nodes[1].x = 0.2;
  const before = structuredClone(line);
  assert.throws(() => run(line, { kind: "snap", axes: ["x"], spacing: 1 }), /degenerate_member/);
  assert.deepEqual(line, before);
  line.nodes[0].x = 1e20;
  assert.throws(() => run(line, { kind: "snap", axes: ["x"], spacing: 1 }), /grid_precision/);
});

test("geometry guards invalid parameters, dimensions, precision and incident-member collapse atomically", () => {
  const model = modelingFixture(2);
  Object.assign(model.nodes[0], { x: 1, y: 0, z: 0 });
  Object.assign(model.nodes[1], { x: -1, y: 0, z: 0 });
  const before = structuredClone(model);
  for (const operation of [
    { kind: "rotate", axis: "z", angleDegrees: NaN, pivot: origin },
    { kind: "rotate", axis: "bad", angleDegrees: 90, pivot: origin },
    { kind: "mirror", axis: "x", pivot: { kind: "wrong" } },
    { kind: "mirror", axis: "x", pivot: { kind: "point", point: { x: Infinity, y: 0 } } },
    { kind: "mirror", axis: "x", pivot: origin, copy: "yes" },
    { kind: "scale", factors: { x: 0, y: 1, z: 1 }, pivot: origin },
    { kind: "scale", factors: { x: -1, y: 1, z: 1 }, pivot: origin },
    { kind: "scale", factors: { x: 1e-20, y: 1, z: 1 }, pivot: origin },
    { kind: "snap", spacing: 0, axes: ["x"] },
    { kind: "snap", spacing: 1, axes: [] },
  ] as ModelBatchOperation[]) assert.throws(() => run(model, operation), /model_batch:/);
  assert.throws(() => run(model, { kind: "mirror", axis: "x", pivot: origin }, [0]), /degenerate_member/);
  assert.throws(() => run(model, { kind: "rotate", axis: "z", angleDegrees: 180, pivot: origin }, [0]), /degenerate_member/);
  assert.throws(() => run(model, { kind: "mirror", axis: "x", pivot: { kind: "point", point: { x: 1e20, y: 0 } }, copy: true }), /degenerate_member/);
  assert.deepEqual(model, before);
  const mixed = structuredClone(model) as BatchModel;
  delete mixed.nodes[1].z;
  assert.throws(() => run(mixed, { kind: "rotate", axis: "z", angleDegrees: 90, pivot: origin }), /invalid_dimension/);
  const overflow = modelingFixture(1);
  overflow.nodes[0].x = 1e308;
  assert.throws(() => run(overflow, { kind: "scale", factors: { x: 2, y: 1 }, pivot: origin }), /invalid_number/);
});

test("planar transforms stay in XY and frame copies clear moments and distributed loads", () => {
  const base = modelingFixture(3);
  const planar = { ...base, nodes: base.nodes.map(({ z, fix_z, load_z, ...node }) => ({ ...node, fix_rz: true, moment_z: 5 })),
    elements: base.elements.map((member) => ({ ...member, distributed_load_y: -3 })) };
  const copied = run(planar, { kind: "rotate", axis: "z", angleDegrees: 90, pivot: origin, copy: true });
  assert.deepEqual(copied.model.nodes.map((node) => node.y), [0, 0, 0, 0, 1, 2]);
  assert.ok(copied.model.nodes.every((node) => !("z" in node)));
  assert.equal(copied.model.nodes[3].moment_z, 0);
  assert.equal(copied.model.nodes[3].fix_rz, false);
  assert.equal(copied.model.elements[2].distributed_load_y, 0);
  for (const operation of [
    { kind: "rotate", axis: "x", angleDegrees: 90, pivot: origin },
    { kind: "mirror", axis: "z", pivot: origin },
    { kind: "snap", axes: ["z"], spacing: 1 },
    { kind: "scale", factors: { x: 1, y: 1, z: 2 }, pivot: origin },
    { kind: "rotate", axis: "z", angleDegrees: 90, pivot: { kind: "point", point: { x: 0, y: 0, z: 1 } } },
  ] as ModelBatchOperation[]) assert.throws(() => run(planar, operation), /unsupported_axis/);
});

test("transform round trips preserve member lengths and coordinates over varied pivots and angles", () => {
  const model = modelingFixture(40), before = structuredClone(model);
  const lengths = (input: BatchModel) => input.elements.map(({ node_i: i, node_j: j }) =>
    Math.hypot(input.nodes[i].x - input.nodes[j].x, input.nodes[i].y - input.nodes[j].y, input.nodes[i].z! - input.nodes[j].z!));
  for (const axis of ["x", "y", "z"] as BatchAxis[]) {
    for (const angleDegrees of [-270, -133.7, -0.1, 0, 23.15, 90, 180, 477]) {
      for (const pivot of [origin, { kind: "selection" }, { kind: "point", point: { x: -12.3, y: 20.5, z: 1.2 } }] as BatchPivot[]) {
        const rotated = run(model, { kind: "rotate", axis, angleDegrees, pivot }).model;
        lengths(rotated).forEach((length, i) => close(length, lengths(model)[i]));
        // A bounds pivot is resolved once; use that same original point for the inverse.
        const inversePivot: BatchPivot = pivot.kind === "selection" ? { kind: "point", point: { x: 19.5, y: 0, z: 3 } } : pivot;
        const restored = run(rotated, { kind: "rotate", axis, angleDegrees: -angleDegrees, pivot: inversePivot }).model;
        restored.nodes.forEach((node, i) => xyz(node).forEach((value, j) => close(value!, xyz(model.nodes[i])[j]!)));
      }
    }
  }
  assert.deepEqual(model, before);
});

test("quick 3D duplication shares stable IDs, deduplicates stale selection and does not quantize geometry", () => {
  const model = modelingFixture(3);
  model.nodes[0].id = "n3"; model.elements[0].id = "e2"; model.nodes[0].x = 0.123456789;
  const copied = cloneTruss3dSelectedNodes(model, [0, 1, 1, 900], null);
  assert.deepEqual(copied.nextSelection, [3, 4]);
  assert.equal(copied.model.nodes[3].x, model.nodes[0].x + 0.4);
  assert.equal(copied.model.elements[0], model.elements[0]);
  assert.equal(new Set(copied.model.nodes.map((node) => node.id)).size, 5);
  assert.equal(new Set(copied.model.elements.map((member) => member.id)).size, 3);
  assert.equal(copied.model.nodes[3].fix_x, true, "quick duplicate retains its existing boundary policy");
  assert.equal(cloneTruss3dSelectedNodes(model, [900], null).model, model);
  const mirrored = cloneTruss3dSelectedNodes(model, [0, 1], null, "x");
  close(mirrored.model.nodes[3].x, model.nodes[1].x);
});

test("transform failures preserve controller history and stale quick copies do not record checkpoints", () => {
  const model = modelingFixture(2), events: string[] = [];
  const controller = createWorkbenchModelBatchController({
    studyKind: "truss_3d", truss3dModel: model, trussModel: model, frameModel: null!, selectedNode: 0,
    selectedTruss3dNodes: [0, 1], setTruss3dModel: () => events.push("model"),
    setTrussModel: () => assert.fail("wrong model"), setFrameModel: () => assert.fail("wrong model"),
    recordHistory: () => events.push("history"), resetActiveResult: () => events.push("reset"),
    setSelectedNode: () => {}, setSelectedElement: () => {}, setSelectedTruss3dNodes: () => {},
    setMemberDraftNodes: () => {}, historyLabel: "Transform",
  });
  assert.throws(() => controller.apply({ query: all, operation: { kind: "snap", spacing: 10, axes: ["x", "z"] } }), /degenerate_member/);
  controller.apply({ query: all, operation: { kind: "rotate", axis: "z", angleDegrees: 360, pivot: origin } });
  assert.deepEqual(events, []);
  controller.apply({ query: all, operation: { kind: "mirror", axis: "z", pivot: origin } });
  assert.deepEqual(events, ["history", "reset", "model"]);
  const quickEvents: string[] = [];
  const quick = createWorkbenchStructureEditController({
    studyKind: "truss_3d", truss3dModel: model, selectedTruss3dNodes: [999], selectedNode: null,
    recordHistory: () => quickEvents.push("history"), resetResults: () => quickEvents.push("reset"),
    setMessage: () => quickEvents.push("message"), setTruss3dModel: () => quickEvents.push("model"),
  } as unknown as Parameters<typeof createWorkbenchStructureEditController>[0]);
  quick.cloneSelectedTruss3dNodes();
  assert.deepEqual(quickEvents, []);
});

test("200k-node transforms use bounded topology scans without argument-stack or copy amplification", () => {
  const model = modelingFixture(200_000);
  let reads = 0;
  model.elements = model.elements.map((member) => ({ ...member, get node_i() { reads++; return member.node_i; } }));
  const result = run(model, { kind: "rotate", axis: "z", angleDegrees: 90, pivot: { kind: "selection" } });
  assert.equal(result.model.nodes.length, 200_000);
  assert.equal(result.summary.affectedMembers, 199_999);
  assert.equal(result.model.elements, model.elements);
  assert.ok(reads <= model.elements.length * 2, `unexpected nested scan: ${reads}`);
});

test("geometry transform copy covers every shipped UI language", () => {
  const english = getModelTransformCopy("en");
  for (const { value } of buildWorkbenchLanguageOptions({ copy: {}, languagePacks: [], currentLanguage: "en" })) {
    const copy = getModelTransformCopy(value);
    assert.deepEqual(Object.keys(copy), Object.keys(english));
    assert.ok(Object.values(copy).every((entry) => typeof entry === "string" && entry.trim().length > 0), value);
    if (value !== "en") assert.notEqual(copy.hint, english.hint, `${value} fell back to English`);
  }
});
