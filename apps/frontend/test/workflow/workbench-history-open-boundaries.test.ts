import test from "node:test";
import assert from "node:assert/strict";
import { applyHistoryJobPayload } from "../../src/components/workbench/workbench-history-result.ts";
import { historyEffects, historyInputs, historyResult } from "../support/history-result-fixture.ts";
import { WORKBENCH_STUDY_KINDS } from "../../src/components/workbench/workbench-study-kind-controller.ts";

const job = { job_id: "history-job", status: "completed", worker_id: "test", progress: 1 } as const;

test("history routing fixtures cover every Workbench study kind", () => {
  assert.deepEqual(Object.keys(historyInputs).sort(), [...WORKBENCH_STUDY_KINDS].sort());
});

for (const kind of Object.keys(historyInputs) as Array<keyof typeof historyInputs>) {
  test(`history opens ${kind} exactly once without falling through to another study`, () => {
    const result = historyResult(kind);
    const original = structuredClone(result);
    const { effects, writes } = historyEffects();
    applyHistoryJobPayload({ job, result }, effects);
    assert.deepEqual(writes.filter(([key]) => key === "setStudyKind").map(([, value]) => value), [kind]);
    assert.equal(writes.filter(([key]) => key === "recordHistory").length, 1);
    assert.equal(writes.filter(([key]) => key === "detachSavedModel").length, 1);
    assert.equal(writes.filter(([key]) => key === "setResult").at(-1)?.[1], result);
    assert.deepEqual(result, original, "opening history must not mutate archived input");
    if (kind !== "axial_bar_1d") {
      const models = writes.filter(([key]) => /^set.*Model$/u.test(key));
      assert.equal(models.length, 1);
      assert.deepEqual(models[0][1], result.input, "archived material values must not be replaced by the current preset");
    }
  });
}

test("history without a result clears the previous result without replacing the working model", () => {
  const { effects, writes } = historyEffects();
  applyHistoryJobPayload({ job: { ...job, status: "failed" } }, effects);
  assert.deepEqual(writes.filter(([key]) => key === "setResult"), [["setResult", null]]);
  assert.equal(writes.some(([key]) => key === "setStudyKind" || key === "detachSavedModel"), false);
});

for (const failure of ["unknown", "null-node", "null-result-node", "null-result-element", "mixed-dimension", "mixed-arity", "conflicting-physics"]) {
  test(`history rejects ${failure} before changing any workspace state`, () => {
    const result = historyResult("heat_plane_triangle_2d");
    if (failure === "unknown") delete result.input;
    if (failure === "null-node") result.input.nodes[0] = null;
    if (failure === "null-result-node") result.nodes[0] = null;
    if (failure === "null-result-element") result.elements[0] = null;
    if (failure === "mixed-dimension") result.input.nodes[0].z = 0;
    if (failure === "mixed-arity") result.input.elements[0].node_l = 3;
    if (failure === "conflicting-physics") result.input.elements[0].stiffness = 12;
    const { effects, writes } = historyEffects();
    assert.throws(() => applyHistoryJobPayload({ job, result }, effects), /HISTORY_RESULT_INVALID/u);
    assert.deepEqual(writes, []);
  });
}

test("history uses archived inline material values without adopting their old project or version", () => {
  const result = historyResult("truss_2d");
  result.input.project_id = "old-project";
  result.input.model_version_id = "old-version";
  delete result.input.materials;
  result.input.elements.forEach((element: any) => { delete element.material_id; element.youngs_modulus = 123e9; });
  const original = structuredClone(result);
  const { effects, writes } = historyEffects();
  applyHistoryJobPayload({ job, result }, effects);
  const input = writes.find(([key]) => key === "setTrussModel")![1];
  assert.equal(input.project_id, undefined);
  assert.equal(input.model_version_id, undefined);
  assert.equal(input.materials, undefined);
  assert.equal(input.nodes, result.input.nodes, "do not deep-copy immutable mesh arrays");
  assert.equal(input.elements, result.input.elements);
  assert.deepEqual(result, original);
});
