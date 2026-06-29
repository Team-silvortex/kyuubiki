import { requestJson } from "./core.ts";
import type { JobEnvelope, ModelMaterial } from "./fem-shared.ts";
import { resolveMaterialLookup } from "./fem-shared.ts";

export type AxialBarJobInput = {
  length: number;
  area: number;
  elements: number;
  tip_force: number;
  youngs_modulus_gpa: number;
  project_id?: string;
  model_version_id?: string;
};

export type ThermalBar1dNodeInput = {
  id: string;
  x: number;
  fix_x: boolean;
  load_x: number;
  temperature_delta: number;
};

export type HeatBar1dNodeInput = {
  id: string;
  x: number;
  fix_temperature: boolean;
  temperature: number;
  heat_load: number;
};

export type ThermalBar1dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  thermal_expansion: number;
};

export type ThermalBar1dJobInput = {
  nodes: ThermalBar1dNodeInput[];
  elements: ThermalBar1dElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type HeatBar1dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  conductivity: number;
};

export type HeatBar1dJobInput = {
  nodes: HeatBar1dNodeInput[];
  elements: HeatBar1dElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type ElectrostaticBar1dNodeInput = {
  id: string;
  x: number;
  fix_potential: boolean;
  potential: number;
  charge_density: number;
};

export type ElectrostaticBar1dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  permittivity: number;
};

export type ElectrostaticBar1dJobInput = {
  nodes: ElectrostaticBar1dNodeInput[];
  elements: ElectrostaticBar1dElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type Beam1dNodeInput = {
  id: string;
  x: number;
  fix_y: boolean;
  fix_rz: boolean;
  load_y: number;
  moment_z: number;
};

export type Beam1dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  youngs_modulus: number;
  moment_of_inertia: number;
  section_modulus: number;
  distributed_load_y: number;
  material_id?: string;
};

export type Beam1dJobInput = {
  nodes: Beam1dNodeInput[];
  elements: Beam1dElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type ThermalBeam1dNodeInput = {
  id: string;
  x: number;
  fix_y: boolean;
  fix_rz: boolean;
  load_y: number;
  moment_z: number;
};

export type ThermalBeam1dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  youngs_modulus: number;
  moment_of_inertia: number;
  section_modulus: number;
  thermal_expansion: number;
  section_depth: number;
  distributed_load_y: number;
  temperature_gradient_y: number;
  material_id?: string;
};

export type ThermalBeam1dJobInput = {
  nodes: ThermalBeam1dNodeInput[];
  elements: ThermalBeam1dElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type Torsion1dNodeInput = {
  id: string;
  x: number;
  fix_rz: boolean;
  torque_z: number;
};

export type Torsion1dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  shear_modulus: number;
  polar_moment: number;
  section_modulus: number;
};

export type Torsion1dJobInput = {
  nodes: Torsion1dNodeInput[];
  elements: Torsion1dElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type Spring1dNodeInput = {
  id: string;
  x: number;
  fix_x: boolean;
  load_x: number;
};

export type Spring1dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  stiffness: number;
};

export type Spring1dJobInput = {
  nodes: Spring1dNodeInput[];
  elements: Spring1dElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type AxialBarResult = {
  tip_displacement: number;
  reaction_force: number;
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; x: number; displacement: number }>;
  elements: Array<{
    index: number;
    x1: number;
    x2: number;
    strain: number;
    stress: number;
    axial_force: number;
  }>;
  input: {
    length: number;
    area: number;
    elements: number;
    tip_force: number;
    youngs_modulus: number;
  };
};

export type ThermalBar1dResult = {
  max_displacement: number;
  max_stress: number;
  max_axial_force: number;
  max_temperature_delta: number;
  nodes: Array<{ index: number; id: string; x: number; ux: number; temperature_delta: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    average_temperature_delta: number;
    thermal_strain: number;
    mechanical_strain: number;
    total_strain: number;
    stress: number;
    axial_force: number;
  }>;
  input: ThermalBar1dJobInput;
};

export type HeatBar1dResult = {
  max_temperature: number;
  max_heat_flux: number;
  nodes: Array<{ index: number; id: string; x: number; temperature: number; heat_load: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    average_temperature: number;
    temperature_gradient: number;
    heat_flux: number;
  }>;
  input: HeatBar1dJobInput;
};

export type ElectrostaticBar1dResult = {
  max_potential: number;
  max_electric_field: number;
  max_flux_density: number;
  nodes: Array<{ index: number; id: string; x: number; potential: number; charge_density: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    average_potential: number;
    potential_gradient: number;
    electric_field: number;
    electric_flux_density: number;
  }>;
  input: ElectrostaticBar1dJobInput;
};

export type Beam1dResult = {
  max_displacement: number;
  max_rotation: number;
  max_moment: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; uy: number; rz: number; displacement_magnitude: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    shear_force_i: number;
    moment_i: number;
    shear_force_j: number;
    moment_j: number;
    max_bending_stress: number;
  }>;
  input: Beam1dJobInput;
};

export type ThermalBeam1dResult = {
  max_displacement: number;
  max_rotation: number;
  max_moment: number;
  max_stress: number;
  max_temperature_gradient: number;
  nodes: Array<{ index: number; id: string; x: number; uy: number; rz: number; displacement_magnitude: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    temperature_gradient_y: number;
    thermal_curvature: number;
    shear_force_i: number;
    moment_i: number;
    shear_force_j: number;
    moment_j: number;
    max_bending_stress: number;
  }>;
  input: ThermalBeam1dJobInput;
};

export type Torsion1dResult = {
  max_rotation: number;
  max_torque: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; rz: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    twist: number;
    torque: number;
    shear_stress: number;
  }>;
  input: Torsion1dJobInput;
};

export type Spring1dResult = {
  max_displacement: number;
  max_force: number;
  nodes: Array<{ index: number; id: string; x: number; ux: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    extension: number;
    force: number;
  }>;
  input: Spring1dJobInput;
};

export function resolveAxialBarJobInput(input: AxialBarJobInput) {
  return {
    length: input.length,
    area: input.area,
    elements: input.elements,
    tip_force: input.tip_force,
    youngs_modulus: input.youngs_modulus_gpa * 1.0e9,
  };
}

export function resolveThermalBar1dJobInput(input: ThermalBar1dJobInput): ThermalBar1dJobInput {
  return {
    nodes: input.nodes,
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveHeatBar1dJobInput(input: HeatBar1dJobInput): HeatBar1dJobInput {
  return {
    nodes: input.nodes,
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveElectrostaticBar1dJobInput(
  input: ElectrostaticBar1dJobInput,
): ElectrostaticBar1dJobInput {
  return {
    nodes: input.nodes,
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveBeam1dJobInput(input: Beam1dJobInput): Omit<Beam1dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);
  return {
    nodes: input.nodes,
    elements: input.elements.map(({ material_id, ...element }) => ({
      ...element,
      youngs_modulus: material_id ? materials.get(material_id)?.youngs_modulus ?? element.youngs_modulus : element.youngs_modulus,
    })),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveThermalBeam1dJobInput(
  input: ThermalBeam1dJobInput,
): Omit<ThermalBeam1dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);
  return {
    nodes: input.nodes,
    elements: input.elements.map(({ material_id, ...element }) => ({
      ...element,
      youngs_modulus: material_id ? materials.get(material_id)?.youngs_modulus ?? element.youngs_modulus : element.youngs_modulus,
    })),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveTorsion1dJobInput(input: Torsion1dJobInput): Torsion1dJobInput {
  return {
    nodes: input.nodes,
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveSpring1dJobInput(input: Spring1dJobInput): Spring1dJobInput {
  return {
    nodes: input.nodes,
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function createAxialBarJob(input: AxialBarJobInput): Promise<JobEnvelope<AxialBarResult>> {
  return requestJson<JobEnvelope<AxialBarResult>>("/api/v1/fem/axial-bar/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createThermalBar1dJob(
  input: ThermalBar1dJobInput,
): Promise<JobEnvelope<ThermalBar1dResult>> {
  return requestJson<JobEnvelope<ThermalBar1dResult>>("/api/v1/fem/thermal-bar-1d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createHeatBar1dJob(input: HeatBar1dJobInput): Promise<JobEnvelope<HeatBar1dResult>> {
  return requestJson<JobEnvelope<HeatBar1dResult>>("/api/v1/fem/heat-bar-1d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createElectrostaticBar1dJob(
  input: ElectrostaticBar1dJobInput,
): Promise<JobEnvelope<ElectrostaticBar1dResult>> {
  return requestJson<JobEnvelope<ElectrostaticBar1dResult>>("/api/v1/fem/electrostatic-bar-1d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createBeam1dJob(input: Beam1dJobInput): Promise<JobEnvelope<Beam1dResult>> {
  return requestJson<JobEnvelope<Beam1dResult>>("/api/v1/fem/beam-1d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createThermalBeam1dJob(
  input: ThermalBeam1dJobInput,
): Promise<JobEnvelope<ThermalBeam1dResult>> {
  return requestJson<JobEnvelope<ThermalBeam1dResult>>("/api/v1/fem/thermal-beam-1d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createTorsion1dJob(input: Torsion1dJobInput): Promise<JobEnvelope<Torsion1dResult>> {
  return requestJson<JobEnvelope<Torsion1dResult>>("/api/v1/fem/torsion-1d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createSpring1dJob(input: Spring1dJobInput): Promise<JobEnvelope<Spring1dResult>> {
  return requestJson<JobEnvelope<Spring1dResult>>("/api/v1/fem/spring-1d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}
