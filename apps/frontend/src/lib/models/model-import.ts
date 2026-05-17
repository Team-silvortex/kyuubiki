import type {
  Beam1dJobInput,
  Frame2dJobInput,
  ModelMaterial,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalBar1dJobInput,
  ThermalTruss2dJobInput,
  ThermalTruss3dJobInput,
  Spring1dJobInput,
  Spring2dJobInput,
  Spring3dJobInput,
  Torsion1dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
} from "@/lib/api";
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

export type ImportedThermalBar1dModel = {
  kind: "thermal_bar_1d";
  name: string;
  model: ThermalBar1dJobInput;
};

export type ImportedThermalTruss2dModel = {
  kind: "thermal_truss_2d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: ThermalTruss2dJobInput;
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

export type ImportedPlaneQuad2dModel = {
  kind: "plane_quad_2d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: PlaneQuad2dJobInput;
};

export type ImportedTruss3dModel = {
  kind: "truss_3d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: Truss3dJobInput;
};

export type ImportedThermalTruss3dModel = {
  kind: "thermal_truss_3d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: ThermalTruss3dJobInput;
};

export type ImportedFrame2dModel = {
  kind: "frame_2d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: Frame2dJobInput;
};

export type ImportedBeam1dModel = {
  kind: "beam_1d";
  name: string;
  material: string;
  youngsModulusGpa: number;
  model: Beam1dJobInput;
};

export type ImportedTorsion1dModel = {
  kind: "torsion_1d";
  name: string;
  model: Torsion1dJobInput;
};

export type ImportedSpring1dModel = {
  kind: "spring_1d";
  name: string;
  model: Spring1dJobInput;
};

export type ImportedSpring2dModel = {
  kind: "spring_2d";
  name: string;
  model: Spring2dJobInput;
};

export type ImportedSpring3dModel = {
  kind: "spring_3d";
  name: string;
  model: Spring3dJobInput;
};

export type ImportedModel =
  | ImportedAxialBarModel
  | ImportedThermalBar1dModel
  | ImportedThermalTruss2dModel
  | ImportedSpring1dModel
  | ImportedSpring2dModel
  | ImportedSpring3dModel
  | ImportedTruss2dModel
  | ImportedPlaneTriangle2dModel
  | ImportedPlaneQuad2dModel
  | ImportedTruss3dModel
  | ImportedThermalTruss3dModel
  | ImportedFrame2dModel
  | ImportedBeam1dModel
  | ImportedTorsion1dModel;

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

function parseThermalBar1dNode(raw: unknown, index: number): ThermalBar1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    fix_x: Boolean(node.fix_x),
    load_x: numberOrZero(node.load_x),
    temperature_delta: numberOrZero(node.temperature_delta),
  };
}

function parseThermalBar1dElement(raw: unknown, index: number): ThermalBar1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    area: requiredNumber(element.area, `elements[${index}].area`),
    youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`),
    thermal_expansion: numberOrZero(element.thermal_expansion),
  };
}

function parseThermalBar1dV1(raw: Record<string, unknown>): ImportedThermalBar1dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalBar1dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseThermalBar1dElement) : [];

  if (nodes.length < 2) {
    throw new Error("nodes must contain at least two entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "thermal_bar_1d",
    name: typeof raw.name === "string" ? raw.name : "imported-thermal-bar-1d",
    model: { nodes, elements },
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

function parseThermalTruss2dNode(raw: unknown, index: number): ThermalTruss2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    y: numberOrZero(node.y),
    fix_x: Boolean(node.fix_x),
    fix_y: Boolean(node.fix_y),
    load_x: numberOrZero(node.load_x),
    load_y: numberOrZero(node.load_y),
    temperature_delta: numberOrZero(node.temperature_delta),
  };
}

function parseThermalTruss2dElement(raw: unknown, index: number): ThermalTruss2dJobInput["elements"][number] {
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
    thermal_expansion: numberOrZero(element.thermal_expansion),
    material_id: optionalString(element.material_id),
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

function parseThermalTruss2dV1(raw: Record<string, unknown>): ImportedThermalTruss2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalTruss2dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parseThermalTruss2dElement).map((element) => ({
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
    kind: "thermal_truss_2d",
    name: typeof raw.name === "string" ? raw.name : "imported-thermal-truss",
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

function parsePlaneQuadElement(
  raw: unknown,
  index: number,
): PlaneQuad2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    node_k: requiredNonNegativeInteger(element.node_k, `elements[${index}].node_k`),
    node_l: requiredNonNegativeInteger(element.node_l, `elements[${index}].node_l`),
    thickness: requiredNumber(element.thickness, `elements[${index}].thickness`),
    youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`),
    poisson_ratio: requiredNumber(element.poisson_ratio, `elements[${index}].poisson_ratio`),
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

function parsePlaneQuad2dV1(raw: Record<string, unknown>): ImportedPlaneQuad2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const poissonRatio = requiredNumber(raw.poisson_ratio ?? 0.33, "poisson_ratio");
  const materials = parseMaterials(raw, material, youngsModulusGpa, poissonRatio);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parsePlaneNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parsePlaneQuadElement).map((element) => ({
        ...element,
        material_id: element.material_id ?? defaultMaterialId,
      }))
    : [];

  if (nodes.length < 4) {
    throw new Error("nodes must contain at least four entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "plane_quad_2d",
    name: typeof raw.name === "string" ? raw.name : "imported-plane-quad",
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

function parseThermalTruss3dNode(raw: unknown, index: number): ThermalTruss3dJobInput["nodes"][number] {
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
    temperature_delta: numberOrZero(node.temperature_delta),
  };
}

function parseThermalTruss3dElement(raw: unknown, index: number): ThermalTruss3dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    area: requiredNumber(element.area, `elements[${index}].area`),
    youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`),
    thermal_expansion: numberOrZero(element.thermal_expansion),
    material_id: optionalString(element.material_id),
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

function parseThermalTruss3dV1(raw: Record<string, unknown>): ImportedThermalTruss3dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseThermalTruss3dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parseThermalTruss3dElement).map((element) => ({
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
    kind: "thermal_truss_3d",
    name: typeof raw.name === "string" ? raw.name : "imported-thermal-truss-3d",
    material,
    youngsModulusGpa,
    model: { nodes, elements, materials },
  };
}

function parseFrame2dNode(raw: unknown, index: number): Frame2dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    y: numberOrZero(node.y),
    fix_x: Boolean(node.fix_x),
    fix_y: Boolean(node.fix_y),
    fix_rz: Boolean(node.fix_rz),
    load_x: numberOrZero(node.load_x),
    load_y: numberOrZero(node.load_y),
    moment_z: numberOrZero(node.moment_z),
  };
}

function parseFrame2dElement(raw: unknown, index: number): Frame2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    area: requiredNumber(element.area, `elements[${index}].area`),
    youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`),
    moment_of_inertia: requiredNumber(element.moment_of_inertia, `elements[${index}].moment_of_inertia`),
    section_modulus: requiredNumber(element.section_modulus, `elements[${index}].section_modulus`),
    material_id: optionalString(element.material_id),
  };
}

function parseFrame2dV1(raw: Record<string, unknown>): ImportedFrame2dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseFrame2dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parseFrame2dElement).map((element) => ({
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
    kind: "frame_2d",
    name: typeof raw.name === "string" ? raw.name : "imported-frame-2d",
    material,
    youngsModulusGpa,
    model: { nodes, elements, materials },
  };
}

function parseBeam1dNode(raw: unknown, index: number): Beam1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    fix_y: Boolean(node.fix_y),
    fix_rz: Boolean(node.fix_rz),
    load_y: numberOrZero(node.load_y),
    moment_z: numberOrZero(node.moment_z),
  };
}

function parseBeam1dElement(raw: unknown, index: number): Beam1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    youngs_modulus: requiredNumber(element.youngs_modulus, `elements[${index}].youngs_modulus`),
    moment_of_inertia: requiredNumber(element.moment_of_inertia, `elements[${index}].moment_of_inertia`),
    section_modulus: requiredNumber(element.section_modulus, `elements[${index}].section_modulus`),
    distributed_load_y: numberOrZero(element.distributed_load_y),
    material_id: optionalString(element.material_id),
  };
}

function parseBeam1dV1(raw: Record<string, unknown>): ImportedBeam1dModel {
  const material = normalizeMaterial(raw.material);
  const youngsModulusGpa = requiredNumber(raw.youngs_modulus_gpa, "youngs_modulus_gpa");
  const materials = parseMaterials(raw, material, youngsModulusGpa);
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseBeam1dNode) : [];
  const defaultMaterialId = materials[0]?.id;
  const elements = Array.isArray(raw.elements)
    ? raw.elements.map(parseBeam1dElement).map((element) => ({
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
    kind: "beam_1d",
    name: typeof raw.name === "string" ? raw.name : "imported-beam-1d",
    material,
    youngsModulusGpa,
    model: { nodes, elements, materials },
  };
}

function parseSpring1dNode(raw: unknown, index: number): Spring1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    fix_x: Boolean(node.fix_x),
    load_x: numberOrZero(node.load_x),
  };
}

function parseSpring1dElement(raw: unknown, index: number): Spring1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    stiffness: requiredNumber(element.stiffness, `elements[${index}].stiffness`),
  };
}

function parseSpring1dV1(raw: Record<string, unknown>): ImportedSpring1dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseSpring1dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseSpring1dElement) : [];

  if (nodes.length < 2) {
    throw new Error("nodes must contain at least two entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "spring_1d",
    name: typeof raw.name === "string" ? raw.name : "imported-spring-1d",
    model: { nodes, elements },
  };
}

function parseSpring2dNode(raw: unknown, index: number): Spring2dJobInput["nodes"][number] {
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

function parseSpring2dElement(raw: unknown, index: number): Spring2dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    stiffness: requiredNumber(element.stiffness, `elements[${index}].stiffness`),
  };
}

function parseSpring2dV1(raw: Record<string, unknown>): ImportedSpring2dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseSpring2dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseSpring2dElement) : [];

  if (nodes.length < 2) {
    throw new Error("nodes must contain at least two entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "spring_2d",
    name: typeof raw.name === "string" ? raw.name : "imported-spring-2d",
    model: { nodes, elements },
  };
}

function parseSpring3dNode(raw: unknown, index: number): Spring3dJobInput["nodes"][number] {
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

function parseSpring3dElement(raw: unknown, index: number): Spring3dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    stiffness: requiredNumber(element.stiffness, `elements[${index}].stiffness`),
  };
}

function parseSpring3dV1(raw: Record<string, unknown>): ImportedSpring3dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseSpring3dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseSpring3dElement) : [];

  if (nodes.length < 2) {
    throw new Error("nodes must contain at least two entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "spring_3d",
    name: typeof raw.name === "string" ? raw.name : "imported-spring-3d",
    model: { nodes, elements },
  };
}

function parseTorsion1dNode(raw: unknown, index: number): Torsion1dJobInput["nodes"][number] {
  const node = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(node.id, `nodes[${index}].id`),
    x: numberOrZero(node.x),
    fix_rz: Boolean(node.fix_rz),
    torque_z: numberOrZero(node.torque_z),
  };
}

function parseTorsion1dElement(raw: unknown, index: number): Torsion1dJobInput["elements"][number] {
  const element = (raw ?? {}) as Record<string, unknown>;
  return {
    id: requiredString(element.id, `elements[${index}].id`),
    node_i: requiredNonNegativeInteger(element.node_i, `elements[${index}].node_i`),
    node_j: requiredNonNegativeInteger(element.node_j, `elements[${index}].node_j`),
    shear_modulus: requiredNumber(element.shear_modulus, `elements[${index}].shear_modulus`),
    polar_moment: requiredNumber(element.polar_moment, `elements[${index}].polar_moment`),
    section_modulus: requiredNumber(element.section_modulus, `elements[${index}].section_modulus`),
  };
}

function parseTorsion1dV1(raw: Record<string, unknown>): ImportedTorsion1dModel {
  const nodes = Array.isArray(raw.nodes) ? raw.nodes.map(parseTorsion1dNode) : [];
  const elements = Array.isArray(raw.elements) ? raw.elements.map(parseTorsion1dElement) : [];

  if (nodes.length < 2) {
    throw new Error("nodes must contain at least two entries");
  }

  if (elements.length < 1) {
    throw new Error("elements must contain at least one entry");
  }

  return {
    kind: "torsion_1d",
    name: typeof raw.name === "string" ? raw.name : "imported-torsion-1d",
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

  if (kind === "plane_quad_2d") {
    return parsePlaneQuad2dV1(raw);
  }

  if (kind === "truss_3d") {
    return parseTruss3dV1(raw);
  }

  if (kind === "thermal_truss_3d") {
    return parseThermalTruss3dV1(raw);
  }

  if (kind === "frame_2d") {
    return parseFrame2dV1(raw);
  }

  if (kind === "beam_1d") {
    return parseBeam1dV1(raw);
  }

  if (kind === "torsion_1d") {
    return parseTorsion1dV1(raw);
  }

  if (kind === "spring_1d") {
    return parseSpring1dV1(raw);
  }

  if (kind === "thermal_bar_1d") {
    return parseThermalBar1dV1(raw);
  }

  if (kind === "thermal_truss_2d") {
    return parseThermalTruss2dV1(raw);
  }

  if (kind === "spring_2d") {
    return parseSpring2dV1(raw);
  }

  if (kind === "spring_3d") {
    return parseSpring3dV1(raw);
  }

  if (kind === "truss_2d") {
    return parseTruss2dV1(raw);
  }

  return parseAxialBarV1(raw);
}
