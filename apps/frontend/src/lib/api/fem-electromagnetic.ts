import { requestJson } from "./core";
import type { JobEnvelope } from "./fem-shared";

export type MagnetostaticBar1dNodeInput = {
  id: string;
  x: number;
  fix_magnetic_potential: boolean;
  magnetic_potential?: number;
  magnetomotive_source?: number;
};

export type MagnetostaticBar1dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  permeability: number;
};

export type MagnetostaticBar1dJobInput = {
  nodes: MagnetostaticBar1dNodeInput[];
  elements: MagnetostaticBar1dElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type MagnetostaticPlaneNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_vector_potential: boolean;
  vector_potential?: number;
  current_density?: number;
};

export type MagnetostaticPlaneTriangleElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  thickness: number;
  permeability: number;
};

export type MagnetostaticPlaneQuadElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  node_l: number;
  thickness: number;
  permeability: number;
};

export type MagnetostaticPlaneTriangle2dJobInput = {
  nodes: MagnetostaticPlaneNodeInput[];
  elements: MagnetostaticPlaneTriangleElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type MagnetostaticPlaneQuad2dJobInput = {
  nodes: MagnetostaticPlaneNodeInput[];
  elements: MagnetostaticPlaneQuadElementInput[];
  project_id?: string;
  model_version_id?: string;
};

export type MagnetostaticBar1dResult = {
  max_magnetic_potential: number;
  max_magnetic_field_strength: number;
  max_flux_density: number;
  total_stored_energy: number;
  nodes: Array<{
    index: number;
    id: string;
    x: number;
    magnetic_potential: number;
    magnetomotive_source: number;
  }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    average_magnetic_potential: number;
    magnetic_potential_gradient: number;
    magnetic_field_strength: number;
    magnetic_flux_density: number;
    stored_energy: number;
  }>;
  input: MagnetostaticBar1dJobInput;
};

export type MagnetostaticPlaneTriangle2dResult = {
  max_vector_potential: number;
  max_magnetic_field_strength: number;
  max_flux_density: number;
  total_stored_energy: number;
  nodes: MagnetostaticPlaneNodeResult[];
  elements: Array<MagnetostaticPlaneTriangleElementResult>;
  input: MagnetostaticPlaneTriangle2dJobInput;
};

export type MagnetostaticPlaneQuad2dResult = {
  max_vector_potential: number;
  max_magnetic_field_strength: number;
  max_flux_density: number;
  total_stored_energy: number;
  nodes: MagnetostaticPlaneNodeResult[];
  elements: Array<MagnetostaticPlaneQuadElementResult>;
  input: MagnetostaticPlaneQuad2dJobInput;
};

export type MagnetostaticPlaneNodeResult = {
  index: number;
  id: string;
  x: number;
  y: number;
  vector_potential: number;
  current_density: number;
};

export type MagnetostaticPlaneTriangleElementResult = {
  index: number;
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  area: number;
  average_vector_potential: number;
  vector_potential_gradient_x: number;
  vector_potential_gradient_y: number;
  magnetic_field_strength_x: number;
  magnetic_field_strength_y: number;
  magnetic_field_strength_magnitude: number;
  magnetic_flux_density_x: number;
  magnetic_flux_density_y: number;
  magnetic_flux_density_magnitude: number;
  stored_energy: number;
};

export type MagnetostaticPlaneQuadElementResult = MagnetostaticPlaneTriangleElementResult & {
  node_l: number;
};

export function resolveMagnetostaticBar1dJobInput(
  input: MagnetostaticBar1dJobInput,
): MagnetostaticBar1dJobInput {
  return {
    nodes: input.nodes.map((node) => ({
      ...node,
      magnetic_potential: node.magnetic_potential ?? 0,
      magnetomotive_source: node.magnetomotive_source ?? 0,
    })),
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveMagnetostaticPlaneTriangle2dJobInput(
  input: MagnetostaticPlaneTriangle2dJobInput,
): MagnetostaticPlaneTriangle2dJobInput {
  return resolveMagnetostaticPlaneJobInput(input);
}

export function resolveMagnetostaticPlaneQuad2dJobInput(
  input: MagnetostaticPlaneQuad2dJobInput,
): MagnetostaticPlaneQuad2dJobInput {
  return resolveMagnetostaticPlaneJobInput(input);
}

export function createMagnetostaticBar1dJob(
  input: MagnetostaticBar1dJobInput,
): Promise<JobEnvelope<MagnetostaticBar1dResult>> {
  return requestJson<JobEnvelope<MagnetostaticBar1dResult>>("/api/v1/fem/magnetostatic-bar-1d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(resolveMagnetostaticBar1dJobInput(input)),
  });
}

export function createMagnetostaticPlaneTriangle2dJob(
  input: MagnetostaticPlaneTriangle2dJobInput,
): Promise<JobEnvelope<MagnetostaticPlaneTriangle2dResult>> {
  return requestJson<JobEnvelope<MagnetostaticPlaneTriangle2dResult>>(
    "/api/v1/fem/magnetostatic-plane-triangle-2d/jobs",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(resolveMagnetostaticPlaneTriangle2dJobInput(input)),
    },
  );
}

export function createMagnetostaticPlaneQuad2dJob(
  input: MagnetostaticPlaneQuad2dJobInput,
): Promise<JobEnvelope<MagnetostaticPlaneQuad2dResult>> {
  return requestJson<JobEnvelope<MagnetostaticPlaneQuad2dResult>>(
    "/api/v1/fem/magnetostatic-plane-quad-2d/jobs",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(resolveMagnetostaticPlaneQuad2dJobInput(input)),
    },
  );
}

function resolveMagnetostaticPlaneJobInput<
  TInput extends MagnetostaticPlaneTriangle2dJobInput | MagnetostaticPlaneQuad2dJobInput,
>(input: TInput): TInput {
  return {
    nodes: input.nodes.map((node) => ({
      ...node,
      vector_potential: node.vector_potential ?? 0,
      current_density: node.current_density ?? 0,
    })),
    elements: input.elements,
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  } as TInput;
}
