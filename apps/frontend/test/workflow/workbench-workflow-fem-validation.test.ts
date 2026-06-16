import test from "node:test";
import assert from "node:assert/strict";

import {
  collectWorkflowInputArtifactContractWarnings,
  validateWorkflowFemInputPayload,
} from "../../src/components/workbench/workflow/workbench-workflow-fem-validation.ts";

test("validateWorkflowFemInputPayload adds mechanical physics warnings for unconstrained unloaded studies", () => {
  const issues = validateWorkflowFemInputPayload("study_model/plane_triangle_2d", {
    nodes: [
      { fix_x: false, fix_y: false, load_x: 0, load_y: 0 },
      { fix_x: false, fix_y: false, load_x: 0, load_y: 0 },
    ],
    elements: [{ youngs_modulus: 210000000000, poisson_ratio: 0.3, thickness: 0.01 }],
  });

  assert.ok(issues.some((issue) => issue.category === "physics" && issue.sectionKey === "boundary" && issue.field === "fix_x"));
  assert.ok(issues.some((issue) => issue.category === "physics" && issue.sectionKey === "load" && issue.field === "load_x"));
});

test("validateWorkflowFemInputPayload adds electrostatic and thermo-mechanical domain warnings", () => {
  const electroIssues = validateWorkflowFemInputPayload("study_model/electrostatic_plane_quad_2d", {
    nodes: [
      { fix_potential: false, potential: 0, charge_density: 0 },
      { fix_potential: false, potential: 0, charge_density: 0 },
    ],
    elements: [{ permittivity: 8.85e-12, thickness: 1 }],
  });
  const thermoIssues = validateWorkflowFemInputPayload("study_model/thermal_plane_triangle_2d", {
    nodes: [
      { fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
    ],
    elements: [{ youngs_modulus: 210000000000, poisson_ratio: 0.3, thermal_expansion: 0, thickness: 0.01 }],
  });

  assert.ok(electroIssues.some((issue) => issue.category === "physics" && issue.field === "fix_potential"));
  assert.ok(thermoIssues.some((issue) => issue.category === "physics" && issue.field === "thermal_expansion"));
});

test("validateWorkflowFemInputPayload reports degenerate and out-of-range operator topology", () => {
  const triangleIssues = validateWorkflowFemInputPayload("study_model/plane_triangle_2d", {
    nodes: [
      { fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
      { fix_x: false, fix_y: false, load_x: 0, load_y: -1 },
    ],
    elements: [
      { node_i: 0, node_j: 0, node_k: 3, thickness: 0.01, youngs_modulus: 210000000000, poisson_ratio: 0.3 },
    ],
  });

  assert.ok(triangleIssues.some((issue) => issue.category === "contract" && issue.message.includes("at least 3 nodes")));
  assert.ok(triangleIssues.some((issue) => issue.category === "contract" && issue.message.includes("reuses the same node")));
  assert.ok(triangleIssues.some((issue) => issue.category === "contract" && issue.message.includes("outside the available node list")));
});

test("validateWorkflowFemInputPayload reports degenerate 1D bar connectivity", () => {
  const barIssues = validateWorkflowFemInputPayload("study_model/thermal_bar_1d", {
    nodes: [
      { fix_x: true, load_x: 0, temperature_delta: 0 },
      { fix_x: false, load_x: 0, temperature_delta: 20 },
    ],
    elements: [
      { node_i: 1, node_j: 1, area: 0.01, youngs_modulus: 210000000000, thermal_expansion: 0.000012 },
    ],
  });

  assert.ok(barIssues.some((issue) => issue.category === "contract" && issue.message.includes("reuses the same node")));
});

test("validateWorkflowFemInputPayload warns when electrostatic boundaries have no driving contrast", () => {
  const issues = validateWorkflowFemInputPayload("study_model/electrostatic_plane_quad_2d", {
    nodes: [
      { fix_potential: true, potential: 5, charge_density: 0 },
      { fix_potential: true, potential: 5, charge_density: 0 },
      { fix_potential: false, potential: 0, charge_density: 0 },
      { fix_potential: false, potential: 0, charge_density: 0 },
    ],
    elements: [{ node_i: 0, node_j: 1, node_k: 2, node_l: 3, permittivity: 8.85e-12, thickness: 1 }],
  });

  assert.ok(issues.some((issue) => issue.category === "physics" && issue.field === "potential" && issue.message.includes("no contrast")));
});

test("validateWorkflowFemInputPayload warns when heat boundaries have no thermal drive", () => {
  const issues = validateWorkflowFemInputPayload("study_model/heat_bar_1d", {
    nodes: [
      { fix_temperature: true, temperature: 20, heat_load: 0 },
      { fix_temperature: true, temperature: 20, heat_load: 0 },
    ],
    elements: [{ node_i: 0, node_j: 1, area: 0.02, conductivity: 45 }],
  });

  assert.ok(issues.some((issue) => issue.category === "physics" && issue.field === "temperature" && issue.message.includes("no contrast")));
});

test("validateWorkflowFemInputPayload warns when planar constraints do not cover both axes", () => {
  const issues = validateWorkflowFemInputPayload("study_model/plane_quad_2d", {
    nodes: [
      { fix_x: true, fix_y: false, load_x: 0, load_y: 0 },
      { fix_x: true, fix_y: false, load_x: 0, load_y: -1000 },
      { fix_x: false, fix_y: false, load_x: 0, load_y: 0 },
      { fix_x: false, fix_y: false, load_x: 0, load_y: 0 },
    ],
    elements: [{ node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.02, youngs_modulus: 70000000000, poisson_ratio: 0.33 }],
  });

  assert.ok(issues.some((issue) => issue.category === "physics" && issue.field === "fix_y" && issue.message.includes("y displacement")));
});

test("validateWorkflowFemInputPayload warns when thermal bar has no axial anchor", () => {
  const issues = validateWorkflowFemInputPayload("study_model/thermal_bar_1d", {
    nodes: [
      { fix_x: false, load_x: 0, temperature_delta: 20 },
      { fix_x: false, load_x: 100, temperature_delta: 30 },
    ],
    elements: [{ node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 210000000000, thermal_expansion: 0.000012 }],
  });

  assert.ok(issues.some((issue) => issue.category === "physics" && issue.field === "fix_x" && issue.message.includes("anchor at least one x displacement")));
});

test("validateWorkflowFemInputPayload warns when planar element area collapses", () => {
  const issues = validateWorkflowFemInputPayload("study_model/plane_triangle_2d", {
    nodes: [
      { x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
      { x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
      { x: 2, y: 0, fix_x: false, fix_y: false, load_x: 0, load_y: -1000 },
    ],
    elements: [{ node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, youngs_modulus: 70000000000, poisson_ratio: 0.33 }],
  });

  assert.ok(issues.some((issue) => issue.category === "physics" && issue.message.includes("near-zero planar area")));
});

test("validateWorkflowFemInputPayload warns when bar element span is zero", () => {
  const issues = validateWorkflowFemInputPayload("study_model/heat_bar_1d", {
    nodes: [
      { x: 1, fix_temperature: true, temperature: 100, heat_load: 0 },
      { x: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
    ],
    elements: [{ node_i: 0, node_j: 1, area: 0.02, conductivity: 45 }],
  });

  assert.ok(issues.some((issue) => issue.category === "physics" && issue.message.includes("zero axial span")));
});

test("collectWorkflowInputArtifactContractWarnings keeps physics-only warnings out of contract summaries", () => {
  const warnings = collectWorkflowInputArtifactContractWarnings({
    entryInputs: [{ node_id: "solver", artifact_type: "study_model/electrostatic_plane_quad_2d" }] as never,
    inputArtifactTexts: {
      solver: JSON.stringify({
        nodes: [
          { fix_potential: false, potential: 0, charge_density: 0 },
          { fix_potential: false, potential: 0, charge_density: 0 },
          { fix_potential: false, potential: 0, charge_density: 0 },
          { fix_potential: false, potential: 0, charge_density: 0 },
        ],
        elements: [{ node_i: 0, node_j: 1, node_k: 2, node_l: 3, permittivity: 8.85e-12, thickness: 1 }],
      }),
    },
  });

  assert.deepEqual(warnings, {});
});
