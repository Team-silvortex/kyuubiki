"use client";

export function builtInWorkflowSampleInputArtifacts(workflowId: string): Record<string, unknown> | null {
  if (workflowId !== "workflow.heat-to-thermo-quad-2d") {
    return null;
  }

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
