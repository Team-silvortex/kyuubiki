import test from "node:test";
import assert from "node:assert/strict";

import type { PlaneTriangle2dJobInput, Truss2dJobInput } from "../../src/lib/api/index.ts";
import {
  addCustomMaterialToTrussModel,
  addPresetMaterialToTrussModel,
  applyMaterialToPlaneModel,
  applyMaterialToTrussModel,
  nextMaterialId,
  updateMaterialInPlaneModel,
  updateMaterialInTrussModel,
} from "../../src/lib/workbench/material-commands.ts";

function trussModel(): Truss2dJobInput {
  return {
    nodes: [],
    elements: [
      {
        id: "member-1",
        node_i: 0,
        node_j: 1,
        area: 0.01,
        youngs_modulus: 70e9,
        material_id: "mat-1",
      },
    ],
    materials: [
      { id: "mat-1", name: "Aluminum", youngs_modulus: 70e9, poisson_ratio: 0.33 },
      { id: "mat-3", name: "Steel", youngs_modulus: 210e9, poisson_ratio: 0.3 },
    ],
  };
}

function planeModel(): PlaneTriangle2dJobInput {
  return {
    nodes: [],
    elements: [
      {
        id: "panel-1",
        node_i: 0,
        node_j: 1,
        node_k: 2,
        thickness: 0.01,
        youngs_modulus: 70e9,
        poisson_ratio: 0.33,
        material_id: "mat-1",
      },
    ],
    materials: [
      { id: "mat-1", name: "Aluminum", youngs_modulus: 70e9, poisson_ratio: 0.33 },
    ],
  };
}

test("material additions allocate an unused stable ID after sparse edits", () => {
  const model = trussModel();

  assert.equal(nextMaterialId(model.materials), "mat-2");
  const withPreset = addPresetMaterialToTrussModel(model, "116");
  const withCustom = addCustomMaterialToTrussModel(withPreset);
  const ids = withCustom.materials?.map((material) => material.id) ?? [];

  assert.deepEqual(ids, ["mat-1", "mat-3", "mat-2", "mat-4"]);
  assert.equal(new Set(ids).size, ids.length);
});

test("numeric material edits stay numeric in both material and element records", () => {
  const truss = updateMaterialInTrussModel(
    trussModel(),
    "mat-1",
    "youngs_modulus",
    "116000000000",
  );
  const plane = updateMaterialInPlaneModel(planeModel(), "mat-1", "poisson_ratio", "0.29");

  assert.equal(truss.materials?.[0]?.youngs_modulus, 116e9);
  assert.equal(typeof truss.materials?.[0]?.youngs_modulus, "number");
  assert.equal(truss.elements[0]?.youngs_modulus, 116e9);
  assert.equal(plane.materials?.[0]?.poisson_ratio, 0.29);
  assert.equal(typeof plane.materials?.[0]?.poisson_ratio, "number");
  assert.equal(plane.elements[0]?.poisson_ratio, 0.29);
});

test("invalid material numbers do not poison an otherwise valid model", () => {
  const truss = trussModel();
  const plane = planeModel();

  assert.strictEqual(
    updateMaterialInTrussModel(truss, "mat-1", "youngs_modulus", Number.NaN),
    truss,
  );
  assert.strictEqual(updateMaterialInPlaneModel(plane, "mat-1", "poisson_ratio", 0.5), plane);
});

test("unknown material IDs cannot create dangling element references", () => {
  const truss = trussModel();
  const plane = planeModel();

  assert.strictEqual(applyMaterialToTrussModel(truss, "missing", "all", null), truss);
  assert.strictEqual(applyMaterialToPlaneModel(plane, "missing", "all", null), plane);
  assert.equal(truss.elements[0]?.material_id, "mat-1");
  assert.equal(plane.elements[0]?.material_id, "mat-1");
});
