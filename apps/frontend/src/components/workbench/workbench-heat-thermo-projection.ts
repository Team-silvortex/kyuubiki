import type {
  HeatBar1dJobInput, HeatBar1dResult, HeatPlaneQuad2dJobInput, HeatPlaneQuad2dResult,
  HeatPlaneTriangle2dJobInput, HeatPlaneTriangle2dResult, ThermalBar1dJobInput,
  ThermalPlaneQuad2dJobInput, ThermalPlaneTriangle2dJobInput,
  PlaneQuad2dJobInput, PlaneTriangle2dJobInput,
  ElectrostaticPlaneQuad2dJobInput, ElectrostaticPlaneTriangle2dJobInput,
} from "@/lib/api";
import { createMaterialDefinition } from "@/lib/materials";
import { defaultThermalBar1d, defaultThermalPlaneQuad, defaultThermalPlaneTriangle } from "./workbench-defaults";

type MeshNode = { id: string; x: number; y?: number };
type MeshElement = { node_i: number; node_j: number; node_k?: number; node_l?: number };
type Mesh = { nodes: MeshNode[]; elements: MeshElement[] };
type HeatPlaneModel = HeatPlaneTriangle2dJobInput | HeatPlaneQuad2dJobInput;
type HeatModel = HeatBar1dJobInput | HeatPlaneModel;
type HeatResult = {
  input: HeatModel;
  nodes: Array<MeshNode & { index: number; temperature: number }>;
};
const INDEX_FIELDS = ["node_i", "node_j", "node_k", "node_l"] as const;

function invalid(reason: string): never {
  throw new Error(`HEAT_THERMO_PROJECTION_INVALID: ${reason}`);
}

function samePosition(left: MeshNode, right: MeshNode) {
  return Boolean(left && right) && Number.isFinite(left.x) && Number.isFinite(right.x) && Math.abs(left.x - right.x) <= 1e-9 &&
    (left.y === undefined && right.y === undefined ||
      typeof left.y === "number" && typeof right.y === "number" &&
      Number.isFinite(left.y) && Number.isFinite(right.y) && Math.abs(left.y - right.y) <= 1e-9);
}

function sameMesh(source: Mesh, target: Mesh) {
  return Boolean(source && target) && Array.isArray(source.nodes) && Array.isArray(target.nodes) &&
    Array.isArray(source.elements) && Array.isArray(target.elements) &&
    source.nodes.length === target.nodes.length && source.elements.length === target.elements.length &&
    source.nodes.every((node, index) => samePosition(node, target.nodes[index])) &&
    source.elements.every((element, index) => element && target.elements[index] &&
      INDEX_FIELDS.every((key) => element[key] === target.elements[index][key]));
}

function validateHeatInput(source: HeatModel, solved: HeatModel, arity: number) {
  if (!source.nodes.length || !source.elements.length || !solved || !Array.isArray(solved.nodes) ||
    !Array.isArray(solved.elements) || !sameMesh(source, solved)) {
    invalid("The working mesh no longer matches the solved heat input. Solve the current heat model again.");
  }
  const nodeIds = new Set<string>();
  const elementIds = new Set<string>();
  source.nodes.forEach((node, index) => {
    const previous = solved.nodes[index];
    if (typeof node.id !== "string" || !node.id.trim() || nodeIds.has(node.id) ||
      typeof node.fix_temperature !== "boolean" || !Number.isFinite(node.temperature ?? 0) || !Number.isFinite(node.heat_load ?? 0)) {
      invalid("Heat node identities, boundaries, and loads must be valid before projecting.");
    }
    nodeIds.add(node.id);
    if (node.id !== previous.id || node.fix_temperature !== previous.fix_temperature ||
      (node.temperature ?? 0) !== (previous.temperature ?? 0) || (node.heat_load ?? 0) !== (previous.heat_load ?? 0)) {
      invalid("The working heat model has changed. Solve the current heat model again before projecting.");
    }
  });
  source.elements.forEach((element, index) => {
    const connectivity: MeshElement = element;
    const indices = INDEX_FIELDS.map((key) => connectivity[key]).filter((value) => value !== undefined);
    if (indices.length !== arity || new Set(indices).size !== indices.length || indices.some((index) =>
      !Number.isInteger(index) || index < 0 || index >= source.nodes.length)) {
      invalid("The source mesh contains invalid element connectivity.");
    }
    const previous = solved.elements[index];
    const section = "area" in element ? element.area : element.thickness;
    const previousSection = "area" in previous ? previous.area : previous.thickness;
    if (typeof element.id !== "string" || !element.id.trim() || elementIds.has(element.id) ||
      !Number.isFinite(element.conductivity) || element.conductivity <= 0 || !Number.isFinite(section) || section <= 0) {
      invalid("Heat element identities, conductivity, and section must be valid before projecting.");
    }
    elementIds.add(element.id);
    if (element.id !== previous.id || element.conductivity !== previous.conductivity || section !== previousSection ||
      ("material_id" in element ? element.material_id : undefined) !== ("material_id" in previous ? previous.material_id : undefined)) {
      invalid("The working heat model has changed. Solve the current heat model again before projecting.");
    }
  });
}

function projectedTemperatures(source: HeatModel, result: HeatResult, arity: number) {
  validateHeatInput(source, result?.input, arity);
  if (!Array.isArray(result.nodes) || result.nodes.length !== source.nodes.length) invalid("A complete heat result is required; load or solve the full result before projecting.");
  const temperatures = new Array<number>(source.nodes.length);
  const seen = new Set<number>();
  for (const node of result.nodes) {
    if (!node) invalid("The heat result contains an invalid node record.");
    const index = node.index;
    if (!Number.isInteger(index) || index < 0 || index >= source.nodes.length || seen.has(index)) {
      invalid("The heat result contains duplicate or invalid node indices.");
    }
    if (node.id !== source.nodes[index].id || !samePosition(node, source.nodes[index])) {
      invalid("The heat result node identity or coordinates do not match the working mesh.");
    }
    if (!Number.isFinite(node.temperature)) invalid("Every projected node must have a finite temperature.");
    seen.add(index);
    // Match the engine's existing node-to-node, zero-reference copy contract.
    temperatures[index] = node.temperature;
  }
  return temperatures;
}

export function buildThermalBarFromHeatResult(
  source: HeatBar1dJobInput,
  result: HeatBar1dResult,
  current: ThermalBar1dJobInput,
): ThermalBar1dJobInput {
  const temperatures = projectedTemperatures(source, result, 2);
  const reuse = sameMesh(source, current);
  const fallback = defaultThermalBar1d.elements[0];
  return {
    nodes: source.nodes.map((node, index) => ({
      id: node.id, x: node.x,
      fix_x: reuse ? current.nodes[index].fix_x : node.fix_temperature,
      load_x: reuse ? current.nodes[index].load_x : 0,
      temperature_delta: temperatures[index],
    })),
    elements: source.elements.map((element, index) => ({
      id: element.id, node_i: element.node_i, node_j: element.node_j, area: element.area,
      youngs_modulus: reuse ? current.elements[index].youngs_modulus : fallback.youngs_modulus,
      thermal_expansion: reuse ? current.elements[index].thermal_expansion : fallback.thermal_expansion,
    })),
  };
}

type MechanicalPlaneSeed = PlaneTriangle2dJobInput | PlaneQuad2dJobInput | ThermalPlaneTriangle2dJobInput | ThermalPlaneQuad2dJobInput;
type PlaneSeed = MechanicalPlaneSeed | ElectrostaticPlaneTriangle2dJobInput | ElectrostaticPlaneQuad2dJobInput;

function isMechanicalSeed(model: PlaneSeed): model is MechanicalPlaneSeed {
  return model.nodes.every((node) => "fix_x" in node && "fix_y" in node) &&
    model.elements.every((element) => "youngs_modulus" in element && "poisson_ratio" in element);
}

function buildThermalPlane(
  source: HeatPlaneModel,
  result: HeatPlaneTriangle2dResult | HeatPlaneQuad2dResult,
  current: PlaneSeed,
  material: string,
  quad: boolean,
): ThermalPlaneTriangle2dJobInput | ThermalPlaneQuad2dJobInput {
  const temperatures = projectedTemperatures(source, result, quad ? 4 : 3);
  const seedModel = sameMesh(source, current) && isMechanicalSeed(current) ? current : null;
  const fallback = (quad ? defaultThermalPlaneQuad : defaultThermalPlaneTriangle).elements[0];
  const materials = seedModel?.materials?.length
    ? seedModel.materials.map((entry) => ({ ...entry }))
    : seedModel ? undefined : [createMaterialDefinition(material, 1, { id: "mat-1", poisson_ratio: fallback.poisson_ratio })];
  return {
    ...(materials ? { materials } : {}),
    nodes: source.nodes.map((node, index) => ({
      id: node.id, x: node.x, y: node.y,
      fix_x: seedModel ? seedModel.nodes[index].fix_x : node.fix_temperature,
      fix_y: seedModel ? seedModel.nodes[index].fix_y : node.fix_temperature,
      load_x: seedModel ? seedModel.nodes[index].load_x : 0,
      load_y: seedModel ? seedModel.nodes[index].load_y : 0,
      temperature_delta: temperatures[index],
    })),
    elements: source.elements.map((element, index) => {
      const seed = seedModel ? seedModel.elements[index] : fallback;
      const seedExpansion = "thermal_expansion" in seed ? seed.thermal_expansion : undefined;
      const materialId = seedModel ? seed.material_id : materials?.[0].id;
      if (materialId && !materials?.some((entry) => entry.id === materialId)) {
        invalid("The mechanical seed references a missing material. Repair the material binding before projecting.");
      }
      return {
        id: element.id, node_i: element.node_i, node_j: element.node_j, node_k: element.node_k,
        ...(quad && "node_l" in element ? { node_l: element.node_l } : {}),
        thickness: element.thickness,
        youngs_modulus: seedModel ? seed.youngs_modulus : materials![0].youngs_modulus,
        poisson_ratio: seed.poisson_ratio,
        thermal_expansion: typeof seedExpansion === "number" ? seedExpansion : fallback.thermal_expansion,
        ...(materialId ? { material_id: materialId } : {}),
      };
    }),
  };
}

export function buildThermalPlaneTriangleFromHeatResult(
  source: HeatPlaneTriangle2dJobInput, result: HeatPlaneTriangle2dResult, current: PlaneSeed, material: string,
): ThermalPlaneTriangle2dJobInput {
  return buildThermalPlane(source, result, current, material, false) as ThermalPlaneTriangle2dJobInput;
}

export function buildThermalPlaneQuadFromHeatResult(
  source: HeatPlaneQuad2dJobInput, result: HeatPlaneQuad2dResult, current: PlaneSeed, material: string,
): ThermalPlaneQuad2dJobInput {
  return buildThermalPlane(source, result, current, material, true) as ThermalPlaneQuad2dJobInput;
}
