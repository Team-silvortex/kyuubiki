import assert from "node:assert/strict";
import test from "node:test";
import {
  projectElectrostaticPlaneQuadResultToHeatModel,
  projectElectrostaticPlaneTriangleResultToHeatModel,
} from "../../src/components/workbench/workbench-electrostatic-heat-projection.ts";
import { electrostaticQuadResult, electrostaticTriangleResult } from "../support/electrostatic-projection-fixture.ts";

for (const quad of [false, true]) {
  const kind = quad ? "quad" : "triangle";
  const build = quad ? projectElectrostaticPlaneQuadResultToHeatModel : projectElectrostaticPlaneTriangleResultToHeatModel;
  const fixture = quad ? electrostaticQuadResult : electrostaticTriangleResult;
  // Both entry points consume the same boundary cases with shape-specific fixtures.
  const project = (result: ReturnType<typeof fixture>, seed?: any, options?: any, current?: any) => build(result as any, seed, options, current);

  test(`electrostatic ${kind} projection is invariant to node and element response order`, () => {
    const result = fixture();
    const baseline = project(result);
    const expectedLoads = quad ? [100, 150, 200, 100, 150, 200] : [100, 500 / 3, 200, 100, 400 / 3, 200];
    baseline.nodes.forEach((node, index) => assert.ok(Math.abs(node.heat_load! - expectedLoads[index]) < 1e-10));
    const seed = structuredClone(baseline);
    seed.elements.forEach((element, index) => { element.conductivity = 20 + index; });
    const expected = project(result, seed);
    result.nodes.reverse();
    result.elements.reverse();
    const before = structuredClone({ result, seed });
    assert.deepEqual(project(result, seed), expected);
    assert.deepEqual({ result, seed }, before);
    assert.equal("project_id" in expected, false);
    assert.equal("model_version_id" in expected, false);
    assert.notEqual(expected.materials, result.input.materials);
  });

  for (const failure of ["missing-node", "duplicate-node", "invalid-node-index", "fractional-node-index", "node-id", "coordinates", "potential",
    "missing-element", "duplicate-element", "invalid-element-index", "fractional-element-index", "element-id", "connectivity", "source-connectivity", "field", "negative-field", "thickness"]) {
    test(`electrostatic ${kind} projection rejects ${failure} rather than fabricating heat input`, () => {
      const result = fixture();
      if (failure === "missing-node") result.nodes.pop();
      if (failure === "duplicate-node") result.nodes[1].index = result.nodes[0].index;
      if (failure === "invalid-node-index") result.nodes[0].index = result.nodes.length;
      if (failure === "fractional-node-index") result.nodes[0].index = 0.5;
      if (failure === "node-id") result.nodes[0].id = "wrong-node";
      if (failure === "coordinates") result.nodes[0].x += 1;
      if (failure === "potential") result.nodes[0].potential = NaN;
      if (failure === "missing-element") result.elements.pop();
      if (failure === "duplicate-element") result.elements[1].index = result.elements[0].index;
      if (failure === "invalid-element-index") result.elements[0].index = result.elements.length;
      if (failure === "fractional-element-index") result.elements[0].index = 0.5;
      if (failure === "element-id") result.elements[0].id = "wrong-element";
      if (failure === "connectivity") result.elements[0].node_j = result.elements[0].node_i;
      if (failure === "source-connectivity") {
        result.input.elements[0].node_i = result.nodes.length;
        result.elements[0].node_i = result.nodes.length;
      }
      if (failure === "field") result.elements[0].electric_field_magnitude = NaN;
      if (failure === "negative-field") result.elements[0].electric_field_magnitude = -1;
      if (failure === "thickness") result.input.elements[0].thickness = 0;
      assert.throws(() => project(result), /ELECTROSTATIC_HEAT_PROJECTION_INVALID/u);
    });
  }

  test(`electrostatic ${kind} projection does not reuse an unrelated same-count thermal seed`, () => {
    const result = fixture();
    const seed = project(result);
    seed.nodes[0].x -= 10;
    seed.elements.forEach((element) => { element.conductivity = 999; });
    assert.ok(project(result, seed, { conductivity: 17 }).elements.every((element) => element.conductivity === 17));
  });

  test(`electrostatic ${kind} fallback temperature anchors follow node identities`, () => {
    const result = fixture();
    result.input.nodes.forEach((node) => { node.fix_potential = false; });
    result.nodes.forEach((node, index) => { node.potential = index; });
    const expected = project(result);
    result.nodes.reverse();
    assert.deepEqual(project(result), expected);
  });

  test(`electrostatic ${kind} seed material bindings are retained without dangling references`, () => {
    const result = fixture();
    const seed = project(result);
    delete result.input.materials;
    result.input.elements.forEach((element) => { delete element.material_id; });
    const actual = project(result, seed);
    assert.deepEqual(actual.materials, seed.materials);
    assert.notEqual(actual.materials, seed.materials);
    assert.ok(actual.elements.every((element) => actual.materials?.some((entry) => entry.id === element.material_id)));
  });

  test(`electrostatic ${kind} matching thermal boundaries preserve explicit zero and released supports`, () => {
    const result = fixture();
    const seed = project(result);
    seed.nodes[0].temperature = 0;
    seed.nodes[2].fix_temperature = false;
    seed.nodes[1].fix_temperature = true;
    seed.nodes[1].temperature = -10;
    seed.nodes[1].heat_load = 999;
    const actual = project(result, seed);
    assert.deepEqual(actual.nodes.map(({ fix_temperature, temperature }) => ({ fix_temperature, temperature })),
      seed.nodes.map(({ fix_temperature, temperature }) => ({ fix_temperature, temperature })));
    assert.notEqual(actual.nodes[1].heat_load, 999, "projected loads replace rather than accumulate a previous projection");
  });

  for (const [label, options] of Object.entries({
    "NaN temperature": { coldTemperature: NaN }, "infinite temperature": { hotTemperature: Infinity },
    "null temperature": { coldTemperature: null }, "reversed range": { coldTemperature: 100, hotTemperature: 10 },
    "zero conductivity": { conductivity: 0 }, "negative conductivity": { conductivity: -1 },
    "null conductivity": { conductivity: null }, "infinite conductivity": { conductivity: Infinity },
    "negative scale": { heatLoadScale: -1 }, "NaN scale": { heatLoadScale: NaN },
    "overflowed temperature range": { coldTemperature: -Number.MAX_VALUE, hotTemperature: Number.MAX_VALUE },
  })) {
    test(`electrostatic ${kind} projection rejects ${label} options`, () => {
      assert.throws(() => project(fixture(), undefined, options), /ELECTROSTATIC_HEAT_PROJECTION_INVALID/u);
    });
  }

  test(`electrostatic ${kind} zero-load and uniform-temperature options remain valid`, () => {
    const target = project(fixture(), undefined, { heatLoadScale: 0, coldTemperature: 0, hotTemperature: 0 });
    assert.ok(target.nodes.every((node) => node.heat_load === 0 && node.temperature === 0));
  });

  test(`electrostatic ${kind} invalid aligned seed values cannot silently fall back to defaults`, () => {
    const result = fixture();
    for (const value of [NaN, Infinity, 0, -1, undefined, null]) {
      const seed = project(result);
      (seed.elements[0] as any).conductivity = value;
      assert.throws(() => project(result, seed), /ELECTROSTATIC_HEAT_PROJECTION_INVALID/u);
    }
    const seed = project(result);
    seed.nodes[0].temperature = NaN;
    assert.throws(() => project(result, seed), /Seed temperature/u);
  });

  for (const change of ["geometry", "potential", "constraint", "permittivity", "thickness", "identity"]) {
    test(`electrostatic ${kind} projection rejects a stale working ${change}`, () => {
      const result = fixture();
      const current = structuredClone(result.input);
      if (change === "geometry") current.nodes[0].x += 1;
      if (change === "potential") current.nodes[0].potential = 99;
      if (change === "constraint") current.nodes[0].fix_potential = !current.nodes[0].fix_potential;
      if (change === "permittivity") current.elements[0].permittivity *= 2;
      if (change === "thickness") current.elements[0].thickness *= 2;
      if (change === "identity") current.elements[0].id = "changed-element";
      assert.throws(() => project(result, undefined, undefined, current), /working electrostatic model has changed/u);
    });
  }

  test(`electrostatic ${kind} projection rejects missing and ambiguous material provenance`, () => {
    const result = fixture();
    result.input.elements[0].material_id = "missing-material";
    assert.throws(() => project(result), /missing material/u);
    const duplicate = fixture();
    duplicate.input.materials!.push({ ...duplicate.input.materials![0] });
    assert.throws(() => project(duplicate), /unique/u);
    const conflicting = fixture();
    const seed = project(conflicting);
    seed.materials![0].name = "different material";
    delete seed.elements.at(-1)!.material_id;
    conflicting.input.elements.at(-1)!.material_id = conflicting.input.elements[0].material_id;
    assert.throws(() => project(conflicting, seed), /conflicting definitions/u);
  });

  test(`electrostatic ${kind} projection guards arithmetic overflow without overflowing finite-load means`, () => {
    const result = fixture();
    result.elements.forEach((element) => { element.electric_field_magnitude = Number.MAX_VALUE; });
    assert.throws(() => project(result), /Projected heat load/u);
    const target = project(result, undefined, { heatLoadScale: 0.5 });
    assert.ok(target.nodes.every((node) => node.heat_load === Number.MAX_VALUE / 2));
    result.nodes[0].potential = -Number.MAX_VALUE;
    result.nodes[1].potential = Number.MAX_VALUE;
    assert.throws(() => project(result), /Potential range/u);
  });
}
