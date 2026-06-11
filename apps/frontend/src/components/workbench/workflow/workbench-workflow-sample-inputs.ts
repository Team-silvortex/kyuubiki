"use client";

export function builtInWorkflowSampleInputArtifacts(workflowId: string): Record<string, unknown> | null {
  if (workflowId === "workflow.electrostatic-to-heat-quad-2d") {
    return {
      electrostatic_model: {
        nodes: [
          { id: "n0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
          { id: "n1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "n2", x: 1, y: 1, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "n3", x: 0, y: 1, fix_potential: true, potential: 10, charge_density: 0 },
        ],
        elements: [
          {
            id: "epq0",
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.05,
            permittivity: 2,
          },
        ],
      },
    };
  }

  if (workflowId === "workflow.electrostatic-plane-quad-2d") {
    return {
      electrostatic_model: {
        nodes: [
          { id: "n0", x: 0, y: 0, fix_potential: true, potential: 10, charge_density: 0 },
          { id: "n1", x: 1, y: 0, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "n2", x: 1, y: 1, fix_potential: true, potential: 0, charge_density: 0 },
          { id: "n3", x: 0, y: 1, fix_potential: true, potential: 10, charge_density: 0 },
        ],
        elements: [
          {
            id: "epq0",
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.05,
            permittivity: 2,
          },
        ],
      },
    };
  }

  if (workflowId === "workflow.heat-to-thermo-quad-2d") {
    return {
      heat_model: {
        nodes: [
          { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
          { id: "h1", x: 1, y: 0, fix_temperature: false, temperature: 0, heat_load: 0 },
          { id: "h2", x: 1, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
          { id: "h3", x: 0, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
        ],
        elements: [
          {
            id: "hq0",
            node_i: 0,
            node_j: 1,
            node_k: 2,
            node_l: 3,
            thickness: 0.02,
            conductivity: 45,
          },
        ],
      },
    };
  }

  return null;
}
