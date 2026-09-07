import test from "node:test";
import assert from "node:assert/strict";
import { applyStudyKindSelection, createStudyKindResetHandlers, WORKBENCH_STUDY_KINDS } from "../../src/components/workbench/workbench-study-kind-controller.ts";
import { ensureBeamModelMaterials } from "../../src/lib/workbench/material-commands.ts";

for (const kind of WORKBENCH_STUDY_KINDS.filter((entry) => !["axial_bar_1d", "truss_2d", "truss_3d"].includes(entry))) {
  test(`study reset owns a nonempty default model for ${kind}`, () => {
    const models: Array<{ nodes: unknown[]; elements: unknown[] }> = [];
    const setter = (model: { nodes: unknown[]; elements: unknown[] }) => models.push(model);
    const handlers = createStudyKindResetHandlers({
      activeMaterial: "210", setPlaneModel: setter, setHeatBarModel: setter, setHeatPlaneModel: setter,
      setThermalBarModel: setter, setThermalBeamModel: setter, setThermalFrameModel: setter,
      setThermalTrussModel: setter, setThermalTruss3dModel: setter, setSpringModel: setter,
      setSpring2dModel: setter, setSpring3dModel: setter, setBeamModel: setter, setTorsionModel: setter,
      setFrameModel: setter, setPlaneResultField: () => {},
    });
    assert.equal(typeof handlers[kind], "function");
    handlers[kind]!();
    assert.equal(models.length, 1);
    assert.ok(models[0]?.nodes.length > 0);
    assert.ok(models[0]?.elements.length > 0);
  });
}

test("beam material normalization preserves thermal fields and does not mutate its input", () => {
  const input = { elements: [{ material_id: undefined as string | undefined, thermal_expansion: 1.2e-5 }] };
  const result = ensureBeamModelMaterials(input, "210");
  assert.equal(result.elements[0].thermal_expansion, input.elements[0].thermal_expansion);
  assert.equal(result.elements[0].material_id, "mat-1");
  assert.equal(input.elements[0].material_id, undefined);
});

test("study selection clears prior results only when changing the study", () => {
  const events: string[] = [];
  const selection = { currentStudyKind: "heat_bar_1d" as const, nextStudyKind: "truss_2d" as const,
    setStudyKind: () => events.push("select"), resetActiveResult: () => events.push("clear"), resetHandlers: {} };
  applyStudyKindSelection(selection);
  assert.deepEqual(events, ["clear", "select"]);
  events.length = 0;
  applyStudyKindSelection({ ...selection, nextStudyKind: "heat_bar_1d" });
  assert.deepEqual(events, ["select"]);
});
