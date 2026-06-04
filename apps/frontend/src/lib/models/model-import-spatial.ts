import type { ThermalTruss3dJobInput, Truss3dJobInput } from "@/lib/api";
import type { ImportedThermalTruss3dModel, ImportedTruss3dModel } from "@/lib/models/model-import-types";
import {
  normalizeMaterial,
  numberOrZero,
  optionalString,
  parseMaterials,
  requiredNonNegativeInteger,
  requiredNumber,
  requiredString,
} from "@/lib/models/model-import-utils";

function parseTruss3dNode(raw: unknown, index: number): Truss3dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), z: numberOrZero(node.z), fix_x: Boolean(node.fix_x), fix_y: Boolean(node.fix_y), fix_z: Boolean(node.fix_z), load_x: numberOrZero(node.load_x), load_y: numberOrZero(node.load_y), load_z: numberOrZero(node.load_z) };
}
function parseThermalTruss3dNode(raw: unknown, index: number): ThermalTruss3dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(node.id, `nodes[${index}].id`), x: numberOrZero(node.x), y: numberOrZero(node.y), z: numberOrZero(node.z), fix_x: Boolean(node.fix_x), fix_y: Boolean(node.fix_y), fix_z: Boolean(node.fix_z), load_x: numberOrZero(node.load_x), load_y: numberOrZero(node.load_y), load_z: numberOrZero(node.load_z), temperature_delta: numberOrZero(node.temperature_delta) };
}
function parseTruss3dElement(raw: unknown, index: number): Truss3dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), area: requiredNumber(element.area, `elements[${index}].area`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), material_id: optionalString(element.material_id) };
}
function parseThermalTruss3dElement(raw: unknown, index: number): ThermalTruss3dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return { id: requiredString(element.id, `elements[${index}].id`), node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`), node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`), area: requiredNumber(element.area, `elements[${index}].area`), youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`), thermal_expansion: numberOrZero(element.thermal_expansion), material_id: optionalString(element.material_id) };
}
export function parseTruss3dV1(raw: Record<string, unknown>): ImportedTruss3dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseTruss3dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseTruss3dElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 3) throw new Error("nodes must contain at least three entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "truss_3d", name: typeof raw.name === "string" ? raw.name : "imported-truss-3d", material, youngsModulusGpa, model: { nodes, elements, materials } };
}
export function parseThermalTruss3dV1(raw: Record<string, unknown>): ImportedThermalTruss3dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalTruss3dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseThermalTruss3dElement).map((element) => ({ ...element, material_id: element.material_id ?? defaultMaterialId })) : [];
  if (nodes.length < 3) throw new Error("nodes must contain at least three entries");
  if (elements.length < 1) throw new Error("elements must contain at least one entry");
  return { kind: "thermal_truss_3d", name: typeof raw.name === "string" ? raw.name : "imported-thermal-truss-3d", material, youngsModulusGpa, model: { nodes, elements, materials } };
}
