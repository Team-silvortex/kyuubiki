import test from "node:test";
import assert from "node:assert/strict";

import { parsePlaygroundModel } from "../src/model-import.mjs";

test("parses a JSON model and normalizes it into playground form fields", () => {
  const imported = parsePlaygroundModel(`
    {
      "name": "steel-demo",
      "length": 1.25,
      "area": 0.015,
      "elements": 5,
      "tip_force": 2400,
      "material": "Steel",
      "youngs_modulus_gpa": 210
    }
  `);

  assert.deepEqual(imported, {
    name: "steel-demo",
    length: 1.25,
    area: 0.015,
    elements: 5,
    tipForce: 2400,
    material: "210",
    youngsModulusGpa: 210,
  });
});

test("accepts custom modulus values even when no material preset is known", () => {
  const imported = parsePlaygroundModel(`
    {
      "length": 0.8,
      "area": 0.006,
      "elements": 2,
      "tip_force": 800,
      "youngs_modulus_gpa": 88.5
    }
  `);

  assert.equal(imported.material, "custom");
  assert.equal(imported.youngsModulusGpa, 88.5);
});
