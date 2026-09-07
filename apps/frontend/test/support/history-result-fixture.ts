import * as defaults from "../../src/components/workbench/workbench-defaults.ts";

export const historyInputs = {
  axial_bar_1d: { length: 1.2, area: 0.01, elements: 6, tip_force: 1800, youngs_modulus: 210e9 },
  heat_bar_1d: defaults.defaultHeatBar1d,
  heat_plane_triangle_2d: defaults.defaultHeatPlaneTriangle,
  heat_plane_quad_2d: defaults.defaultHeatPlaneQuad,
  electrostatic_plane_triangle_2d: defaults.defaultElectrostaticPlaneTriangle,
  electrostatic_plane_quad_2d: defaults.defaultElectrostaticPlaneQuad,
  thermal_bar_1d: defaults.defaultThermalBar1d,
  thermal_beam_1d: defaults.defaultThermalBeam1d,
  thermal_frame_2d: defaults.defaultThermalFrame2d,
  thermal_truss_2d: defaults.defaultThermalTruss2d,
  thermal_truss_3d: defaults.defaultThermalTruss3d,
  thermal_plane_triangle_2d: defaults.defaultThermalPlaneTriangle,
  thermal_plane_quad_2d: defaults.defaultThermalPlaneQuad,
  spring_1d: defaults.defaultSpring1d,
  spring_2d: defaults.defaultSpring2d,
  spring_3d: defaults.defaultSpring3d,
  beam_1d: defaults.defaultBeam1d,
  torsion_1d: defaults.defaultTorsion1d,
  truss_2d: defaults.defaultTruss,
  truss_3d: defaults.defaultTruss3d,
  plane_triangle_2d: defaults.defaultPlaneTriangle,
  plane_quad_2d: defaults.defaultPlaneQuad,
  frame_2d: defaults.defaultFrame2d,
};

// Shape fixtures exercise archive routing, not numerical accuracy.
export function historyResult(kind: keyof typeof historyInputs): any {
  const input: any = structuredClone(historyInputs[kind]);
  if (kind === "axial_bar_1d") return { input, tip_displacement: 0, reaction_force: 0,
    max_displacement: 0, max_stress: 0, nodes: [{ index: 0, x: 0, displacement: 0 }], elements: [] };
  const result: any = { input,
    nodes: input.nodes.map((node: any, index: number) => ({ ...node, index })),
    elements: input.elements.map((element: any, index: number) => ({ ...element, index })),
  };
  if (kind.startsWith("heat_")) Object.assign(result, { max_temperature: 100, max_heat_flux: 1 });
  else if (kind.startsWith("electrostatic_")) Object.assign(result, { max_potential: 1, max_electric_field: 1, max_flux_density: 1 });
  else {
    result.max_displacement = 0;
    if (kind.startsWith("spring_")) result.max_force = 0;
    else result.max_stress = 0;
    if (kind.includes("beam") || kind.includes("frame")) Object.assign(result, { max_rotation: 0, max_moment: 0 });
    if (kind === "torsion_1d") Object.assign(result, { max_rotation: 0, max_torque: 0 });
    if (kind.startsWith("thermal_")) {
      if (!kind.includes("beam")) Object.assign(result, { max_temperature_delta: 0, max_axial_force: 0 });
      if (kind.includes("beam") || kind.includes("frame")) result.max_temperature_gradient = 0;
    }
  }
  return result;
}

export function historyEffects() {
  const writes: Array<[string, any]> = [];
  const names = ["setJob", "setResult", "setStudyKind", "recordHistory", "openWorkspaceStudy", "setMessage",
    "setAxialForm", "setHeatBarModel", "setHeatPlaneModel", "setPlaneResultField", "setThermalBarModel",
    "setThermalBeamModel", "setThermalFrameModel", "setThermalTrussModel", "setThermalTruss3dModel",
    "setSpringModel", "setSpring2dModel", "setSpring3dModel", "setBeamModel", "setTorsionModel",
    "setTrussModel", "setTruss3dModel", "setFrameModel", "setPlaneModel", "setSidebarSection",
    "setWorkflowPanelTab", "setSelectedWorkflowId", "setWorkflowRuns", "detachSavedModel", "commitObservation"];
  const effects: any = { activeMaterial: "210", copy: { historyAction: "history", historyLoaded: "loaded",
    workflowCatalogCompleted: "workflow complete" } };
  for (const name of names) effects[name] = (value: any) => writes.push([name, value]);
  return { effects, writes };
}
