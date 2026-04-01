import type { ModelMaterial, PlaneTriangle2dJobInput, Truss2dJobInput, Truss3dJobInput } from "@/lib/api";
import { createMaterialDefinition } from "@/lib/materials";

export type ImportedAxialBarModel = {
  kind: "axial_bar_1d";
  name: string;
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  material: string;
  youngsModulusGpa: number;
};

export type ImportedTruss2dModel = {
  kind: "truss_2d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: Truss2dJobInput;
};

export type ImportedPlaneTriangle2dModel = {
  kind: "plane_triangle_2d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: PlaneTriangle2dJobInput;
};

export type ImportedTruss3dModel = {
  kind: "truss_3d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: Truss3dJobInput;
};

export type ImportedModel =
  | ImportedAxialBarModel
  | ImportedTruss2dModel
  | ImportedPlaneTriangle2dModel
  | ImportedTruss3dModel;

const MODEL_SCHEMA_VERSION = "kyuubiki.model/v1";

const MATERIAL_MAP = new Map([
  ["steel", "210"],
  ["210", "210"],
  ["aluminum", "70"],
  ["70", "70"],
  ["titanium", "116"],
  ["116", "116"],
  ["concrete", "30"],
  ["30", "30"],
  ["carbon fiber", "135"],
  ["carbon_fiber", "135"],
  ["135", "135"],
]);

function requiredNumber(value: unknown, name: string): number {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) {
    throw new Error(`${name} must be a positive number`);
  }
  return number;
}

function numberOrZero(value: unknown): number {
  const number = Number(value ?? 0);
  return Number.isFinite(number) ? number : 0;
}

function requiredNonNegativeInteger(value: unknown, name: string): number {
  const number = Number(value);
  if (!Number.isInteger(number) || number < 0) {
    throw new Error(`${name} must be a non-negative integer`);
  }

  return number;
}

function requiredString(value: unknown, name: string): string {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${name} must be a non-empty string`);
  }

  return value;
}

function normalizeMaterial(value: unknown): string {
  const materialKey = typeof value === "string" ? value.trim().toLowerCase() : "";
  return MATERIAL_MAP.get(materialKey) ?? "custom";
}

function optionalString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim().length > 0 ? value : undefined;
}

function parseMaterials(
  raw: Record<string, unknown>,
  fallbackMaterial: string,
  fallbackYoungsModulusGpa: number,
  fallbackPoissonRatio?: number,
): ModelMaterial[] {
  if (!Array.isArray(raw.materials) || raw.materials.length === 0) {
    return [
      createMaterialDefinition(fallbackMaterial, 1, {
        id: "mat-1",
        youngs_modulus: fallbackYoungsModulusGpa * 1.0e9,
        poisson_ratio: fallbackPoissonRatio ?? null,
      }),
    ];
  }

  return raw.materials.map((entry, index) => {
    const material = (entry ?? {}) as Record<string, unknown>;
    return createMaterialDefinition(
      fallbackMaterial,
      index + 1,
      {
        id: requiredString(material.id, `materials[${index}].id`),
        name: optionalString(material.name) ?? createMaterialDefinition(fallbackMaterial, index + 1).name,
        youngs_modulus: requiredNumber(
          material.youngs_modulus,
          `materials[${index}].youngs_modulus`,
        ),
        poisson_ratio:
          material.poisson_ratio === undefined || material.poisson_ratio === null
            ? fallbackPoissonRatio ?? null
            : Number(material.poisson_ratio),
      },
    );
  });
}

function assertSupportedVersion(raw: Record<string, unknown>) {
  const version = raw.model_schema_version;
  if (version === undefined) return;
  if (version !== MODEL_SCHEMA_VERSION) {
    throw new Error(`unsupported model_schema_version: ${String(version)}`);
  }
}

function parseAxialBarV1(raw: Record<string, unknown>): ImportedAxialBarModel {
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

function parseTrussNode(raw: unknown, index: number): Truss2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    y: numberOrZero(node.y),
    fix_x: Boolean(node.fix_x),
    fix_y: Boolean(node.fix_y),
    load_x: numberOrZero(node.load_x),
    load_y: numberOrZero(node.load_y),
  };
}

function parseTrussElement(raw: unknown, index: number): Truss2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    area: requiredNumber(element.area, `elements[${index}].area`),
    youngs_modulus: requiredNumber(
      element.youngs_modulus,
      `elements[${index}].youngs_modulus`,
    ),
    material_id: optionalString(element.material_id),
  };
}

function parseTruss2dV1(raw: Record<string, unknown>): ImportedTruss2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseTrussNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parseTrussElement).map((element) => ({
        ...element,
        material_id: element.material_id ?? defaultMaterialId,
      }))
    : [];

  if (nodes.length < 2) {
    throw new Error("nodes must contain at least two entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "truss_2d",
    name: typeof raw.name === "string" ? raw.name : "imported-truss",
    material,
    youngsModulusGpa,
    model: { nodes, elements, materials },
  };
}

function parsePlaneNode(raw: unknown, index: number): PlaneTriangle2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    y: numberOrZero(node.y),
    fix_x: Boolean(node.fix_x),
    fix_y: Boolean(node.fix_y),
    load_x: numberOrZero(node.load_x),
    load_y: numberOrZero(node.load_y),
  };
}

function parsePlaneElement(
  raw: unknown,
  index: number,
): PlaneTriangle2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    node_k: requiredNonNegativeInteger(element.node_k, `elements[${index}].node_k`),
    thickness: requiredNumber(element.thickness, `elements[${index}].thickness`),
    youngs_modulus: requiredNumber(
      element.youngs_modulus,
      `elements[${index}].youngs_modulus`,
    ),
    poisson_ratio: numberOrZero(element.poisson_ratio),
    material_id: optionalString(element.material_id),
  };
}

function parsePlaneTriangle2dV1(raw: Record<string, unknown>): ImportedPlaneTriangle2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(
    raw,
    material,
    youngsModulusGpa,
    Array.isArray(raw.elements) && raw.elements.length > 0
      ? numberOrZero(((raw.elements[0] ?? {}) as Record<string, unknown>).poisson_ratio)
      : 0.33,
  );
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parsePlaneNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parsePlaneElement).map((element) => ({
        ...element,
        material_id: element.material_id ?? defaultMaterialId,
      }))
    : [];

  if (nodes.length < 3) {
    throw new Error("nodes must contain at least three entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "plane_triangle_2d",
    name: typeof raw.name === "string" ? raw.name : "imported-plane",
    material,
    youngsModulusGpa,
    model: { nodes, elements, materials },
  };
}

function parseTruss3dNode(raw: unknown, index: number): Truss3dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    y: numberOrZero(node.y),
    z: numberOrZero(node.z),
    fix_x: Boolean(node.fix_x),
    fix_y: Boolean(node.fix_y),
    fix_z: Boolean(node.fix_z),
    load_x: numberOrZero(node.load_x),
    load_y: numberOrZero(node.load_y),
    load_z: numberOrZero(node.load_z),
  };
}

function parseTruss3dElement(raw: unknown, index: number): Truss3dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    area: requiredNumber(element.area, `elements[${index}].area`),
    youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`),
    material_id: optionalString(element.material_id),
  };
}

function parseTruss3dV1(raw: Record<string, unknown>): ImportedTruss3dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseTruss3dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parseTruss3dElement).map((element) => ({
        ...element,
        material_id: element.material_id ?? defaultMaterialId,
      }))
    : [];

  if (nodes.length < 3) {
    throw new Error("nodes must contain at least three entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "truss_3d",
    name: typeof raw.name === "string" ? raw.name : "imported-truss-3d",
    material,
    youngsModulusGpa,
    model: { nodes, elements, materials },
  };
}

export function parsePlaygroundModel(text: string): ImportedModel {
  const raw = JSON.parse(text) as Record<string, unknown>;
  assertSupportedVersion(raw);

  const kind =
    typeof raw.kind === "string"
      ? raw.kind
      : Array.isArray(raw.nodes) || Array.isArray(raw.elements)
        ? "truss_2d"
        : "axial_bar_1d";

  if (kind === "plane_triangle_2d") {
    return parsePlaneTriangle2dV1(raw);
  }

  if (kind === "truss_3d") {
    return parseTruss3dV1(raw);
  }

  if (kind === "truss_2d") {
    return parseTruss2dV1(raw);
  }

  return parseAxialBarV1(raw);
}
