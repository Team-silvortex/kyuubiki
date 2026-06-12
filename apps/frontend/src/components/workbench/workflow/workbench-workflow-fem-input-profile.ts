"use client";

export type WorkflowFemInputSection = {
  key: "material" | "boundary" | "load" | "control";
  label: string;
  summary: string;
  fields: string[];
  target: "nodes" | "elements" | "root";
};

export type WorkflowFemInputProfile = {
  semanticType: string;
  studyFamily: string;
  sections: WorkflowFemInputSection[];
};

export type WorkflowFemInputSectionCoverage = {
  key: WorkflowFemInputSection["key"];
  matchedFields: string[];
};

function buildPlaneProfile(params: {
  semanticType: string;
  studyFamily: string;
  materialFields: string[];
  boundaryFields: string[];
  loadFields: string[];
  controlFields: string[];
}): WorkflowFemInputProfile {
  return {
    semanticType: params.semanticType,
    studyFamily: params.studyFamily,
    sections: [
      { key: "material", label: "Material", summary: "Element-side constitutive properties.", fields: params.materialFields, target: "elements" },
      { key: "boundary", label: "Boundary", summary: "Node-side constraint and fixed-state fields.", fields: params.boundaryFields, target: "nodes" },
      { key: "load", label: "Load", summary: "Node-side source, force, or excitation fields.", fields: params.loadFields, target: "nodes" },
      { key: "control", label: "Control", summary: "Mesh density, thickness, or discretization controls.", fields: params.controlFields, target: "elements" },
    ],
  };
}

const FEM_INPUT_PROFILES: Record<string, WorkflowFemInputProfile> = {
  "study_model/electrostatic_plane_quad_2d": buildPlaneProfile({
    semanticType: "study_model/electrostatic_plane_quad_2d",
    studyFamily: "Electrostatic plane quad",
    materialFields: ["permittivity", "thickness"],
    boundaryFields: ["fix_potential", "potential"],
    loadFields: ["charge_density"],
    controlFields: ["node_i", "node_j", "node_k", "node_l"],
  }),
  "study_model/electrostatic_plane_triangle_2d": buildPlaneProfile({
    semanticType: "study_model/electrostatic_plane_triangle_2d",
    studyFamily: "Electrostatic plane triangle",
    materialFields: ["permittivity", "thickness"],
    boundaryFields: ["fix_potential", "potential"],
    loadFields: ["charge_density"],
    controlFields: ["node_i", "node_j", "node_k"],
  }),
  "study_model/heat_plane_quad_2d": buildPlaneProfile({
    semanticType: "study_model/heat_plane_quad_2d",
    studyFamily: "Heat plane quad",
    materialFields: ["conductivity", "thickness"],
    boundaryFields: ["fix_temperature", "temperature"],
    loadFields: ["heat_load"],
    controlFields: ["node_i", "node_j", "node_k", "node_l"],
  }),
  "study_model/heat_plane_triangle_2d": buildPlaneProfile({
    semanticType: "study_model/heat_plane_triangle_2d",
    studyFamily: "Heat plane triangle",
    materialFields: ["conductivity", "thickness"],
    boundaryFields: ["fix_temperature", "temperature"],
    loadFields: ["heat_load"],
    controlFields: ["node_i", "node_j", "node_k"],
  }),
  "study_model/plane_quad_2d": buildPlaneProfile({
    semanticType: "study_model/plane_quad_2d",
    studyFamily: "Mechanical plane quad",
    materialFields: ["youngs_modulus", "poisson_ratio", "thickness"],
    boundaryFields: ["fix_x", "fix_y"],
    loadFields: ["load_x", "load_y"],
    controlFields: ["node_i", "node_j", "node_k", "node_l"],
  }),
  "study_model/plane_triangle_2d": buildPlaneProfile({
    semanticType: "study_model/plane_triangle_2d",
    studyFamily: "Mechanical plane triangle",
    materialFields: ["youngs_modulus", "poisson_ratio", "thickness"],
    boundaryFields: ["fix_x", "fix_y"],
    loadFields: ["load_x", "load_y"],
    controlFields: ["node_i", "node_j", "node_k"],
  }),
  "study_model/thermal_plane_quad_2d": buildPlaneProfile({
    semanticType: "study_model/thermal_plane_quad_2d",
    studyFamily: "Thermo-mechanical plane quad",
    materialFields: ["youngs_modulus", "poisson_ratio", "thermal_expansion", "thickness"],
    boundaryFields: ["fix_x", "fix_y"],
    loadFields: ["load_x", "load_y", "temperature_delta"],
    controlFields: ["node_i", "node_j", "node_k", "node_l"],
  }),
  "study_model/thermal_plane_triangle_2d": buildPlaneProfile({
    semanticType: "study_model/thermal_plane_triangle_2d",
    studyFamily: "Thermo-mechanical plane triangle",
    materialFields: ["youngs_modulus", "poisson_ratio", "thermal_expansion", "thickness"],
    boundaryFields: ["fix_x", "fix_y"],
    loadFields: ["load_x", "load_y", "temperature_delta"],
    controlFields: ["node_i", "node_j", "node_k"],
  }),
  "study_model/bar_1d": {
    semanticType: "study_model/bar_1d",
    studyFamily: "Bar 1D",
    sections: [
      { key: "material", label: "Material", summary: "Scalar axial constitutive properties.", fields: ["youngs_modulus", "area"], target: "root" },
      { key: "boundary", label: "Boundary", summary: "Implicit root restraint and support assumptions.", fields: ["length"], target: "root" },
      { key: "load", label: "Load", summary: "Global axial load controls.", fields: ["tip_force"], target: "root" },
      { key: "control", label: "Control", summary: "1D discretization controls.", fields: ["elements"], target: "root" },
    ],
  },
  "study_model/heat_bar_1d": {
    semanticType: "study_model/heat_bar_1d",
    studyFamily: "Heat bar 1D",
    sections: [
      { key: "material", label: "Material", summary: "Conduction properties per element.", fields: ["conductivity", "area"], target: "elements" },
      { key: "boundary", label: "Boundary", summary: "Fixed-temperature node fields.", fields: ["fix_temperature", "temperature"], target: "nodes" },
      { key: "load", label: "Load", summary: "Distributed heat source fields.", fields: ["heat_load"], target: "nodes" },
      { key: "control", label: "Control", summary: "1D topology and spacing controls.", fields: ["node_i", "node_j"], target: "elements" },
    ],
  },
  "study_model/thermal_bar_1d": {
    semanticType: "study_model/thermal_bar_1d",
    studyFamily: "Thermal bar 1D",
    sections: [
      { key: "material", label: "Material", summary: "Axial and thermal-expansion properties.", fields: ["youngs_modulus", "area", "thermal_expansion"], target: "elements" },
      { key: "boundary", label: "Boundary", summary: "Support constraints per node.", fields: ["fix_x"], target: "nodes" },
      { key: "load", label: "Load", summary: "Mechanical and thermal excitations.", fields: ["load_x", "temperature_delta"], target: "nodes" },
      { key: "control", label: "Control", summary: "1D topology and connectivity.", fields: ["node_i", "node_j"], target: "elements" },
    ],
  },
};

export function resolveWorkflowFemInputProfile(
  artifactType: string,
): WorkflowFemInputProfile | null {
  return FEM_INPUT_PROFILES[artifactType] ?? null;
}

export function summarizeWorkflowFemInputCoverage(
  artifactType: string,
  payload: unknown,
): WorkflowFemInputSectionCoverage[] {
  const profile = resolveWorkflowFemInputProfile(artifactType);
  if (!profile || typeof payload !== "object" || payload === null) return [];
  const fields = collectPresentFields(payload);
  return profile.sections.map((section) => ({
    key: section.key,
    matchedFields: section.fields.filter((field) => fields.has(field)),
  }));
}

const FIELD_DEFAULTS: Record<string, boolean | number> = {
  fix_potential: false,
  potential: 0,
  charge_density: 0,
  conductivity: 0,
  thickness: 0,
  fix_temperature: false,
  temperature: 0,
  heat_load: 0,
  youngs_modulus: 0,
  poisson_ratio: 0,
  fix_x: false,
  fix_y: false,
  load_x: 0,
  load_y: 0,
  thermal_expansion: 0,
  temperature_delta: 0,
  area: 0,
  length: 0,
  tip_force: 0,
  elements: 1,
  node_i: 0,
  node_j: 0,
  node_k: 0,
  node_l: 0,
};

export function applyWorkflowFemSectionDefaults(
  artifactType: string,
  payload: unknown,
  sectionKey: WorkflowFemInputSection["key"],
) {
  const profile = resolveWorkflowFemInputProfile(artifactType);
  if (!profile || typeof payload !== "object" || payload === null) return payload;
  const section = profile.sections.find((entry) => entry.key === sectionKey);
  if (!section) return payload;
  const next = JSON.parse(JSON.stringify(payload)) as Record<string, unknown>;
  if (section.target === "root") {
    for (const field of section.fields) {
      if (!(field in next)) next[field] = FIELD_DEFAULTS[field] ?? 0;
    }
    return next;
  }

  const collection = next[section.target];
  if (!Array.isArray(collection) || collection.length === 0) return next;
  next[section.target] = collection.map((entry) => {
    if (typeof entry !== "object" || entry === null) return entry;
    const record = { ...(entry as Record<string, unknown>) };
    for (const field of section.fields) {
      if (!(field in record)) record[field] = FIELD_DEFAULTS[field] ?? 0;
    }
    return record;
  });
  return next;
}

export function applyWorkflowFemFieldDefault(
  artifactType: string,
  payload: unknown,
  sectionKey: WorkflowFemInputSection["key"],
  field: string,
) {
  const profile = resolveWorkflowFemInputProfile(artifactType);
  if (!profile || typeof payload !== "object" || payload === null) return payload;
  const section = profile.sections.find((entry) => entry.key === sectionKey);
  if (!section || !section.fields.includes(field)) return payload;
  const next = JSON.parse(JSON.stringify(payload)) as Record<string, unknown>;
  if (section.target === "root") {
    if (!(field in next)) next[field] = FIELD_DEFAULTS[field] ?? 0;
    return next;
  }

  const collection = next[section.target];
  if (!Array.isArray(collection) || collection.length === 0) return next;
  next[section.target] = collection.map((entry) => {
    if (typeof entry !== "object" || entry === null) return entry;
    const record = { ...(entry as Record<string, unknown>) };
    if (!(field in record)) record[field] = FIELD_DEFAULTS[field] ?? 0;
    return record;
  });
  return next;
}

function collectPresentFields(payload: unknown): Set<string> {
  const found = new Set<string>();
  walkPayload(payload, found);
  return found;
}

function walkPayload(value: unknown, found: Set<string>) {
  if (Array.isArray(value)) {
    value.forEach((entry) => walkPayload(entry, found));
    return;
  }
  if (typeof value !== "object" || value === null) return;
  for (const [key, nested] of Object.entries(value)) {
    found.add(key);
    walkPayload(nested, found);
  }
}
