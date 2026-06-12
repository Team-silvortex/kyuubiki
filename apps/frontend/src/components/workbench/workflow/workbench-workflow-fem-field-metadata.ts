"use client";

export type WorkflowFemFieldMetadata = {
  unit: string;
  kind: "material" | "boundary" | "load" | "control";
  summary: string;
};

const FIELD_METADATA: Record<string, WorkflowFemFieldMetadata> = {
  area: { unit: "m^2", kind: "material", summary: "Cross-sectional area." },
  charge_density: { unit: "C/m^3", kind: "load", summary: "Volumetric electric charge density." },
  conductivity: { unit: "W/(m*K)", kind: "material", summary: "Thermal conductivity." },
  elements: { unit: "count", kind: "control", summary: "Requested 1D element count." },
  fix_potential: { unit: "bool", kind: "boundary", summary: "Whether electric potential is clamped." },
  fix_temperature: { unit: "bool", kind: "boundary", summary: "Whether nodal temperature is clamped." },
  fix_x: { unit: "bool", kind: "boundary", summary: "Whether x displacement is restrained." },
  fix_y: { unit: "bool", kind: "boundary", summary: "Whether y displacement is restrained." },
  heat_load: { unit: "W/m^3", kind: "load", summary: "Distributed thermal source intensity." },
  length: { unit: "m", kind: "boundary", summary: "Total physical bar length." },
  load_x: { unit: "N", kind: "load", summary: "Applied nodal force along x." },
  load_y: { unit: "N", kind: "load", summary: "Applied nodal force along y." },
  node_i: { unit: "index", kind: "control", summary: "First connectivity node index." },
  node_j: { unit: "index", kind: "control", summary: "Second connectivity node index." },
  node_k: { unit: "index", kind: "control", summary: "Third connectivity node index." },
  node_l: { unit: "index", kind: "control", summary: "Fourth connectivity node index." },
  permittivity: { unit: "relative", kind: "material", summary: "Relative dielectric permittivity." },
  poisson_ratio: { unit: "ratio", kind: "material", summary: "Poisson's ratio." },
  potential: { unit: "V", kind: "boundary", summary: "Electric potential boundary value." },
  temperature: { unit: "K", kind: "boundary", summary: "Absolute nodal temperature." },
  temperature_delta: { unit: "K", kind: "load", summary: "Applied temperature increment." },
  thermal_expansion: { unit: "1/K", kind: "material", summary: "Linear thermal expansion coefficient." },
  thickness: { unit: "m", kind: "control", summary: "Physical section or plate thickness." },
  tip_force: { unit: "N", kind: "load", summary: "Axial force applied at the bar tip." },
  youngs_modulus: { unit: "Pa", kind: "material", summary: "Elastic stiffness modulus." },
};

export function resolveWorkflowFemFieldMetadata(field: string) {
  return FIELD_METADATA[field] ?? null;
}

export function formatWorkflowFemFieldLabel(field: string) {
  const metadata = resolveWorkflowFemFieldMetadata(field);
  return metadata ? `${field} (${metadata.unit})` : field;
}
