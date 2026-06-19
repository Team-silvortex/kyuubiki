"use client";

type WorkflowSampleInputArtifactDefinition = {
  semanticType: string;
  payload: Record<string, unknown>;
};

type WorkflowSampleInputDefinition = {
  artifacts: Record<string, WorkflowSampleInputArtifactDefinition>;
};

function sampleInputArtifact(
  semanticType: string,
  payload: Record<string, unknown>,
): WorkflowSampleInputArtifactDefinition {
  return { semanticType, payload };
}

const SAMPLE_INPUTS: Record<string, WorkflowSampleInputDefinition> = {
  electrostatic_plane_quad: {
    artifacts: {
      electrostatic_model: sampleInputArtifact("study_model/electrostatic_plane_quad_2d", {
        nodes: [
          { id: "n0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
          { id: "n1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "n2", x: 1, y: 1, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "n3", x: 0, y: 1, fix_potential: true, potential: 10, charge_density: 0 },
        ],
        elements: [{ id: "epq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.05, permittivity: 2 }],
      }),
    },
  },
  electrostatic_plane_triangle: {
    artifacts: {
      electrostatic_plane_triangle_model: sampleInputArtifact("study_model/electrostatic_plane_triangle_2d", {
        nodes: [
          { id: "et0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
          { id: "et1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "et2", x: 0, y: 1, fix_potential: false, potential: 0, charge_density: 0 },
        ],
        elements: [{ id: "ept0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.05, permittivity: 2 }],
      }),
    },
  },
  electrostatic_quad_triangle_pair: {
    artifacts: {
      electrostatic_model: sampleInputArtifact("study_model/electrostatic_plane_quad_2d", {
        nodes: [
          { id: "n0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
          { id: "n1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "n2", x: 1, y: 1, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "n3", x: 0, y: 1, fix_potential: true, potential: 10, charge_density: 0 },
        ],
        elements: [{ id: "epq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.05, permittivity: 2 }],
      }),
      electrostatic_plane_triangle_model: sampleInputArtifact("study_model/electrostatic_plane_triangle_2d", {
        nodes: [
          { id: "et0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
          { id: "et1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "et2", x: 0, y: 1, fix_potential: false, potential: 0, charge_density: 0 },
        ],
        elements: [{ id: "ept0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.05, permittivity: 2 }],
      }),
    },
  },
  heat_plane_quad: {
    artifacts: {
      heat_model: sampleInputArtifact("study_model/heat_plane_quad_2d", {
        nodes: [
          { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
          { id: "h1", x: 1, y: 0, fix_temperature: false, temperature: 0, heat_load: 0 },
          { id: "h2", x: 1, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
          { id: "h3", x: 0, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
        ],
        elements: [{ id: "hq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.02, conductivity: 45 }],
      }),
    },
  },
  diagnostics_bundle_guard_report: {
    artifacts: {
      electrostatic_input: sampleInputArtifact("workflow_diagnostics/electrostatic_summary", {
        summary: {
          diagnostic_contract: "kyuubiki.workflow_diagnostics/v1",
          diagnostic_domain: "electrostatic",
          diagnostic_subject: "electrostatic_result",
          diagnostic_prefix: "electrostatic",
          diagnostic_node_count: 4,
          diagnostic_element_count: 2,
          diagnostic_metric_groups: ["field", "potential"],
          electrostatic_field_peak_magnitude: 8.4,
          electrostatic_potential_max: 12.0,
          electrostatic_potential_min: 0.0,
        },
      }),
      thermal_input: sampleInputArtifact("workflow_diagnostics/thermal_summary", {
        summary: {
          diagnostic_contract: "kyuubiki.workflow_diagnostics/v1",
          diagnostic_domain: "thermal",
          diagnostic_subject: "thermal_result",
          diagnostic_prefix: "thermal",
          diagnostic_node_count: 4,
          diagnostic_element_count: 3,
          diagnostic_metric_groups: ["temperature", "flux"],
          thermal_temperature_max: 92.0,
          thermal_temperature_min: 24.0,
          thermal_heat_flux_peak_magnitude: 16.5,
        },
      }),
      thermo_input: sampleInputArtifact("workflow_diagnostics/thermo_summary", {
        summary: {
          diagnostic_contract: "kyuubiki.workflow_diagnostics/v1",
          diagnostic_domain: "thermo",
          diagnostic_subject: "thermo_result",
          diagnostic_prefix: "thermo",
          diagnostic_node_count: 4,
          diagnostic_element_count: 3,
          diagnostic_metric_groups: ["stress", "displacement", "temperature"],
          thermo_peak_stress: 142.0,
          thermo_peak_displacement: 0.0028,
          thermo_temperature_max: 88.0,
        },
      }),
    },
  },
  peak_diagnostics_bundle_guard_report: {
    artifacts: {
      electrostatic_input: sampleInputArtifact("workflow_diagnostics/electrostatic_peak_summary", {
        summary: {
          electrostatic_peak_field: 8.4,
          electrostatic_field_peak_magnitude: 8.4,
          electrostatic_peak_flux_density: 2.1,
          electrostatic_peak_field_id: "eq1",
          electrostatic_potential_max: 12.0,
          max_potential: 12.0,
          max_electric_field: 8.4,
          max_flux_density: 2.1,
        },
      }),
      thermal_input: sampleInputArtifact("workflow_diagnostics/thermal_peak_summary", {
        summary: {
          thermal_peak_flux: 16.5,
          thermal_flux_peak_magnitude: 16.5,
          thermal_peak_flux_id: "hq1",
          thermal_temperature_max: 92.0,
          max_temperature: 92.0,
          max_heat_flux: 16.5,
        },
      }),
      thermo_input: sampleInputArtifact("workflow_diagnostics/thermo_peak_summary", {
        summary: {
          thermo_peak_displacement: 0.0028,
          thermo_displacement_peak_magnitude: 0.0028,
          thermo_peak_displacement_id: "t1",
          thermo_peak_stress: 142.0,
          thermo_stress_peak: 142.0,
          thermo_peak_stress_id: "te1",
          thermo_temperature_delta_max: 88.0,
          max_displacement: 0.0028,
          max_stress: 142.0,
          max_temperature_delta: 88.0,
        },
      }),
    },
  },
  bar_1d: {
    artifacts: {
      bar_1d_model: sampleInputArtifact("study_model/bar_1d", { length: 1, area: 0.01, youngs_modulus: 210000000000, elements: 2, tip_force: 1200 }),
    },
  },
  thermal_bar_1d: {
    artifacts: {
      thermal_bar_1d_model: sampleInputArtifact("study_model/thermal_bar_1d", {
        nodes: [
          { id: "n0", x: 0, fix_x: true, load_x: 0, temperature_delta: 0 },
          { id: "n1", x: 1, fix_x: true, load_x: 0, temperature_delta: 35 },
        ],
        elements: [{ id: "e0", node_i: 0, node_j: 1, area: 0.01, youngs_modulus: 210000000000, thermal_expansion: 0.000012 }],
      }),
    },
  },
  heat_bar_1d: {
    artifacts: {
      heat_bar_1d_model: sampleInputArtifact("study_model/heat_bar_1d", {
        nodes: [
          { id: "h0", x: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
          { id: "h1", x: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
        ],
        elements: [{ id: "he0", node_i: 0, node_j: 1, area: 0.02, conductivity: 45 }],
      }),
    },
  },
  heat_plane_triangle_2d: {
    artifacts: {
      heat_plane_triangle_2d_model: sampleInputArtifact("study_model/heat_plane_triangle_2d", {
        nodes: [
          { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
          { id: "h1", x: 1, y: 0, fix_temperature: true, temperature: 20, heat_load: 0 },
          { id: "h2", x: 0, y: 1, fix_temperature: false, temperature: 0, heat_load: 0 },
        ],
        elements: [{ id: "ht0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, conductivity: 45 }],
      }),
    },
  },
  thermal_truss_2d: {
    artifacts: {
      thermal_truss_2d_model: sampleInputArtifact("study_model/thermal_truss_2d", {
        nodes: [
          { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 20 },
          { id: "n1", x: 1, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
          { id: "n2", x: 0.5, y: 0.8, fix_x: false, fix_y: false, load_x: 0, load_y: 0, temperature_delta: 40 },
        ],
        elements: [
          { id: "tt0", node_i: 0, node_j: 2, area: 0.01, youngs_modulus: 210000000000, thermal_expansion: 0.000012 },
          { id: "tt1", node_i: 1, node_j: 2, area: 0.01, youngs_modulus: 210000000000, thermal_expansion: 0.000012 },
        ],
      }),
    },
  },
  torsion_1d: {
    artifacts: {
      torsion_1d_model: sampleInputArtifact("study_model/torsion_1d", {
        nodes: [
          { id: "t0", x: 0, fix_rz: true, torque_z: 0 },
          { id: "t1", x: 1, fix_rz: false, torque_z: 500 },
        ],
        elements: [{ id: "te0", node_i: 0, node_j: 1, shear_modulus: 80000000000, polar_moment: 0.000005, section_modulus: 0.00016 }],
      }),
    },
  },
  plane_triangle_2d: {
    artifacts: {
      plane_triangle_2d_model: sampleInputArtifact("study_model/plane_triangle_2d", {
        nodes: [
          { id: "p0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
          { id: "p1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
          { id: "p2", x: 0, y: 1, fix_x: false, fix_y: false, load_x: 0, load_y: -1000 },
        ],
        elements: [{ id: "pt0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, youngs_modulus: 70000000000, poisson_ratio: 0.33 }],
      }),
    },
  },
  thermal_plane_triangle_2d: {
    artifacts: {
      thermal_plane_triangle_2d_model: sampleInputArtifact("study_model/thermal_plane_triangle_2d", {
        nodes: [
          { id: "tp0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 20 },
          { id: "tp1", x: 1, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
          { id: "tp2", x: 0, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 40 },
        ],
        elements: [{ id: "tpt0", node_i: 0, node_j: 1, node_k: 2, thickness: 0.02, youngs_modulus: 70000000000, poisson_ratio: 0.33, thermal_expansion: 0.000011 }],
      }),
    },
  },
  plane_quad_2d: {
    artifacts: {
      plane_quad_2d_model: sampleInputArtifact("study_model/plane_quad_2d", {
        nodes: [
          { id: "q0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0 },
          { id: "q1", x: 1, y: 0, fix_x: false, fix_y: true, load_x: 0, load_y: 0 },
          { id: "q2", x: 1, y: 1, fix_x: false, fix_y: false, load_x: 0, load_y: -1000 },
          { id: "q3", x: 0, y: 1, fix_x: true, fix_y: false, load_x: 0, load_y: 0 },
        ],
        elements: [{ id: "pq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.02, youngs_modulus: 70000000000, poisson_ratio: 0.33 }],
      }),
    },
  },
};

const SAMPLE_INPUT_ALIASES: Record<string, keyof typeof SAMPLE_INPUTS> = {
  "workflow.electrostatic-to-heat-quad-2d": "electrostatic_plane_quad",
  "workflow.electrostatic-plane-quad-2d": "electrostatic_plane_quad",
  "workflow.electrostatic-plane-quad-field-statistics-json": "electrostatic_plane_quad",
  "workflow.electrostatic-preheat-guard-markdown": "electrostatic_plane_quad",
  "workflow.electrostatic-preheat-guard-heat-json": "electrostatic_plane_quad",
  "workflow.electrostatic-preheat-guard-heat-thermo-json": "electrostatic_plane_quad",
  "solve.electrostatic_plane_quad_2d": "electrostatic_plane_quad",
  "workflow.electrostatic-to-heat-triangle-2d": "electrostatic_plane_triangle",
  "workflow.electrostatic-plane-triangle-summary-json": "electrostatic_plane_triangle",
  "workflow.electrostatic-triangle-preheat-guard-markdown": "electrostatic_plane_triangle",
  "workflow.electrostatic-triangle-preheat-guard-heat-json": "electrostatic_plane_triangle",
  "workflow.electrostatic-triangle-preheat-guard-heat-thermo-json":
    "electrostatic_plane_triangle",
  "electrostatic_triangle_summary": "electrostatic_plane_triangle",
  "electrostatic_triangle_field_statistics": "electrostatic_plane_triangle",
  "electrostatic_triangle_hotspot_alert": "electrostatic_plane_triangle",
  "electrostatic_triangle_hotspot_guard": "electrostatic_plane_triangle",
  "electrostatic_triangle_preheat_guard": "electrostatic_plane_triangle",
  "electrostatic_triangle_heat_thermo_triangle_summary": "electrostatic_plane_triangle",
  "workflow.electrostatic-heat-thermo-triangle-summary-json": "electrostatic_plane_triangle",
  "solve.electrostatic_plane_triangle_2d": "electrostatic_plane_triangle",
  "workflow.heat-to-thermo-quad-2d": "heat_plane_quad",
  "electrostatic_heat_thermo_summary": "electrostatic_plane_quad",
  "electrostatic_field_statistics": "electrostatic_plane_quad",
  "electrostatic_hotspot_alert": "electrostatic_plane_quad",
  "electrostatic_hotspot_guard": "electrostatic_plane_quad",
  "electrostatic_preheat_guard": "electrostatic_plane_quad",
  "electrostatic_quad_triangle_compare": "electrostatic_quad_triangle_pair",
  "workflow.electrostatic-quad-triangle-compare-json": "electrostatic_quad_triangle_pair",
  "workflow.electrostatic-heat-thermo-summary-json": "electrostatic_plane_quad",
  "solve.heat_plane_quad_2d": "heat_plane_quad",
  "workflow.diagnostics-bundle-guard-report-markdown": "diagnostics_bundle_guard_report",
  "workflow.peak-diagnostics-bundle-report-markdown": "peak_diagnostics_bundle_guard_report",
  "workflow.bar-1d-summary-json": "bar_1d",
  "solve.bar_1d": "bar_1d",
  "workflow.thermal-bar-1d-summary-json": "thermal_bar_1d",
  "solve.thermal_bar_1d": "thermal_bar_1d",
  "workflow.heat-bar-1d-summary-json": "heat_bar_1d",
  "solve.heat_bar_1d": "heat_bar_1d",
  "workflow.heat-plane-triangle-summary-json": "heat_plane_triangle_2d",
  "solve.heat_plane_triangle_2d": "heat_plane_triangle_2d",
  "workflow.thermal-truss-2d-summary-json": "thermal_truss_2d",
  "solve.thermal_truss_2d": "thermal_truss_2d",
  "workflow.torsion-1d-summary-json": "torsion_1d",
  "solve.torsion_1d": "torsion_1d",
  "workflow.plane-triangle-2d-summary-json": "plane_triangle_2d",
  "solve.plane_triangle_2d": "plane_triangle_2d",
  "workflow.thermal-plane-triangle-2d-summary-json": "thermal_plane_triangle_2d",
  "solve.thermal_plane_triangle_2d": "thermal_plane_triangle_2d",
  "workflow.plane-quad-2d-summary-json": "plane_quad_2d",
  "solve.plane_quad_2d": "plane_quad_2d",
};

export function builtInWorkflowSampleInputArtifacts(
  workflowId: string,
): Record<string, unknown> | null {
  const sampleKey = SAMPLE_INPUT_ALIASES[workflowId];
  return sampleKey
    ? Object.fromEntries(
        Object.entries(SAMPLE_INPUTS[sampleKey].artifacts).map(([artifactId, entry]) => [
          artifactId,
          entry.payload,
        ]),
      )
    : null;
}

export function builtInWorkflowSampleInputSemanticTypes(
  workflowId: string,
): Record<string, string> | null {
  const sampleKey = SAMPLE_INPUT_ALIASES[workflowId];
  return sampleKey
    ? Object.fromEntries(
        Object.entries(SAMPLE_INPUTS[sampleKey].artifacts).map(([artifactId, entry]) => [
          artifactId,
          entry.semanticType,
        ]),
      )
    : null;
}
