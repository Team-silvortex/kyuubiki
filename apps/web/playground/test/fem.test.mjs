import test from "node:test";
import assert from "node:assert/strict";

import { solveBar1D } from "../src/fem.mjs";

test("solves a single-element axial bar with the expected tip displacement", () => {
  const result = solveBar1D({
    length: 1,
    area: 0.01,
    youngsModulus: 210e9,
    elements: 1,
    tipForce: 1000,
  });

  assert.equal(result.nodes.length, 2);
  assert.equal(result.elements.length, 1);
  assert.ok(Math.abs(result.nodes[1].displacement - 4.761904761904762e-7) < 1e-12);
  assert.ok(Math.abs(result.elements[0].stress - 100000) < 1e-6);
  assert.ok(Math.abs(result.reactionForce + 1000) < 1e-6);
});

test("keeps nodal displacement monotonic for a tensile load across multiple elements", () => {
  const result = solveBar1D({
    length: 2,
    area: 0.02,
    youngsModulus: 70e9,
    elements: 4,
    tipForce: 5000,
  });

  assert.equal(result.nodes[0].displacement, 0);

  for (let index = 1; index < result.nodes.length; index += 1) {
    assert.ok(result.nodes[index].displacement >= result.nodes[index - 1].displacement);
  }

  assert.equal(result.elements.length, 4);
  assert.ok(result.maxDisplacement > 0);
  assert.ok(result.maxStress > 0);
});
