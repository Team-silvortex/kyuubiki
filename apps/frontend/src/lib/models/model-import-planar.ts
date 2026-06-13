import type {
  ElectrostaticPlaneQuad2dJobInput,
  ElectrostaticPlaneTriangle2dJobInput,
  Frame2dJobInput,
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalFrame2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ThermalTruss2dJobInput,
  Truss2dJobInput,
} from "@/lib/api";
import type {
  ImportedElectrostaticPlaneQuad2dModel,
  ImportedElectrostaticPlaneTriangle2dModel,
  ImportedFrame2dModel,
  ImportedHeatPlaneQuad2dModel,
  ImportedHeatPlaneTriangle2dModel,
  ImportedPlaneQuad2dModel,
  ImportedPlaneTriangle2dModel,
  ImportedThermalFrame2dModel,
  ImportedThermalPlaneQuad2dModel,
  ImportedThermalPlaneTriangle2dModel,
  ImportedThermalTruss2dModel,
  ImportedTruss2dModel,
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

function parseTrussNode(raw: unknown, index: number): Truss2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), fix_x: Boolean(node.fix_x), fix_y: Boolean(node.fix_y), load_x: numberOrZero(node.load_x), load_y: numberOrZero(node.load_y) };
}
function parseTrussElement(raw: unknown, index: number): Truss2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), area: requiredNumber(element.area, `elements[${index}].area`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), material_id: optionalString(element.material_id) };
}
export function parseTruss2dV1(raw: Record<string, unknown>): ImportedTruss2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseTrussNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseTrussElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "truss_2d", name: typeof raw.name === "string" ? raw.name : "imported-truss", material, youngsModulusGpa, model: { nodes, elements, materials } };
}

function parseThermalTruss2dNode(raw: unknown, index: number): ThermalTruss2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), fix_x: Boolean(node.fix_x), fix_y: Boolean(node.fix_y), load_x: numberOrZero(node.load_x), load_y: numberOrZero(node.load_y), temperature_delta: numberOrZero(node.temperature_delta) };
}
function parseThermalTruss2dElement(raw: unknown, index: number): ThermalTruss2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), area: requiredNumber(element.area, `elements[${index}].area`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), thermal_expansion: numberOrZero(element.thermal_expansion), material_id: optionalString(element.material_id) };
}
export function parseThermalTruss2dV1(raw: Record<string, unknown>): ImportedThermalTruss2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalTruss2dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseThermalTruss2dElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "thermal_truss_2d", name: typeof raw.name === "string" ? raw.name : "imported-thermal-truss", material, youngsModulusGpa, model: { nodes, elements, materials } };
}

function parsePlaneNode(raw: unknown, index: number): PlaneTriangle2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), fix_x: Boolean(node.fix_x), fix_y: Boolean(node.fix_y), load_x: numberOrZero(node.load_x), load_y: numberOrZero(node.load_y) };
}
function parseThermalPlaneNode(raw: unknown, index: number): ThermalPlaneTriangle2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { ...parsePlaneNode(raw, index), temperature_delta: numberOrZero(node.temperature_delta) };
}
function parseHeatPlaneNode(raw: unknown, index: number): HeatPlaneTriangle2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), fix_temperature: Boolean(node.fix_temperature), temperature: numberOrZero(node.temperature), heat_load: numberOrZero(node.heat_load) };
}
function parseElectrostaticPlaneNode(raw: unknown, index: number): ElectrostaticPlaneTriangle2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), fix_potential: Boolean(node.fix_potential), potential: numberOrZero(node.potential), charge_density: numberOrZero(node.charge_density) };
}
function parsePlaneElement(raw: unknown, index: number): PlaneTriangle2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), node_k: requiredNonNegativeInteger(element.node_k, `elements[${index}].node_k`), thickness: requiredNumber(element.thickness, `elements[${index}].thickness`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), poisson_ratio: numberOrZero(element.poisson_ratio), material_id: optionalString(element.material_id) };
}
function parsePlaneQuadElement(raw: unknown, index: number): PlaneQuad2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), node_k: requiredNonNegativeInteger(element.node_k, `elements[${index}].node_k`), node_l: requiredNonNegativeInteger(element.node_l, `elements[${index}].node_l`), thickness: requiredNumber(element.thickness, `elements[${index}].thickness`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), poisson_ratio: requiredNumber(element.poisson_ratio, `elements[${index}].poisson_ratio`), material_id: optionalString(element.material_id) };
}
function parseThermalPlaneElement(raw: unknown, index: number): ThermalPlaneTriangle2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { ...parsePlaneElement(raw, index), thermal_expansion: requiredNumber(element.thermal_expansion, `elements[${index}].thermal_expansion`) };
}
function parseThermalPlaneQuadElement(raw: unknown, index: number): ThermalPlaneQuad2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { ...parsePlaneQuadElement(raw, index), thermal_expansion: requiredNumber(element.thermal_expansion, `elements[${index}].thermal_expansion`) };
}
function parseHeatPlaneTriangleElement(raw: unknown, index: number): HeatPlaneTriangle2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), node_k: requiredNonNegativeInteger(element.node_k, `elements[${index}].node_k`), thickness: requiredNumber(element.thickness, `elements[${index}].thickness`), conductivity: requiredNumber(element.conductivity, `elements[${index}].conductivity`), material_id: optionalString(element.material_id) };
}
function parseHeatPlaneQuadElement(raw: unknown, index: number): HeatPlaneQuad2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), node_k: requiredNonNegativeInteger(element.node_k, `elements[${index}].node_k`), node_l: requiredNonNegativeInteger(element.node_l, `elements[${index}].node_l`), thickness: requiredNumber(element.thickness, `elements[${index}].thickness`), conductivity: requiredNumber(element.conductivity, `elements[${index}].conductivity`), material_id: optionalString(element.material_id) };
}
function parseElectrostaticPlaneTriangleElement(raw: unknown, index: number): ElectrostaticPlaneTriangle2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), node_k: requiredNonNegativeInteger(element.node_k, `elements[${index}].node_k`), thickness: requiredNumber(element.thickness, `elements[${index}].thickness`), permittivity: requiredNumber(element.permittivity, `elements[${index}].permittivity`), material_id: optionalString(element.material_id) };
}
function parseElectrostaticPlaneQuadElement(raw: unknown, index: number): ElectrostaticPlaneQuad2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), node_k: requiredNonNegativeInteger(element.node_k, `elements[${index}].node_k`), node_l: requiredNonNegativeInteger(element.node_l, `elements[${index}].node_l`), thickness: requiredNumber(element.thickness, `elements[${index}].thickness`), permittivity: requiredNumber(element.permittivity, `elements[${index}].permittivity`), material_id: optionalString(element.material_id) };
}
export function parsePlaneTriangle2dV1(raw: Record<string, unknown>): ImportedPlaneTriangle2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa, Array.isArray(raw.elements) && raw.elements.length > 0 ? numberOrZero(((raw.elements[0] ?? {}) as Record<string, unknown>).poisson_ratio) : 0.33);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parsePlaneNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parsePlaneElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 3) throw new Error("nodes must contain at least three entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "plane_triangle_2d", name: typeof raw.name === "string" ? raw.name : "imported-plane", material, youngsModulusGpa, model: { nodes, elements, materials } };
}
export function parsePlaneQuad2dV1(raw: Record<string, unknown>): ImportedPlaneQuad2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const poissonRatio = requiredNumber(raw.poisson_ratio ?? 0.33, "poisson_ratio");
  const materials = parseMaterials(raw, material, youngsModulusGpa, poissonRatio);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parsePlaneNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parsePlaneQuadElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 4) throw new Error("nodes must contain at least four entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "plane_quad_2d", name: typeof raw.name === "string" ? raw.name : "imported-plane-quad", material, youngsModulusGpa, model: { nodes, elements, materials } };
}
export function parseThermalPlaneTriangle2dV1(raw: Record<string, unknown>): ImportedThermalPlaneTriangle2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa, Array.isArray(raw.elements) && raw.elements.length > 0 ? numberOrZero(((raw.elements[0] ?? {}) as Record<string, unknown>).poisson_ratio) : 0.33);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalPlaneNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseThermalPlaneElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 3) throw new Error("nodes must contain at least three entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "thermal_plane_triangle_2d", name: typeof raw.name === "string" ? raw.name : "imported-thermal-plane", material, youngsModulusGpa, model: { nodes, elements, materials } };
}
export function parseThermalPlaneQuad2dV1(raw: Record<string, unknown>): ImportedThermalPlaneQuad2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const poissonRatio = requiredNumber(raw.poisson_ratio ?? 0.33, "poisson_ratio");
  const materials = parseMaterials(raw, material, youngsModulusGpa, poissonRatio);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalPlaneNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseThermalPlaneQuadElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 4) throw new Error("nodes must contain at least four entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "thermal_plane_quad_2d", name: typeof raw.name === "string" ? raw.name : "imported-thermal-plane-quad", material, youngsModulusGpa, model: { nodes, elements, materials } };
}
export function parseHeatPlaneTriangle2dV1(raw: Record<string, unknown>): ImportedHeatPlaneTriangle2dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseHeatPlaneNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseHeatPlaneTriangleElement) : [];
  if (nodes.length < 3) throw new Error("nodes must contain at least three entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "heat_plane_triangle_2d", name: typeof raw.name === "string" ? raw.name : "imported-heat-plane", model: { nodes, elements } };
}
export function parseHeatPlaneQuad2dV1(raw: Record<string, unknown>): ImportedHeatPlaneQuad2dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseHeatPlaneNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseHeatPlaneQuadElement) : [];
  if (nodes.length < 4) throw new Error("nodes must contain at least four entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "heat_plane_quad_2d", name: typeof raw.name === "string" ? raw.name : "imported-heat-plane-quad", model: { nodes, elements } };
}
export function parseElectrostaticPlaneTriangle2dV1(raw: Record<string, unknown>): ImportedElectrostaticPlaneTriangle2dModel {
  const material = normalizeMaterial(raw.material);
  const materials = parseMaterials(raw, material, 1);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseElectrostaticPlaneNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parseElectrostaticPlaneTriangleElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId }))
    : [];
  if (nodes.length < 3) throw new Error("nodes must contain at least three entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "electrostatic_plane_triangle_2d", name: typeof raw.name === "string" ? raw.name : "imported-electrostatic-plane", material, model: { nodes, elements, materials } };
}
export function parseElectrostaticPlaneQuad2dV1(raw: Record<string, unknown>): ImportedElectrostaticPlaneQuad2dModel {
  const material = normalizeMaterial(raw.material);
  const materials = parseMaterials(raw, material, 1);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseElectrostaticPlaneNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parseElectrostaticPlaneQuadElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId }))
    : [];
  if (nodes.length < 4) throw new Error("nodes must contain at least four entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "electrostatic_plane_quad_2d", name: typeof raw.name === "string" ? raw.name : "imported-electrostatic-plane-quad", material, model: { nodes, elements, materials } };
}

function parseFrame2dNode(raw: unknown, index: number): Frame2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), fix_x: Boolean(node.fix_x), fix_y: Boolean(node.fix_y), fix_rz: Boolean(node.fix_rz), load_x: numberOrZero(node.load_x), load_y: numberOrZero(node.load_y), moment_z: numberOrZero(node.moment_z) };
}
function parseThermalFrame2dNode(raw: unknown, index: number): ThermalFrame2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { ...parseFrame2dNode(raw, index), temperature_delta: numberOrZero(node.temperature_delta) };
}
function parseFrame2dElement(raw: unknown, index: number): Frame2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), area: requiredNumber(element.area, `elements[${index}].area`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), moment_of_inertia: requiredNumber(element.moment_of_inertia, `elements[${index}].moment_of_inertia`), section_modulus: requiredNumber(element.section_modulus, `elements[${index}].section_modulus`), material_id: optionalString(element.material_id) };
}
function parseThermalFrame2dElement(raw: unknown, index: number): ThermalFrame2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), area: requiredNumber(element.area, `elements[${index}].area`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), moment_of_inertia: requiredNumber(element.moment_of_inertia, `elements[${index}].moment_of_inertia`), section_modulus: requiredNumber(element.section_modulus, `elements[${index}].section_modulus`), thermal_expansion: numberOrZero(element.thermal_expansion), section_depth: requiredNumber(element.section_depth, `elements[${index}].section_depth`), temperature_gradient_y: numberOrZero(element.temperature_gradient_y), material_id: optionalString(element.material_id) };
}
export function parseFrame2dV1(raw: Record<string, unknown>): ImportedFrame2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseFrame2dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseFrame2dElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "frame_2d", name: typeof raw.name === "string" ? raw.name : "imported-frame-2d", material, youngsModulusGpa, model: { nodes, elements, materials } };
}
export function parseThermalFrame2dV1(raw: Record<string, unknown>): ImportedThermalFrame2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalFrame2dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseThermalFrame2dElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 2) throw new Error("nodes must contain at least two entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "thermal_frame_2d", name: typeof raw.name === "string" ? raw.name : "imported-thermal-frame-2d", material, youngsModulusGpa, model: { nodes, elements, materials } };
}
