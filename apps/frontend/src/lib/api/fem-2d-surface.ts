import { requestJson } from "./core";
import type { JobEnvelope, ModelMaterial } from "./fem-shared";
import { resolveMaterialLookup } from "./fem-shared";

export type HeatPlaneNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_temperature: boolean;
  temperature?: number;
  heat_load?: number;
};

export type PlaneNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

export type ThermalPlaneNodeInput = PlaneNodeInput & {
  temperature_delta?: number;
};

export type HeatPlaneTriangleElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  thickness: number;
  conductivity: number;
  material_id?: string;
};

export type HeatPlaneTriangle2dJobInput = {
  nodes: HeatPlaneNodeInput[];
  elements: HeatPlaneTriangleElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type HeatPlaneQuadElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  node_l: number;
  thickness: number;
  conductivity: number;
  material_id?: string;
};

export type HeatPlaneQuad2dJobInput = {
  nodes: HeatPlaneNodeInput[];
  elements: HeatPlaneQuadElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type ElectrostaticPlaneNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_potential: boolean;
  potential?: number;
  charge_density?: number;
};

export type ElectrostaticPlaneTriangleElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  thickness: number;
  permittivity: number;
  material_id?: string;
};

export type ElectrostaticPlaneTriangle2dJobInput = {
  nodes: ElectrostaticPlaneNodeInput[];
  elements: ElectrostaticPlaneTriangleElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type ElectrostaticPlaneQuadElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  node_l: number;
  thickness: number;
  permittivity: number;
  material_id?: string;
};

export type ElectrostaticPlaneQuad2dJobInput = {
  nodes: ElectrostaticPlaneNodeInput[];
  elements: ElectrostaticPlaneQuadElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type PlaneTriangleElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  thickness: number;
  youngs_modulus: number;
  poisson_ratio: number;
  material_id?: string;
};

export type PlaneQuadElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  node_l: number;
  thickness: number;
  youngs_modulus: number;
  poisson_ratio: number;
  material_id?: string;
};

export type PlaneTriangle2dJobInput = {
  nodes: PlaneNodeInput[];
  elements: PlaneTriangleElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type ThermalPlaneTriangleElementInput = PlaneTriangleElementInput & {
  thermal_expansion: number;
};

export type ThermalPlaneTriangle2dJobInput = {
  nodes: ThermalPlaneNodeInput[];
  elements: ThermalPlaneTriangleElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type PlaneQuad2dJobInput = {
  nodes: PlaneNodeInput[];
  elements: PlaneQuadElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type ThermalPlaneQuadElementInput = PlaneQuadElementInput & {
  thermal_expansion: number;
};

export type ThermalPlaneQuad2dJobInput = {
  nodes: ThermalPlaneNodeInput[];
  elements: ThermalPlaneQuadElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type HeatPlaneTriangle2dResult = {
  max_temperature: number;
  max_heat_flux: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; temperature: number; heat_load: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    area: number;
    average_temperature: number;
    temperature_gradient_x: number;
    temperature_gradient_y: number;
    heat_flux_x: number;
    heat_flux_y: number;
    heat_flux_magnitude: number;
  }>;
  input: HeatPlaneTriangle2dJobInput;
};

export type HeatPlaneQuad2dResult = {
  max_temperature: number;
  max_heat_flux: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; temperature: number; heat_load: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    node_l: number;
    area: number;
    average_temperature: number;
    temperature_gradient_x: number;
    temperature_gradient_y: number;
    heat_flux_x: number;
    heat_flux_y: number;
    heat_flux_magnitude: number;
  }>;
  input: HeatPlaneQuad2dJobInput;
};

export type ElectrostaticPlaneTriangle2dResult = {
  max_potential: number;
  max_electric_field: number;
  max_flux_density: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; potential: number; charge_density: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    area: number;
    average_potential: number;
    potential_gradient_x: number;
    potential_gradient_y: number;
    electric_field_x: number;
    electric_field_y: number;
    electric_field_magnitude: number;
    electric_flux_density_x: number;
    electric_flux_density_y: number;
    electric_flux_density_magnitude: number;
  }>;
  input: ElectrostaticPlaneTriangle2dJobInput;
};

export type ElectrostaticPlaneQuad2dResult = {
  max_potential: number;
  max_electric_field: number;
  max_flux_density: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; potential: number; charge_density: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    node_l: number;
    area: number;
    average_potential: number;
    potential_gradient_x: number;
    potential_gradient_y: number;
    electric_field_x: number;
    electric_field_y: number;
    electric_field_magnitude: number;
    electric_flux_density_x: number;
    electric_flux_density_y: number;
    electric_flux_density_magnitude: number;
  }>;
  input: ElectrostaticPlaneQuad2dJobInput;
};

export type PlaneTriangle2dResult = {
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number; displacement_magnitude: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    area: number;
    strain_x: number;
    strain_y: number;
    gamma_xy: number;
    stress_x: number;
    stress_y: number;
    tau_xy: number;
    principal_stress_1: number;
    principal_stress_2: number;
    max_in_plane_shear: number;
    von_mises: number;
  }>;
  input: PlaneTriangle2dJobInput;
};

export type ThermalPlaneTriangle2dResult = {
  max_displacement: number;
  max_stress: number;
  max_temperature_delta: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number; displacement_magnitude: number; temperature_delta: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    area: number;
    average_temperature_delta: number;
    thermal_strain: number;
    mechanical_strain_x: number;
    mechanical_strain_y: number;
    total_strain_x: number;
    total_strain_y: number;
    gamma_xy: number;
    stress_x: number;
    stress_y: number;
    tau_xy: number;
    principal_stress_1: number;
    principal_stress_2: number;
    max_in_plane_shear: number;
    von_mises: number;
  }>;
  input: ThermalPlaneTriangle2dJobInput;
};

export type PlaneQuad2dResult = {
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number; displacement_magnitude: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    node_l: number;
    area: number;
    strain_x: number;
    strain_y: number;
    gamma_xy: number;
    stress_x: number;
    stress_y: number;
    tau_xy: number;
    principal_stress_1: number;
    principal_stress_2: number;
    max_in_plane_shear: number;
    von_mises: number;
  }>;
  input: PlaneQuad2dJobInput;
};

export type ThermalPlaneQuad2dResult = {
  max_displacement: number;
  max_stress: number;
  max_temperature_delta: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number; displacement_magnitude: number; temperature_delta: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    node_l: number;
    area: number;
    average_temperature_delta: number;
    thermal_strain: number;
    mechanical_strain_x: number;
    mechanical_strain_y: number;
    total_strain_x: number;
    total_strain_y: number;
    gamma_xy: number;
    stress_x: number;
    stress_y: number;
    tau_xy: number;
    principal_stress_1: number;
    principal_stress_2: number;
    max_in_plane_shear: number;
    von_mises: number;
  }>;
  input: ThermalPlaneQuad2dJobInput;
};

export function resolveHeatPlaneTriangle2dJobInput(
  input: HeatPlaneTriangle2dJobInput,
): Omit<HeatPlaneTriangle2dJobInput, "materials"> {
  return {
    nodes: input.nodes.map((node) => ({ ...node, temperature: node.temperature ?? 0, heat_load: node.heat_load ?? 0 })),
    elements: input.elements.map(({ material_id, ...element }) => ({ ...element })),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveHeatPlaneQuad2dJobInput(
  input: HeatPlaneQuad2dJobInput,
): Omit<HeatPlaneQuad2dJobInput, "materials"> {
  return {
    nodes: input.nodes.map((node) => ({ ...node, temperature: node.temperature ?? 0, heat_load: node.heat_load ?? 0 })),
    elements: input.elements.map(({ material_id, ...element }) => ({ ...element })),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveElectrostaticPlaneTriangle2dJobInput(
  input: ElectrostaticPlaneTriangle2dJobInput,
): Omit<ElectrostaticPlaneTriangle2dJobInput, "materials"> {
  return {
    nodes: input.nodes.map((node) => ({ ...node, potential: node.potential ?? 0, charge_density: node.charge_density ?? 0 })),
    elements: input.elements.map(({ material_id, ...element }) => ({ ...element })),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveElectrostaticPlaneQuad2dJobInput(
  input: ElectrostaticPlaneQuad2dJobInput,
): Omit<ElectrostaticPlaneQuad2dJobInput, "materials"> {
  return {
    nodes: input.nodes.map((node) => ({ ...node, potential: node.potential ?? 0, charge_density: node.charge_density ?? 0 })),
    elements: input.elements.map(({ material_id, ...element }) => ({ ...element })),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolvePlaneTriangle2dJobInput(
  input: PlaneTriangle2dJobInput,
): Omit<PlaneTriangle2dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);
  return {
    nodes: input.nodes,
    elements: input.elements.map(({ material_id, ...element }) => {
      const material = material_id ? materials.get(material_id) : null;
      return {
        ...element,
        youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
        poisson_ratio:
          material?.poisson_ratio === null || material?.poisson_ratio === undefined
            ? element.poisson_ratio
            : material.poisson_ratio,
      };
    }),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveThermalPlaneTriangle2dJobInput(
  input: ThermalPlaneTriangle2dJobInput,
): Omit<ThermalPlaneTriangle2dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);
  return {
    nodes: input.nodes.map((node) => ({ ...node, temperature_delta: node.temperature_delta ?? 0 })),
    elements: input.elements.map(({ material_id, ...element }) => {
      const material = material_id ? materials.get(material_id) : null;
      return {
        ...element,
        youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
        poisson_ratio:
          material?.poisson_ratio === null || material?.poisson_ratio === undefined
            ? element.poisson_ratio
            : material.poisson_ratio,
      };
    }),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolvePlaneQuad2dJobInput(
  input: PlaneQuad2dJobInput,
): Omit<PlaneQuad2dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);
  return {
    nodes: input.nodes,
    elements: input.elements.map(({ material_id, ...element }) => {
      const material = material_id ? materials.get(material_id) : null;
      return {
        ...element,
        youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
        poisson_ratio:
          material?.poisson_ratio === null || material?.poisson_ratio === undefined
            ? element.poisson_ratio
            : material.poisson_ratio,
      };
    }),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveThermalPlaneQuad2dJobInput(
  input: ThermalPlaneQuad2dJobInput,
): Omit<ThermalPlaneQuad2dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);
  return {
    nodes: input.nodes.map((node) => ({ ...node, temperature_delta: node.temperature_delta ?? 0 })),
    elements: input.elements.map(({ material_id, ...element }) => {
      const material = material_id ? materials.get(material_id) : null;
      return {
        ...element,
        youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
        poisson_ratio:
          material?.poisson_ratio === null || material?.poisson_ratio === undefined
            ? element.poisson_ratio
            : material.poisson_ratio,
      };
    }),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function createHeatPlaneTriangle2dJob(
  input: HeatPlaneTriangle2dJobInput,
): Promise<JobEnvelope<HeatPlaneTriangle2dResult>> {
  return requestJson<JobEnvelope<HeatPlaneTriangle2dResult>>("/api/v1/fem/heat-plane-triangle-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createHeatPlaneQuad2dJob(
  input: HeatPlaneQuad2dJobInput,
): Promise<JobEnvelope<HeatPlaneQuad2dResult>> {
  return requestJson<JobEnvelope<HeatPlaneQuad2dResult>>("/api/v1/fem/heat-plane-quad-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createElectrostaticPlaneTriangle2dJob(
  input: ElectrostaticPlaneTriangle2dJobInput,
): Promise<JobEnvelope<ElectrostaticPlaneTriangle2dResult>> {
  return requestJson<JobEnvelope<ElectrostaticPlaneTriangle2dResult>>("/api/v1/fem/electrostatic-plane-triangle-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createElectrostaticPlaneQuad2dJob(
  input: ElectrostaticPlaneQuad2dJobInput,
): Promise<JobEnvelope<ElectrostaticPlaneQuad2dResult>> {
  return requestJson<JobEnvelope<ElectrostaticPlaneQuad2dResult>>("/api/v1/fem/electrostatic-plane-quad-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createPlaneTriangle2dJob(
  input: PlaneTriangle2dJobInput,
): Promise<JobEnvelope<PlaneTriangle2dResult>> {
  return requestJson<JobEnvelope<PlaneTriangle2dResult>>("/api/v1/fem/plane-triangle-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createThermalPlaneTriangle2dJob(
  input: ThermalPlaneTriangle2dJobInput,
): Promise<JobEnvelope<ThermalPlaneTriangle2dResult>> {
  return requestJson<JobEnvelope<ThermalPlaneTriangle2dResult>>("/api/v1/fem/thermal-plane-triangle-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createPlaneQuad2dJob(
  input: PlaneQuad2dJobInput,
): Promise<JobEnvelope<PlaneQuad2dResult>> {
  return requestJson<JobEnvelope<PlaneQuad2dResult>>("/api/v1/fem/plane-quad-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createThermalPlaneQuad2dJob(
  input: ThermalPlaneQuad2dJobInput,
): Promise<JobEnvelope<ThermalPlaneQuad2dResult>> {
  return requestJson<JobEnvelope<ThermalPlaneQuad2dResult>>("/api/v1/fem/thermal-plane-quad-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}
