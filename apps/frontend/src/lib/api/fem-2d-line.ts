import { requestJson } from "./core";
import type { JobEnvelope, ModelMaterial } from "./fem-shared";
import { resolveMaterialLookup } from "./fem-shared";

export type ThermalTruss2dNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
  temperature_delta: number;
};

export type ThermalTruss2dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  thermal_expansion: number;
  material_id?: string;
};

export type ThermalTruss2dJobInput = {
  nodes: ThermalTruss2dNodeInput[];
  elements: ThermalTruss2dElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type TrussNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

export type TrussElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  material_id?: string;
};

export type Truss2dJobInput = {
  nodes: TrussNodeInput[];
  elements: TrussElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type Frame2dNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_x: boolean;
  fix_y: boolean;
  fix_rz: boolean;
  load_x: number;
  load_y: number;
  moment_z: number;
};

export type Frame2dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  moment_of_inertia: number;
  section_modulus: number;
  material_id?: string;
};

export type Frame2dJobInput = {
  nodes: Frame2dNodeInput[];
  elements: Frame2dElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type ThermalFrame2dNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_x: boolean;
  fix_y: boolean;
  fix_rz: boolean;
  load_x: number;
  load_y: number;
  moment_z: number;
  temperature_delta: number;
};

export type ThermalFrame2dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  moment_of_inertia: number;
  section_modulus: number;
  thermal_expansion: number;
  section_depth: number;
  temperature_gradient_y: number;
  material_id?: string;
};

export type ThermalFrame2dJobInput = {
  nodes: ThermalFrame2dNodeInput[];
  elements: ThermalFrame2dElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type Spring2dNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

export type Spring2dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  stiffness: number;
};

export type Spring2dJobInput = {
  nodes: Spring2dNodeInput[];
  elements: Spring2dElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type ThermalTruss2dResult = {
  max_displacement: number;
  max_stress: number;
  max_axial_force: number;
  max_temperature_delta: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number; temperature_delta: number }>;
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
  input: ThermalTruss2dJobInput;
};

export type Truss2dResult = {
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    strain: number;
    stress: number;
    axial_force: number;
  }>;
  input: Truss2dJobInput;
};

export type Frame2dResult = {
  max_displacement: number;
  max_rotation: number;
  max_moment: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number; rz: number; displacement_magnitude: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    axial_force_i: number;
    shear_force_i: number;
    moment_i: number;
    axial_force_j: number;
    shear_force_j: number;
    moment_j: number;
    axial_stress: number;
    max_bending_stress: number;
    max_combined_stress: number;
  }>;
  input: Frame2dJobInput;
};

export type ThermalFrame2dResult = {
  max_displacement: number;
  max_rotation: number;
  max_moment: number;
  max_stress: number;
  max_axial_force: number;
  max_temperature_delta: number;
  max_temperature_gradient: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number; rz: number; displacement_magnitude: number; temperature_delta: number }>;
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
    temperature_gradient_y: number;
    thermal_curvature: number;
    axial_force_i: number;
    shear_force_i: number;
    moment_i: number;
    axial_force_j: number;
    shear_force_j: number;
    moment_j: number;
    axial_stress: number;
    max_bending_stress: number;
    max_combined_stress: number;
  }>;
  input: ThermalFrame2dJobInput;
};

export type Spring2dResult = {
  max_displacement: number;
  max_force: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    extension: number;
    force: number;
  }>;
  input: Spring2dJobInput;
};

export function resolveThermalTruss2dJobInput(
  input: ThermalTruss2dJobInput,
): Omit<ThermalTruss2dJobInput, "materials"> {
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

export function resolveTruss2dJobInput(input: Truss2dJobInput): Omit<Truss2dJobInput, "materials"> {
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

export function resolveFrame2dJobInput(input: Frame2dJobInput): Omit<Frame2dJobInput, "materials"> {
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

export function resolveThermalFrame2dJobInput(
  input: ThermalFrame2dJobInput,
): Omit<ThermalFrame2dJobInput, "materials"> {
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

export function resolveSpring2dJobInput(input: Spring2dJobInput): Spring2dJobInput {
  return {
    nodes: input.nodes,
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function createThermalTruss2dJob(
  input: ThermalTruss2dJobInput,
): Promise<JobEnvelope<ThermalTruss2dResult>> {
  return requestJson<JobEnvelope<ThermalTruss2dResult>>("/api/v1/fem/thermal-truss-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createTruss2dJob(input: Truss2dJobInput): Promise<JobEnvelope<Truss2dResult>> {
  return requestJson<JobEnvelope<Truss2dResult>>("/api/v1/fem/truss-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createFrame2dJob(input: Frame2dJobInput): Promise<JobEnvelope<Frame2dResult>> {
  return requestJson<JobEnvelope<Frame2dResult>>("/api/v1/fem/frame-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createThermalFrame2dJob(
  input: ThermalFrame2dJobInput,
): Promise<JobEnvelope<ThermalFrame2dResult>> {
  return requestJson<JobEnvelope<ThermalFrame2dResult>>("/api/v1/fem/thermal-frame-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createSpring2dJob(input: Spring2dJobInput): Promise<JobEnvelope<Spring2dResult>> {
  return requestJson<JobEnvelope<Spring2dResult>>("/api/v1/fem/spring-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}
