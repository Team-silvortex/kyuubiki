import type {
  AxialBarJobInput,
  Beam1dJobInput,
  HeatBar1dJobInput,
  Spring1dJobInput,
  Spring2dJobInput,
  Spring3dJobInput,
  ThermalBar1dJobInput,
  ThermalBeam1dJobInput,
  Torsion1dJobInput,
} from "@/lib/api";
import type {
  ImportedAxialBarModel,
  ImportedBeam1dModel,
  ImportedHeatBar1dModel,
  ImportedSpring1dModel,
  ImportedSpring2dModel,
  ImportedSpring3dModel,
  ImportedThermalBar1dModel,
  ImportedThermalBeam1dModel,
  ImportedTorsion1dModel,
} from "@/lib/models/model-import-types";
import {
  normalizeMaterial,
  numberOrZero,
  optionalString,
  parseMaterials,
  requiredNonNegativeInteger,
  requiredNumber,
  requiredString,
} from "@/lib/models/model-import-utils";

export function parseAxialBarV1(raw: Record<string, unknown>): ImportedAxialBarModel {
  return {
    kind: "axial_bar_1d",
    name: typeof raw.name === "string" ? raw.name : "imported-model",
    length: requiredNumber(raw.length, "length"),
    area: requiredNumber(raw.area, "area"),
    elements: Math.trunc(requiredNumber(raw.elements, "elements")),
    tipForce: numberOrZero(raw.tip_force ?? raw.tipForce),
    material: normalizeMaterial(raw.material),
    youngsModulusGpa: requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa"),
  };
}

function parseThermalBar1dNode(raw: unknown, index: number): ThermalBar1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), fix_x: Boolean(node.fix_x), load_x: numberOrZero(node.load_x), temperature_delta: numberOrZero(node.temperature_delta) };
}
function parseThermalBar1dElement(raw: unknown, index: number): ThermalBar1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), area: requiredNumber(element.area, `elements[${index}].area`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), thermal_expansion: numberOrZero(element.thermal_expansion) };
}
export function parseThermalBar1dV1(raw: Record<string, unknown>): ImportedThermalBar1dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalBar1dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseThermalBar1dElement) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "thermal_bar_1d", name: typeof raw.name === "string" ? raw.name : "imported-thermal-bar-1d", model: { nodes, elements } };
}

function parseHeatBar1dNode(raw: unknown, index: number): HeatBar1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), fix_temperature: Boolean(node.fix_temperature), temperature: numberOrZero(node.temperature), heat_load: numberOrZero(node.heat_load) };
}
function parseHeatBar1dElement(raw: unknown, index: number): HeatBar1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), area: requiredNumber(element.area, `elements[${index}].area`), conductivity: requiredNumber(element.conductivity, `elements[${index}].conductivity`) };
}
export function parseHeatBar1dV1(raw: Record<string, unknown>): ImportedHeatBar1dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseHeatBar1dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseHeatBar1dElement) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "heat_bar_1d", name: typeof raw.name === "string" ? raw.name : "imported-heat-bar-1d", model: { nodes, elements } };
}

function parseBeam1dNode(raw: unknown, index: number): Beam1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), fix_y: Boolean(node.fix_y), fix_rz: Boolean(node.fix_rz), load_y: numberOrZero(node.load_y), moment_z: numberOrZero(node.moment_z) };
}
function parseBeam1dElement(raw: unknown, index: number): Beam1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), moment_of_inertia: requiredNumber(element.moment_of_inertia, `elements[${index}].moment_of_inertia`), section_modulus: requiredNumber(element.section_modulus, `elements[${index}].section_modulus`), distributed_load_y: numberOrZero(element.distributed_load_y), material_id: optionalString(element.material_id) };
}
export function parseBeam1dV1(raw: Record<string, unknown>): ImportedBeam1dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseBeam1dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseBeam1dElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "beam_1d", name: typeof raw.name === "string" ? raw.name : "imported-beam-1d", material, youngsModulusGpa, model: { nodes, elements, materials } };
}

function parseThermalBeam1dNode(raw: unknown, index: number): ThermalBeam1dJobInput["nodes"][number] {
  return parseBeam1dNode(raw, index);
}
function parseThermalBeam1dElement(raw: unknown, index: number): ThermalBeam1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), moment_of_inertia: requiredNumber(element.moment_of_inertia, `elements[${index}].moment_of_inertia`), section_modulus: requiredNumber(element.section_modulus, `elements[${index}].section_modulus`), thermal_expansion: numberOrZero(element.thermal_expansion), section_depth: requiredNumber(element.section_depth, `elements[${index}].section_depth`), distributed_load_y: numberOrZero(element.distributed_load_y), temperature_gradient_y: numberOrZero(element.temperature_gradient_y), material_id: optionalString(element.material_id) };
}
export function parseThermalBeam1dV1(raw: Record<string, unknown>): ImportedThermalBeam1dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalBeam1dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseThermalBeam1dElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "thermal_beam_1d", name: typeof raw.name === "string" ? raw.name : "imported-thermal-beam-1d", material, youngsModulusGpa, model: { nodes, elements, materials } };
}

function parseSpring1dNode(raw: unknown, index: number): Spring1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), fix_x: Boolean(node.fix_x), load_x: numberOrZero(node.load_x) };
}
function parseSpring1dElement(raw: unknown, index: number): Spring1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), stiffness: requiredNumber(element.stiffness, `elements[${index}].stiffness`) };
}
export function parseSpring1dV1(raw: Record<string, unknown>): ImportedSpring1dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseSpring1dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseSpring1dElement) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "spring_1d", name: typeof raw.name === "string" ? raw.name : "imported-spring-1d", model: { nodes, elements } };
}

function parseSpring2dNode(raw: unknown, index: number): Spring2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), fix_x: Boolean(node.fix_x), fix_y: Boolean(node.fix_y), load_x: numberOrZero(node.load_x), load_y: numberOrZero(node.load_y) };
}
function parseSpring2dElement(raw: unknown, index: number): Spring2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), stiffness: requiredNumber(element.stiffness, `elements[${index}].stiffness`) };
}
export function parseSpring2dV1(raw: Record<string, unknown>): ImportedSpring2dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseSpring2dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseSpring2dElement) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "spring_2d", name: typeof raw.name === "string" ? raw.name : "imported-spring-2d", model: { nodes, elements } };
}

function parseSpring3dNode(raw: unknown, index: number): Spring3dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), z: numberOrZero(node.z), fix_x: Boolean(node.fix_x), fix_y: Boolean(node.fix_y), fix_z: Boolean(node.fix_z), load_x: numberOrZero(node.load_x), load_y: numberOrZero(node.load_y), load_z: numberOrZero(node.load_z) };
}
function parseSpring3dElement(raw: unknown, index: number): Spring3dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), stiffness: requiredNumber(element.stiffness, `elements[${index}].stiffness`) };
}
export function parseSpring3dV1(raw: Record<string, unknown>): ImportedSpring3dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseSpring3dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseSpring3dElement) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "spring_3d", name: typeof raw.name === "string" ? raw.name : "imported-spring-3d", model: { nodes, elements } };
}

function parseTorsion1dNode(raw: unknown, index: number): Torsion1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), fix_rz: Boolean(node.fix_rz), torque_z: numberOrZero(node.torque_z) };
}
function parseTorsion1dElement(raw: unknown, index: number): Torsion1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), shear_modulus: requiredNumber(element.shear_modulus, `elements[${index}].shear_modulus`), polar_moment: requiredNumber(element.polar_moment, `elements[${index}].polar_moment`), section_modulus: requiredNumber(element.section_modulus, `elements[${index}].section_modulus`) };
}
export function parseTorsion1dV1(raw: Record<string, unknown>): ImportedTorsion1dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseTorsion1dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseTorsion1dElement) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "torsion_1d", name: typeof raw.name === "string" ? raw.name : "imported-torsion-1d", model: { nodes, elements } };
}
