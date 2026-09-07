"use client";

import type {
  ElectrostaticPlaneQuad2dJobInput, ElectrostaticPlaneQuad2dResult,
  ElectrostaticPlaneTriangle2dJobInput, ElectrostaticPlaneTriangle2dResult,
  HeatPlaneQuad2dJobInput, HeatPlaneTriangle2dJobInput, ModelMaterial,
} from "@/lib/api";

type ElectrostaticPlaneResult = ElectrostaticPlaneTriangle2dResult | ElectrostaticPlaneQuad2dResult;
type ElectrostaticPlaneModel = ElectrostaticPlaneTriangle2dJobInput | ElectrostaticPlaneQuad2dJobInput;
type HeatPlaneModel = HeatPlaneTriangle2dJobInput | HeatPlaneQuad2dJobInput;
type Mesh = ElectrostaticPlaneModel | HeatPlaneModel;
type Connectivity = { node_i: number; node_j: number; node_k: number; node_l?: number };
type ElectrostaticHeatProjectionOptions = {
  coldTemperature?: number;
  conductivity?: number;
  heatLoadScale?: number;
  hotTemperature?: number;
};

function invalid(reason: string): never {
  throw new Error(`ELECTROSTATIC_HEAT_PROJECTION_INVALID: ${reason}`);
}

function finite(value: number, label: string) {
  if (!Number.isFinite(value)) invalid(`${label} must be finite.`);
  return value;
}

function positive(value: number, label: string) {
  if (finite(value, label) <= 0) invalid(`${label} must be positive.`);
  return value;
}

function resolveOptions(raw: ElectrostaticHeatProjectionOptions = {}) {
  const options = {
    coldTemperature: raw.coldTemperature === undefined ? 20 : raw.coldTemperature,
    conductivity: raw.conductivity === undefined ? 45 : raw.conductivity,
    heatLoadScale: raw.heatLoadScale === undefined ? 50 : raw.heatLoadScale,
    hotTemperature: raw.hotTemperature === undefined ? 100 : raw.hotTemperature,
  };
  finite(options.coldTemperature, "Cold temperature");
  finite(options.hotTemperature, "Hot temperature");
  const span = finite(options.hotTemperature - options.coldTemperature, "Temperature range");
  if (span < 0) invalid("Hot temperature cannot be lower than cold temperature.");
  positive(options.conductivity, "Conductivity");
  if (finite(options.heatLoadScale, "Heat-load scale") < 0) invalid("Heat-load scale cannot be negative.");
  return options;
}

function indices(element: Connectivity) {
  return element.node_l === undefined ? [element.node_i, element.node_j, element.node_k]
    : [element.node_i, element.node_j, element.node_k, element.node_l];
}

function samePosition(left: { x: number; y: number }, right: { x: number; y: number }) {
  return Number.isFinite(left.x) && Number.isFinite(left.y) && Number.isFinite(right.x) && Number.isFinite(right.y) &&
    Math.abs(left.x - right.x) <= 1e-9 && Math.abs(left.y - right.y) <= 1e-9;
}

function sameConnectivity(left: Connectivity, right: Connectivity) {
  return left.node_i === right.node_i && left.node_j === right.node_j && left.node_k === right.node_k && left.node_l === right.node_l;
}

function sameMesh(left: Mesh, right: Mesh) {
  return left.nodes.length === right.nodes.length && left.elements.length === right.elements.length &&
    left.nodes.every((node, index) => samePosition(node, right.nodes[index])) &&
    left.elements.every((element, index) => sameConnectivity(element, right.elements[index]));
}

function orderedRecords<T extends { index: number; id: string }>(records: T[], source: { id: string }[], label: string): T[] {
  if (!Array.isArray(records) || records.length !== source.length) invalid(`A complete ${label} result is required.`);
  const ordered = new Array<T>(source.length);
  const identities = new Set<string>();
  for (const record of records) {
    if (!record || !Number.isInteger(record.index) || record.index < 0 || record.index >= source.length || ordered[record.index]) {
      invalid(`The ${label} result contains duplicate or invalid indices.`);
    }
    if (typeof record.id !== "string" || !record.id.trim() || record.id !== source[record.index]?.id || identities.has(record.id)) {
      invalid(`The ${label} result identity does not match its input.`);
    }
    identities.add(record.id);
    ordered[record.index] = record;
  }
  return ordered;
}

function validateResult(result: ElectrostaticPlaneResult, quad: boolean, current?: ElectrostaticPlaneModel) {
  const input = result?.input;
  if (!input || !Array.isArray(input.nodes) || !input.nodes.length || !Array.isArray(input.elements) || !input.elements.length) {
    invalid("The solved electrostatic input must contain a nonempty mesh.");
  }
  const nodes = orderedRecords(result.nodes, input.nodes, "node");
  const elements = orderedRecords(result.elements, input.elements, "element");
  input.nodes.forEach((node, index) => {
    if (!samePosition(node, nodes[index])) invalid("Result node coordinates do not match the solved input.");
    if (typeof node.fix_potential !== "boolean") invalid("Every source node must declare its potential boundary.");
    finite(nodes[index].potential, "Nodal potential");
  });
  input.elements.forEach((element, index) => {
    const connectivity = indices(element);
    if (connectivity.length !== (quad ? 4 : 3) || new Set(connectivity).size !== connectivity.length ||
      connectivity.some((node) => !Number.isInteger(node) || node < 0 || node >= input.nodes.length) ||
      !sameConnectivity(element, elements[index])) invalid("Element connectivity does not match the solved mesh.");
    positive(element.thickness, "Source thickness");
    if (finite(elements[index].electric_field_magnitude, "Electric-field magnitude") < 0) invalid("Electric-field magnitude cannot be negative.");
  });
  if (current && (!sameMesh(input, current) ||
    input.nodes.some((node, index) => {
      const other = current.nodes[index];
      return node.id !== other.id || node.fix_potential !== other.fix_potential ||
        (node.potential ?? 0) !== (other.potential ?? 0) || (node.charge_density ?? 0) !== (other.charge_density ?? 0);
    }) || input.elements.some((element, index) => {
      const other = current.elements[index];
      return element.id !== other.id || element.thickness !== other.thickness ||
        element.permittivity !== other.permittivity || element.material_id !== other.material_id;
    }))) invalid("The working electrostatic model has changed. Solve the current model again before projecting.");
  return { input, nodes, elements };
}

function materialLookup(materials: ModelMaterial[] | undefined) {
  const lookup = new Map<string, ModelMaterial>();
  for (const material of materials ?? []) {
    if (typeof material?.id !== "string" || !material.id.trim() || lookup.has(material.id)) invalid("Material identifiers must be nonempty and unique.");
    lookup.set(material.id, material);
  }
  return lookup;
}

function buildHeatModel(
  result: ElectrostaticPlaneResult, currentHeatModel: HeatPlaneModel | undefined,
  rawOptions: ElectrostaticHeatProjectionOptions | undefined, quad: boolean, current?: ElectrostaticPlaneModel,
): HeatPlaneModel {
  const options = resolveOptions(rawOptions);
  const { input, nodes, elements } = validateResult(result, quad, current);
  const seed = currentHeatModel && sameMesh(input, currentHeatModel) ? currentHeatModel : undefined;
  const sourceMaterials = materialLookup(input.materials);
  const seedMaterials = materialLookup(seed?.materials);
  const outputMaterials = new Map<string, ModelMaterial>();
  const heatLoads = new Array<number>(nodes.length).fill(0);
  const counts = new Array<number>(nodes.length).fill(0);
  let minIndex = 0;
  let maxIndex = 0;
  nodes.forEach((node, index) => {
    if (node.potential < nodes[minIndex].potential) minIndex = index;
    if (node.potential > nodes[maxIndex].potential) maxIndex = index;
  });
  const minPotential = nodes[minIndex].potential;
  const potentialSpan = finite(nodes[maxIndex].potential - minPotential, "Potential range");
  const fixed = new Set<number>();
  input.nodes.forEach((node, index) => { if (node.fix_potential) fixed.add(index); });
  if (fixed.size === 0) { fixed.add(minIndex); fixed.add(maxIndex); }
  for (const element of elements) {
    const load = finite(element.electric_field_magnitude * options.heatLoadScale, "Projected heat load");
    for (const index of indices(element)) {
      // Online mean avoids overflowing an intermediate sum of finite, nonnegative loads.
      heatLoads[index] += (load - heatLoads[index]) / ++counts[index];
    }
  }
  const heatNodes = input.nodes.map((node, index) => {
    const seedNode = seed?.nodes[index];
    if (seedNode && typeof seedNode.fix_temperature !== "boolean") invalid("Every seed node must declare its thermal boundary.");
    const fixTemperature = seedNode ? seedNode.fix_temperature : fixed.has(index);
    const ratio = potentialSpan < 1e-12 ? 1 : (nodes[index].potential - minPotential) / potentialSpan;
    const temperature = seedNode ? finite(seedNode.temperature === undefined ? 0 : seedNode.temperature, "Seed temperature")
      : fixTemperature ? finite(options.coldTemperature + ratio * (options.hotTemperature - options.coldTemperature), "Projected temperature") : 0;
    return { id: node.id, x: node.x, y: node.y, fix_temperature: fixTemperature, temperature, heat_load: heatLoads[index] };
  });
  const heatElements = input.elements.map((element, index) => {
    const seedElement = seed?.elements[index];
    const materialId = seedElement?.material_id ?? element.material_id;
    if (materialId !== undefined) {
      const material = (seedElement?.material_id !== undefined ? seedMaterials : sourceMaterials).get(materialId);
      if (!material) invalid("An element references a missing material in its source library.");
      const existing = outputMaterials.get(materialId);
      if (existing && (existing.name !== material.name || existing.youngs_modulus !== material.youngs_modulus ||
        existing.poisson_ratio !== material.poisson_ratio)) invalid("A material identifier has conflicting definitions across the source and seed libraries.");
      outputMaterials.set(materialId, { ...material });
    }
    return {
      id: element.id, node_i: element.node_i, node_j: element.node_j, node_k: element.node_k,
      ...(quad && "node_l" in element ? { node_l: element.node_l } : {}), thickness: element.thickness,
      conductivity: positive(seedElement ? seedElement.conductivity : options.conductivity, "Seed conductivity"),
      ...(materialId !== undefined ? { material_id: materialId } : {}),
    };
  });
  return { nodes: heatNodes, elements: heatElements,
    ...(outputMaterials.size ? { materials: [...outputMaterials.values()] } : {}) };
}

export function projectElectrostaticPlaneTriangleResultToHeatModel(
  result: ElectrostaticPlaneTriangle2dResult, currentHeatModel?: HeatPlaneTriangle2dJobInput,
  rawOptions?: ElectrostaticHeatProjectionOptions, currentElectrostaticModel?: ElectrostaticPlaneTriangle2dJobInput,
): HeatPlaneTriangle2dJobInput {
  return buildHeatModel(result, currentHeatModel, rawOptions, false, currentElectrostaticModel) as HeatPlaneTriangle2dJobInput;
}

export function projectElectrostaticPlaneQuadResultToHeatModel(
  result: ElectrostaticPlaneQuad2dResult, currentHeatModel?: HeatPlaneQuad2dJobInput,
  rawOptions?: ElectrostaticHeatProjectionOptions, currentElectrostaticModel?: ElectrostaticPlaneQuad2dJobInput,
): HeatPlaneQuad2dJobInput {
  return buildHeatModel(result, currentHeatModel, rawOptions, true, currentElectrostaticModel) as HeatPlaneQuad2dJobInput;
}
