import test from "node:test";
import assert from "node:assert/strict";

import {
  projectElectrostaticPlaneQuadResultToHeatModel,
  projectElectrostaticPlaneTriangleResultToHeatModel,
} from "../../src/components/workbench/workbench-electrostatic-heat-projection.ts";
import type {
  ElectrostaticPlaneQuad2dJobInput,
  ElectrostaticPlaneQuad2dResult,
  ElectrostaticPlaneTriangle2dJobInput,
  ElectrostaticPlaneTriangle2dResult,
} from "../../src/lib/api/index.ts";

test("electrostatic quad results project into heat quad models with thermal loads", () => {
  const input: ElectrostaticPlaneQuad2dJobInput = {
    nodes: [
      { id: "ep0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
      { id: "ep1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
      { id: "ep2", x: 1, y: 1, fix_potential: true, potential: 0, charge_density: 0 },
      { id: "ep3", x: 0, y: 1, fix_potential: true, potential: 10, charge_density: 0 },
    ],
    elements: [
      {
        id: "epq0",
        node_i: 0,
        node_j: 1,
        node_k: 2,
        node_l: 3,
        permittivity: 2,
        material_id: "dielectric-a",
        thickness: 0.08,
      },
    ],
    materials: [
      {
        id: "dielectric-a",
        name: "Dielectric A",
        youngs_modulus: 70e9,
      },
    ],
  };
  const result: ElectrostaticPlaneQuad2dResult = {
    max_electric_field: 4,
    max_flux_density: 8,
    max_potential: 10,
    input,
    nodes: [
      { index: 0, id: "ep0", x: 0, y: 0, potential: 10, charge_density: 0 },
      { index: 1, id: "ep1", x: 1, y: 0, potential: 0, charge_density: 0 },
      { index: 2, id: "ep2", x: 1, y: 1, potential: 0, charge_density: 0 },
      { index: 3, id: "ep3", x: 0, y: 1, potential: 10, charge_density: 0 },
    ],
    elements: [
      {
        index: 0,
        id: "epq0",
        node_i: 0,
        node_j: 1,
        node_k: 2,
        node_l: 3,
        area: 1,
        average_potential: 5,
        potential_gradient_x: -10,
        potential_gradient_y: 0,
        electric_field_x: 10,
        electric_field_y: 0,
        electric_field_magnitude: 4,
        electric_flux_density_x: 8,
        electric_flux_density_y: 0,
        electric_flux_density_magnitude: 8,
      },
    ],
  };

  const heatModel = projectElectrostaticPlaneQuadResultToHeatModel(result, undefined, {
    coldTemperature: 25,
    conductivity: 33,
    heatLoadScale: 10,
    hotTemperature: 125,
  });

  assert.equal(heatModel.nodes.length, 4);
  assert.deepEqual(
    heatModel.nodes.map((node) => node.heat_load),
    [40, 40, 40, 40],
  );
  assert.deepEqual(
    heatModel.nodes.map((node) => node.temperature),
    [125, 25, 25, 125],
  );
  assert.ok(heatModel.nodes.every((node) => node.fix_temperature));
  assert.deepEqual(heatModel.elements[0], {
    id: "epq0",
    node_i: 0,
    node_j: 1,
    node_k: 2,
    node_l: 3,
    thickness: 0.08,
    conductivity: 33,
    material_id: "dielectric-a",
  });
  assert.equal(heatModel.materials?.[0]?.id, "dielectric-a");
});

test("electrostatic triangle results project into heat triangle models with inherited topology", () => {
  const input: ElectrostaticPlaneTriangle2dJobInput = {
    nodes: [
      { id: "et0", x: 0, y: 0, fix_potential: true, potential: 6, charge_density: 0 },
      { id: "et1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
      { id: "et2", x: 0, y: 1, fix_potential: false, potential: 0, charge_density: 0 },
    ],
    elements: [
      {
        id: "ept0",
        node_i: 0,
        node_j: 1,
        node_k: 2,
        permittivity: 3,
        material_id: "dielectric-b",
        thickness: 0.04,
      },
    ],
    materials: [{ id: "dielectric-b", name: "Dielectric B", youngs_modulus: 120e9 }],
  };
  const result: ElectrostaticPlaneTriangle2dResult = {
    max_electric_field: 3,
    max_flux_density: 9,
    max_potential: 6,
    input,
    nodes: [
      { index: 0, id: "et0", x: 0, y: 0, potential: 6, charge_density: 0 },
      { index: 1, id: "et1", x: 1, y: 0, potential: 0, charge_density: 0 },
      { index: 2, id: "et2", x: 0, y: 1, potential: 0, charge_density: 0 },
    ],
    elements: [
      {
        index: 0,
        id: "ept0",
        node_i: 0,
        node_j: 1,
        node_k: 2,
        area: 0.5,
        average_potential: 2,
        potential_gradient_x: -6,
        potential_gradient_y: 0,
        electric_field_x: 6,
        electric_field_y: 0,
        electric_field_magnitude: 3,
        electric_flux_density_x: 9,
        electric_flux_density_y: 0,
        electric_flux_density_magnitude: 9,
      },
    ],
  };

  const heatModel = projectElectrostaticPlaneTriangleResultToHeatModel(result, undefined, {
    conductivity: 24,
    heatLoadScale: 5,
  });

  assert.deepEqual(heatModel.nodes.map((node) => node.heat_load), [15, 15, 15]);
  assert.deepEqual(heatModel.nodes.map((node) => node.fix_temperature), [true, true, false]);
  assert.equal(heatModel.elements[0].node_k, 2);
  assert.equal(heatModel.elements[0].thickness, 0.04);
  assert.equal(heatModel.elements[0].conductivity, 24);
  assert.equal(heatModel.elements[0].material_id, "dielectric-b");
});
