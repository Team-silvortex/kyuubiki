import test from "node:test";
import assert from "node:assert/strict";
import { createModelBatchDraft, readModelBatchDraft } from "../../src/components/workbench/model/workbench-model-batch-draft.ts";

test("batch drafts retain only lightweight form parameters across editor remounts", () => {
  const draft = { ...createModelBatchDraft(true), kind: "rotate" as const, angle: "37.5", indices: "1-200" };
  const restored = readModelBatchDraft({ studyKind: "truss_3d", draft }, "truss_3d", true);
  assert.equal(restored, draft);
  assert.equal(restored.angle, "37.5");
  for (const field of ["model", "confirmation", "message", "nodes", "elements"]) assert.equal(field in restored, false);
  assert.ok(JSON.stringify(restored).length < 1000);
});

test("different study kinds cannot inherit incompatible batch parameters or shared mutable defaults", () => {
  const spatial = createModelBatchDraft(true);
  const planar = readModelBatchDraft({ studyKind: "truss_3d", draft: spatial }, "truss_2d", false);
  assert.deepEqual(planar.snapAxes, ["x", "y"]);
  assert.notEqual(planar.vector, spatial.vector);
  planar.vector.x = "12";
  planar.snapAxes.pop();
  assert.equal(createModelBatchDraft(false).vector.x, "0");
  assert.deepEqual(createModelBatchDraft(false).snapAxes, ["x", "y"]);
});
