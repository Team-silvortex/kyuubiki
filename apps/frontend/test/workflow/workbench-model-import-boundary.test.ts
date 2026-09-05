import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import { parsePlaygroundModel } from "../../src/lib/models/model-import.ts";
import { SAMPLE_LIBRARY } from "../../src/lib/models/sample-library.ts";

const legacyAxialPayload = {
  name: "legacy axial bar",
  length: 1,
  area: 0.01,
  elements: 4,
  tip_force: 1000,
  material: "steel",
  youngs_modulus_gpa: 210,
};

function trussPayload(overrides: Record<string, unknown> = {}) {
  return {
    kind: "truss_2d",
    name: "imported truss",
    material: "steel",
    youngs_modulus_gpa: 210,
    nodes: [
      { id: "node-1", x: 0, y: 0, fix_x: true, fix_y: true },
      { id: "node-2", x: 1, y: 0 },
    ],
    elements: [
      {
        id: "element-1",
        node_i: 0,
        node_j: 1,
        area: 0.01,
        youngs_modulus: 210e9,
        material_id: "mat-1",
      },
    ],
    materials: [
      { id: "mat-1", name: "Steel", youngs_modulus: 210e9, poisson_ratio: 0.3 },
    ],
    ...overrides,
  };
}

test("model import rejects an explicit unknown study kind", () => {
  assert.throws(
    () => parsePlaygroundModel(JSON.stringify({ ...legacyAxialPayload, kind: "axial_bra_1d" })),
    /unsupported model kind: axial_bra_1d/,
  );
});

test("model import rejects a malformed explicit study kind", () => {
  assert.throws(
    () => parsePlaygroundModel(JSON.stringify({ ...legacyAxialPayload, kind: 42 })),
    /kind must be a non-empty string/,
  );
});

test("model import rejects a non-object document root", () => {
  assert.throws(() => parsePlaygroundModel("null"), /model payload must be an object/);
});

test("model import preserves legacy kind inference when the field is absent", () => {
  const imported = parsePlaygroundModel(JSON.stringify(legacyAxialPayload));

  assert.equal(imported.kind, "axial_bar_1d");
  assert.equal(imported.name, "legacy axial bar");
});

test("model import rejects out-of-range element node references", () => {
  const payload = trussPayload({
    elements: [
      {
        id: "element-1",
        node_i: 0,
        node_j: 2,
        area: 0.01,
        youngs_modulus: 210e9,
        material_id: "mat-1",
      },
    ],
  });

  assert.throws(() => parsePlaygroundModel(JSON.stringify(payload)), /elements\[0\]\.node_j references missing node 2/);
});

test("model import rejects duplicate entity IDs", () => {
  const nodes = [
    { id: "node-1", x: 0, y: 0, fix_x: true, fix_y: true },
    { id: "node-1", x: 1, y: 0 },
  ];

  assert.throws(() => parsePlaygroundModel(JSON.stringify(trussPayload({ nodes }))), /duplicate nodes id: node-1/);
});

test("model import rejects dangling material references", () => {
  const elements = [
    {
      id: "element-1",
      node_i: 0,
      node_j: 1,
      area: 0.01,
      youngs_modulus: 210e9,
      material_id: "mat-missing",
    },
  ];

  assert.throws(
    () => parsePlaygroundModel(JSON.stringify(trussPayload({ elements }))),
    /elements\[0\]\.material_id references missing material: mat-missing/,
  );
});

test("model import rejects non-finite material Poisson ratios", () => {
  const materials = [
    { id: "mat-1", name: "Steel", youngs_modulus: 210e9, poisson_ratio: "not-a-number" },
  ];

  assert.throws(
    () => parsePlaygroundModel(JSON.stringify(trussPayload({ materials }))),
    /materials\[0\]\.poisson_ratio must be a finite number between -1 and 0.5/,
  );
});

test("every visible Workbench sample has a complete import mapping", async () => {
  for (const sample of SAMPLE_LIBRARY) {
    const sampleUrl = new URL(`../../public${sample.href}`, import.meta.url);
    const imported = parsePlaygroundModel(await readFile(sampleUrl, "utf8"));
    assert.equal(imported.kind, sample.kind, sample.id);
  }
});
