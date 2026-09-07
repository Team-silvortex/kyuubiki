import assert from "node:assert/strict";
import test from "node:test";
import {
  buildThermalBarFromHeatResult, buildThermalPlaneTriangleFromHeatResult, buildThermalPlaneQuadFromHeatResult,
} from "../../src/components/workbench/workbench-heat-thermo-projection.ts";
import {
  defaultHeatBar1d, defaultHeatPlaneTriangle, defaultHeatPlaneQuad, defaultThermalBar1d,
  defaultThermalPlaneTriangle, defaultThermalPlaneQuad, defaultElectrostaticPlaneQuad,
} from "../../src/components/workbench/workbench-defaults.ts";
import type { HeatBar1dResult, HeatPlaneTriangle2dResult, HeatPlaneQuad2dResult } from "../../src/lib/api/index.ts";

const cases = [
  { kind: "bar", source: defaultHeatBar1d, seed: defaultThermalBar1d,
    build: (source: any, result: any, seed: any) => buildThermalBarFromHeatResult(source, result, seed) },
  { kind: "triangle", source: defaultHeatPlaneTriangle, seed: defaultThermalPlaneTriangle,
    build: (source: any, result: any, seed: any) => buildThermalPlaneTriangleFromHeatResult(source, result, seed, "210") },
  { kind: "quad", source: defaultHeatPlaneQuad, seed: defaultThermalPlaneQuad,
    build: (source: any, result: any, seed: any) => buildThermalPlaneQuadFromHeatResult(source, result, seed, "210") },
];

function heatResult(source: typeof cases[number]["source"]) {
  return {
    input: structuredClone(source), max_temperature: 50, max_heat_flux: 1,
    nodes: source.nodes.map((node, index) => ({ ...node, index, temperature: 50 - index * 25 })),
    elements: source.elements.map((element, index) => ({ ...element, index, area: 1, length: 1,
      average_temperature: 25, temperature_gradient: 1, heat_flux: 1,
      temperature_gradient_x: 1, temperature_gradient_y: 0, heat_flux_x: 1, heat_flux_y: 0, heat_flux_magnitude: 1,
    })),
  } as HeatBar1dResult | HeatPlaneTriangle2dResult | HeatPlaneQuad2dResult;
}

for (const entry of cases) {
  for (const record of ["input-node", "input-element", "result-node"]) {
    test(`heat-to-thermo ${entry.kind} rejects a null ${record} with a controlled projection error`, () => {
      const source = structuredClone(entry.source);
      const result = heatResult(source);
      if (record === "input-node") (result.input.nodes as any[])[0] = null;
      if (record === "input-element") (result.input.elements as any[])[0] = null;
      if (record === "result-node") (result.nodes as any[])[0] = null;
      assert.throws(() => entry.build(source, result, entry.seed), /HEAT_THERMO_PROJECTION_INVALID/u);
    });
  }

  for (const field of ["conductivity", "heat_load", "temperature", "fix_temperature", "section", "element-id", "input-node-id"]) {
    test(`heat-to-thermo ${entry.kind} rejects stale solved ${field} before state replacement`, () => {
      const source = structuredClone(entry.source);
      const result = heatResult(source);
      if (field === "conductivity") source.elements[0].conductivity *= 2;
      if (field === "heat_load") source.nodes[0].heat_load = 123;
      if (field === "temperature") source.nodes[0].temperature = 123;
      if (field === "fix_temperature") source.nodes[0].fix_temperature = !source.nodes[0].fix_temperature;
      if (field === "section") {
        const element = source.elements[0];
        if ("area" in element) element.area *= 2;
        else element.thickness *= 2;
      }
      if (field === "element-id") source.elements[0].id = "different-element";
      if (field === "input-node-id") result.input.nodes[0].id = "different-solved-node";
      assert.throws(() => entry.build(source, result, entry.seed), /HEAT_THERMO_PROJECTION_INVALID/u);
    });
  }

  test(`heat-to-thermo ${entry.kind} copies temperatures by index without mutating or keeping saved bindings`, () => {
    const source = { ...structuredClone(entry.source), project_id: "old-project", model_version_id: "heat-version" };
    const seed = structuredClone(entry.seed);
    const result = heatResult(source);
    result.nodes.reverse();
    const before = structuredClone({ source, seed, result });
    const target = entry.build(source, result, seed);
    assert.deepEqual(target.nodes.map((node: any) => node.temperature_delta), source.nodes.map((_, index) => 50 - index * 25));
    assert.deepEqual(target.nodes.map(({ id, x }: any) => ({ id, x })), source.nodes.map(({ id, x }) => ({ id, x })));
    assert.equal("project_id" in target, false);
    assert.equal("model_version_id" in target, false);
    assert.deepEqual({ source, seed, result }, before);
  });

  for (const failure of ["missing", "duplicate", "out-of-range", "fractional-index", "temperature", "coordinates", "identity", "stale-mesh", "connectivity"]) {
    test(`heat-to-thermo ${entry.kind} rejects ${failure} instead of fabricating temperatures`, () => {
      const source = structuredClone(entry.source);
      const result = heatResult(source);
      if (failure === "missing") result.nodes.pop();
      if (failure === "duplicate") result.nodes[1].index = result.nodes[0].index;
      if (failure === "out-of-range") result.nodes[0].index = result.nodes.length;
      if (failure === "fractional-index") result.nodes[0].index = 0.5;
      if (failure === "temperature") result.nodes[0].temperature = Number.NaN;
      if (failure === "coordinates") result.nodes[0].x += 1;
      if (failure === "identity") result.nodes[0].id = "unrelated-node";
      if (failure === "stale-mesh") source.nodes[0].x += 1;
      if (failure === "connectivity") {
        source.elements[0].node_i = source.nodes.length;
        result.input.elements[0].node_i = source.nodes.length;
      }
      assert.throws(() => entry.build(source, result, entry.seed), /HEAT_THERMO_PROJECTION_INVALID/u);
    });
  }
}

for (const entry of cases.filter((entry) => entry.kind !== "bar")) {
  test(`heat-to-thermo ${entry.kind} accepts omitted zero-valued heat data without comparing save metadata`, () => {
    const source = { ...structuredClone(entry.source), project_id: "current-project", model_version_id: "current-version" };
    source.nodes.forEach((node) => { node.temperature = 0; node.heat_load = 0; });
    const result = heatResult(source);
    result.input.project_id = "solved-project";
    result.input.model_version_id = "solved-version";
    result.input.nodes.forEach((node) => { delete (node as any).temperature; delete (node as any).heat_load; });
    assert.ok(entry.build(source, result, entry.seed).nodes.every((node: any) => Number.isFinite(node.temperature_delta)));
  });
}

test("aligned mechanical seed preserves supports, loads, materials and zero expansion", () => {
  const source = structuredClone(defaultHeatPlaneQuad);
  const seed = structuredClone(defaultThermalPlaneQuad);
  seed.nodes[0].fix_x = false;
  seed.nodes[0].load_x = 456;
  seed.elements[0].thermal_expansion = 0;
  const target = buildThermalPlaneQuadFromHeatResult(source, heatResult(source) as HeatPlaneQuad2dResult, seed, "210");
  assert.equal(target.nodes[0].fix_x, false);
  assert.equal(target.nodes[0].load_x, 456);
  assert.equal(target.elements[0].thermal_expansion, 0);
  assert.deepEqual(target.materials, seed.materials);
  assert.notEqual(target.materials, seed.materials);
});

for (const mismatch of ["coordinates", "connectivity"]) {
  test(`equal counts alone cannot reuse an unrelated mechanical seed with different ${mismatch}`, () => {
    const source = structuredClone(defaultHeatPlaneQuad);
    const seed = structuredClone(defaultThermalPlaneQuad);
    seed.nodes[0].load_x = 456;
    seed.elements[0].thermal_expansion = 0.4;
    if (mismatch === "coordinates") seed.nodes[1].x += 1;
    else [seed.elements[0].node_i, seed.elements[0].node_j] = [seed.elements[0].node_j, seed.elements[0].node_i];
    const target = buildThermalPlaneQuadFromHeatResult(source, heatResult(source) as HeatPlaneQuad2dResult, seed, "210");
    assert.equal(target.nodes[0].load_x, 0);
    assert.equal(target.elements[0].thermal_expansion, defaultThermalPlaneQuad.elements[0].thermal_expansion);
    assert.equal(target.elements[0].youngs_modulus, 210e9);
    assert.equal(target.materials?.[0].youngs_modulus, 210e9);
  });
}

test("an electrostatic workspace is not a mechanical material or support seed", () => {
  const source = structuredClone(defaultHeatPlaneQuad);
  const target = buildThermalPlaneQuadFromHeatResult(source, heatResult(source) as HeatPlaneQuad2dResult, defaultElectrostaticPlaneQuad, "210");
  assert.ok(target.nodes.every((node) => typeof node.fix_x === "boolean"));
  assert.equal(target.elements[0].youngs_modulus, 210e9);
  assert.ok(Number.isFinite(target.elements[0].thermal_expansion));
});

test("a dangling mechanical material binding is rejected before state replacement", () => {
  const source = structuredClone(defaultHeatPlaneQuad);
  const seed = structuredClone(defaultThermalPlaneQuad);
  seed.elements[0].material_id = "missing-material";
  assert.throws(() => buildThermalPlaneQuadFromHeatResult(source, heatResult(source) as HeatPlaneQuad2dResult, seed, "210"), /missing material/u);
});
