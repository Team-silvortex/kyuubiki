function resolveMaterialLookup(materials) {
  const lookup = new Map();
  for (const entry of Array.isArray(materials) ? materials : []) {
    if (entry && typeof entry.id === "string") lookup.set(entry.id, entry);
  }
  return lookup;
}

function withJobContext(input) {
  return {
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

function mapYoungsModulusElements(elements, materials) {
  const lookup = resolveMaterialLookup(materials);
  return elements.map(({ material_id, ...element }) => ({
    ...element,
    youngs_modulus: material_id ? lookup.get(material_id)?.youngs_modulus ?? element.youngs_modulus : element.youngs_modulus,
  }));
}

function mapPlaneElements(elements, materials) {
  const lookup = resolveMaterialLookup(materials);
  return elements.map(({ material_id, ...element }) => {
    const material = material_id ? lookup.get(material_id) : null;
    return {
      ...element,
      youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
      poisson_ratio:
        material?.poisson_ratio === null || material?.poisson_ratio === undefined
          ? element.poisson_ratio
          : material.poisson_ratio,
    };
  });
}

function inferStudyKindFromModel(modelLike) {
  if (!modelLike || typeof modelLike !== "object") return null;
  if (typeof modelLike.study_kind === "string" && modelLike.study_kind.trim()) return modelLike.study_kind;
  if (typeof modelLike.kind === "string" && modelLike.kind.trim()) return modelLike.kind;
  const metadata = modelLike.analysis_metadata;
  if (metadata && typeof metadata === "object" && typeof metadata.study_kind === "string" && metadata.study_kind.trim()) {
    return metadata.study_kind;
  }
  return null;
}

function resolveAxialBarInput(input) {
  return {
    length: input.length,
    area: input.area,
    elements: input.elements,
    tip_force: input.tip_force,
    youngs_modulus: input.youngs_modulus_gpa * 1.0e9,
  };
}

const STUDY_KIND_RESOLVERS = {
  axial_bar_1d: resolveAxialBarInput,
  thermal_bar_1d: (input) => ({ nodes: input.nodes, elements: input.elements, ...withJobContext(input) }),
  heat_bar_1d: (input) => ({ nodes: input.nodes, elements: input.elements, ...withJobContext(input) }),
  beam_1d: (input) => ({ nodes: input.nodes, elements: mapYoungsModulusElements(input.elements, input.materials), ...withJobContext(input) }),
  thermal_beam_1d: (input) => ({ nodes: input.nodes, elements: mapYoungsModulusElements(input.elements, input.materials), ...withJobContext(input) }),
  torsion_1d: (input) => ({ nodes: input.nodes, elements: input.elements, ...withJobContext(input) }),
  spring_1d: (input) => ({ nodes: input.nodes, elements: input.elements, ...withJobContext(input) }),
  thermal_truss_2d: (input) => ({ nodes: input.nodes, elements: mapYoungsModulusElements(input.elements, input.materials), ...withJobContext(input) }),
  truss_2d: (input) => ({ nodes: input.nodes, elements: mapYoungsModulusElements(input.elements, input.materials), ...withJobContext(input) }),
  frame_2d: (input) => ({ nodes: input.nodes, elements: mapYoungsModulusElements(input.elements, input.materials), ...withJobContext(input) }),
  thermal_frame_2d: (input) => ({ nodes: input.nodes, elements: mapYoungsModulusElements(input.elements, input.materials), ...withJobContext(input) }),
  spring_2d: (input) => ({ nodes: input.nodes, elements: input.elements, ...withJobContext(input) }),
  heat_plane_triangle_2d: (input) => ({
    nodes: input.nodes.map((node) => ({ ...node, temperature: node.temperature ?? 0, heat_load: node.heat_load ?? 0 })),
    elements: input.elements.map(({ material_id, ...element }) => ({ ...element })),
    ...withJobContext(input),
  }),
  heat_plane_quad_2d: (input) => ({
    nodes: input.nodes.map((node) => ({ ...node, temperature: node.temperature ?? 0, heat_load: node.heat_load ?? 0 })),
    elements: input.elements.map(({ material_id, ...element }) => ({ ...element })),
    ...withJobContext(input),
  }),
  plane_triangle_2d: (input) => ({ nodes: input.nodes, elements: mapPlaneElements(input.elements, input.materials), ...withJobContext(input) }),
  thermal_plane_triangle_2d: (input) => ({
    nodes: input.nodes.map((node) => ({ ...node, temperature_delta: node.temperature_delta ?? 0 })),
    elements: mapPlaneElements(input.elements, input.materials),
    ...withJobContext(input),
  }),
  plane_quad_2d: (input) => ({ nodes: input.nodes, elements: mapPlaneElements(input.elements, input.materials), ...withJobContext(input) }),
  thermal_plane_quad_2d: (input) => ({
    nodes: input.nodes.map((node) => ({ ...node, temperature_delta: node.temperature_delta ?? 0 })),
    elements: mapPlaneElements(input.elements, input.materials),
    ...withJobContext(input),
  }),
  truss_3d: (input) => ({ nodes: input.nodes, elements: mapYoungsModulusElements(input.elements, input.materials), ...withJobContext(input) }),
  thermal_truss_3d: (input) => ({ nodes: input.nodes, elements: mapYoungsModulusElements(input.elements, input.materials), ...withJobContext(input) }),
  spring_3d: (input) => ({ nodes: input.nodes, elements: input.elements, ...withJobContext(input) }),
};

export function resolveStudyInput(studyKind, input) {
  const resolver = STUDY_KIND_RESOLVERS[studyKind];
  if (!resolver) throw new Error(`Unsupported study kind "${studyKind}" for headless solve.`);
  return resolver(input);
}

export function resolveStudyKindAndInput(payload, modelLike = null) {
  const studyKind =
    (typeof payload.study_kind === "string" && payload.study_kind.trim()) ||
    inferStudyKindFromModel(modelLike) ||
    inferStudyKindFromModel(payload.model_payload) ||
    null;
  if (!studyKind) {
    throw new Error("direct_mesh_solve requires study_kind, model kind, or model payload metadata.");
  }

  const rawInput =
    (payload.input && typeof payload.input === "object" ? payload.input : null) ??
    (payload.model_payload && typeof payload.model_payload === "object" ? payload.model_payload : null);
  if (!rawInput) {
    throw new Error("direct_mesh_solve requires input, model_payload, model_id, or model_version_id.");
  }

  return {
    studyKind,
    input: resolveStudyInput(studyKind, {
      ...rawInput,
      ...(Array.isArray(payload.materials) ? { materials: payload.materials } : {}),
      ...(typeof payload.project_id === "string" ? { project_id: payload.project_id } : {}),
      ...(typeof payload.model_version_id === "string" ? { model_version_id: payload.model_version_id } : {}),
    }),
  };
}
