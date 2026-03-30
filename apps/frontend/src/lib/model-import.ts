import type { PlaneTriangle2dJobInput, Truss2dJobInput } from "@/lib/api";

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

export type ImportedModel =
  | ImportedAxialBarModel
  | ImportedTruss2dModel
  | ImportedPlaneTriangle2dModel;

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
  };
}

function parseTruss2dV1(raw: Record<string, unknown>): ImportedTruss2dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseTrussNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseTrussElement) : [];

  if (nodes.length < 2) {
    throw new Error("nodes must contain at least two entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "truss_2d",
    name: typeof raw.name === "string" ? raw.name : "imported-truss",
    material: normalizeMaterial(raw.material),
    youngsModulusGpa: requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa"),
    model: { nodes, elements },
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
  };
}

function parsePlaneTriangle2dV1(raw: Record<string, unknown>): ImportedPlaneTriangle2dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parsePlaneNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parsePlaneElement) : [];

  if (nodes.length < 3) {
    throw new Error("nodes must contain at least three entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "plane_triangle_2d",
    name: typeof raw.name === "string" ? raw.name : "imported-plane",
    material: normalizeMaterial(raw.material),
    youngsModulusGpa: requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa"),
    model: { nodes, elements },
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

  if (kind === "truss_2d") {
    return parseTruss2dV1(raw);
  }

  return parseAxialBarV1(raw);
}
