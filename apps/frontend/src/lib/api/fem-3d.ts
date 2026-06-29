import { requestJson } from "./core.ts";
import type { JobEnvelope, ModelMaterial } from "./fem-shared.ts";
import { resolveMaterialLookup } from "./fem-shared.ts";

export type Truss3dNodeInput = {
  id: string;
  x: number;
  y: number;
  z: number;
  fix_x: boolean;
  fix_y: boolean;
  fix_z: boolean;
  load_x: number;
  load_y: number;
  load_z: number;
};

export type Truss3dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  material_id?: string;
};

export type Truss3dJobInput = {
  nodes: Truss3dNodeInput[];
  elements: Truss3dElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type ThermalTruss3dNodeInput = {
  id: string;
  x: number;
  y: number;
  z: number;
  fix_x: boolean;
  fix_y: boolean;
  fix_z: boolean;
  load_x: number;
  load_y: number;
  load_z: number;
  temperature_delta: number;
};

export type ThermalTruss3dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  thermal_expansion: number;
  material_id?: string;
};

export type ThermalTruss3dJobInput = {
  nodes: ThermalTruss3dNodeInput[];
  elements: ThermalTruss3dElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type Spring3dNodeInput = {
  id: string;
  x: number;
  y: number;
  z: number;
  fix_x: boolean;
  fix_y: boolean;
  fix_z: boolean;
  load_x: number;
  load_y: number;
  load_z: number;
};

export type Spring3dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  stiffness: number;
};

export type Spring3dJobInput = {
  nodes: Spring3dNodeInput[];
  elements: Spring3dElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type Truss3dResult = {
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; z: number; ux: number; uy: number; uz: number }>;
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
  input: Truss3dJobInput;
};

export type ThermalTruss3dResult = {
  max_displacement: number;
  max_stress: number;
  max_axial_force: number;
  max_temperature_delta: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; z: number; ux: number; uy: number; uz: number; temperature_delta: number }>;
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
  input: ThermalTruss3dJobInput;
};

export type Spring3dResult = {
  max_displacement: number;
  max_force: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; z: number; ux: number; uy: number; uz: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    extension: number;
    force: number;
  }>;
  input: Spring3dJobInput;
};

export function resolveTruss3dJobInput(input: Truss3dJobInput): Omit<Truss3dJobInput, "materials"> {
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

export function resolveThermalTruss3dJobInput(
  input: ThermalTruss3dJobInput,
): Omit<ThermalTruss3dJobInput, "materials"> {
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

export function resolveSpring3dJobInput(input: Spring3dJobInput): Spring3dJobInput {
  return {
    nodes: input.nodes,
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function createTruss3dJob(input: Truss3dJobInput): Promise<JobEnvelope<Truss3dResult>> {
  return requestJson<JobEnvelope<Truss3dResult>>("/api/v1/fem/truss-3d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createThermalTruss3dJob(
  input: ThermalTruss3dJobInput,
): Promise<JobEnvelope<ThermalTruss3dResult>> {
  return requestJson<JobEnvelope<ThermalTruss3dResult>>("/api/v1/fem/thermal-truss-3d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createSpring3dJob(input: Spring3dJobInput): Promise<JobEnvelope<Spring3dResult>> {
  return requestJson<JobEnvelope<Spring3dResult>>("/api/v1/fem/spring-3d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}
