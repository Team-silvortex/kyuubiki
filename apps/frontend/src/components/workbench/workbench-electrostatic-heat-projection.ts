"use client";

import type {
  ElectrostaticPlaneQuad2dResult,
  ElectrostaticPlaneTriangle2dResult,
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
} from "@/lib/api";

type ElectrostaticPlaneResult = ElectrostaticPlaneTriangle2dResult | ElectrostaticPlaneQuad2dResult;
type HeatPlaneModel = HeatPlaneTriangle2dJobInput | HeatPlaneQuad2dJobInput;

type ElectrostaticHeatProjectionOptions = {
  coldTemperature?: number;
  conductivity?: number;
  heatLoadScale?: number;
  hotTemperature?: number;
};

const DEFAULT_COLD_TEMPERATURE = 20;
const DEFAULT_CONDUCTIVITY = 45;
const DEFAULT_HEAT_LOAD_SCALE = 50;
const DEFAULT_HOT_TEMPERATURE = 100;

function finiteNumber(value: unknown, fallback: number) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

function elementNodeIndices(element: Record<string, unknown>) {
  return ["node_i", "node_j", "node_k", "node_l"]
    .map((key) => finiteNumber(element[key], Number.NaN))
    .filter((value) => Number.isInteger(value) && value >= 0);
}

function buildNodeHeatLoads(result: ElectrostaticPlaneResult, heatLoadScale: number) {
  const sums = new Array(result.nodes.length).fill(0) as number[];
  const counts = new Array(result.nodes.length).fill(0) as number[];

  for (const element of result.elements) {
    const load = finiteNumber(element.electric_field_magnitude, 0) * heatLoadScale;
    for (const nodeIndex of elementNodeIndices(element as unknown as Record<string, unknown>)) {
      if (nodeIndex >= sums.length) continue;
      sums[nodeIndex] += load;
      counts[nodeIndex] += 1;
    }
  }

  return sums.map((sum, index) => (counts[index] > 0 ? sum / counts[index] : 0));
}

function normalizedTemperature(
  potential: number,
  minPotential: number,
  maxPotential: number,
  coldTemperature: number,
  hotTemperature: number,
) {
  if (Math.abs(maxPotential - minPotential) < 1.0e-12) return hotTemperature;
  const ratio = (potential - minPotential) / (maxPotential - minPotential);
  return coldTemperature + ratio * (hotTemperature - coldTemperature);
}

function fixedTemperatureNodeIndices(result: ElectrostaticPlaneResult) {
  const inputNodes = result.input.nodes;
  const fixed = new Set<number>();
  inputNodes.forEach((node, index) => {
    if (node.fix_potential) fixed.add(index);
  });
  if (fixed.size > 0) return fixed;

  let minIndex = 0;
  let maxIndex = 0;
  result.nodes.forEach((node, index) => {
    if (finiteNumber(node.potential, 0) < finiteNumber(result.nodes[minIndex]?.potential, 0)) minIndex = index;
    if (finiteNumber(node.potential, 0) > finiteNumber(result.nodes[maxIndex]?.potential, 0)) maxIndex = index;
  });
  fixed.add(minIndex);
  fixed.add(maxIndex);
  return fixed;
}

function buildHeatNodes(
  result: ElectrostaticPlaneResult,
  options: Required<ElectrostaticHeatProjectionOptions>,
) {
  const potentials = result.nodes.map((node, index) =>
    finiteNumber(node.potential, finiteNumber(result.input.nodes[index]?.potential, 0)),
  );
  const minPotential = Math.min(...potentials);
  const maxPotential = Math.max(...potentials);
  const fixedNodes = fixedTemperatureNodeIndices(result);
  const heatLoads = buildNodeHeatLoads(result, options.heatLoadScale);

  return result.nodes.map((node, index) => {
    const inputIndex = Number.isInteger(node.index) ? node.index : index;
    const inputNode = result.input.nodes[inputIndex] ?? result.input.nodes[index];
    const potential = potentials[index] ?? 0;
    const fixTemperature = fixedNodes.has(inputIndex);
    return {
      id: inputNode?.id ?? node.id ?? `h${index}`,
      x: finiteNumber(node.x, finiteNumber(inputNode?.x, 0)),
      y: finiteNumber(node.y, finiteNumber(inputNode?.y, 0)),
      fix_temperature: fixTemperature,
      temperature: fixTemperature
        ? normalizedTemperature(
          potential,
          minPotential,
          maxPotential,
          options.coldTemperature,
          options.hotTemperature,
        )
        : 0,
      heat_load: heatLoads[node.index] ?? heatLoads[index] ?? 0,
    };
  });
}

function resolveOptions(options: ElectrostaticHeatProjectionOptions = {}) {
  return {
    coldTemperature: options.coldTemperature ?? DEFAULT_COLD_TEMPERATURE,
    conductivity: options.conductivity ?? DEFAULT_CONDUCTIVITY,
    heatLoadScale: options.heatLoadScale ?? DEFAULT_HEAT_LOAD_SCALE,
    hotTemperature: options.hotTemperature ?? DEFAULT_HOT_TEMPERATURE,
  };
}

function materialIdFor(index: number, result: ElectrostaticPlaneResult, currentHeatModel?: HeatPlaneModel) {
  return result.input.elements[index]?.material_id ?? currentHeatModel?.elements[index]?.material_id;
}

export function projectElectrostaticPlaneTriangleResultToHeatModel(
  result: ElectrostaticPlaneTriangle2dResult,
  currentHeatModel?: HeatPlaneTriangle2dJobInput,
  rawOptions?: ElectrostaticHeatProjectionOptions,
): HeatPlaneTriangle2dJobInput {
  const options = resolveOptions(rawOptions);
  return {
    nodes: buildHeatNodes(result, options),
    elements: result.elements.map((element, index) => ({
      id: result.input.elements[element.index]?.id ?? element.id ?? `het${index}`,
      node_i: element.node_i,
      node_j: element.node_j,
      node_k: element.node_k,
      thickness: finiteNumber(result.input.elements[element.index]?.thickness, currentHeatModel?.elements[index]?.thickness ?? 0.02),
      conductivity: finiteNumber(currentHeatModel?.elements[index]?.conductivity, options.conductivity),
      ...(materialIdFor(element.index, result, currentHeatModel) ? { material_id: materialIdFor(element.index, result, currentHeatModel) } : {}),
    })),
    ...(result.input.materials ? { materials: result.input.materials } : {}),
  };
}

export function projectElectrostaticPlaneQuadResultToHeatModel(
  result: ElectrostaticPlaneQuad2dResult,
  currentHeatModel?: HeatPlaneQuad2dJobInput,
  rawOptions?: ElectrostaticHeatProjectionOptions,
): HeatPlaneQuad2dJobInput {
  const options = resolveOptions(rawOptions);
  return {
    nodes: buildHeatNodes(result, options),
    elements: result.elements.map((element, index) => ({
      id: result.input.elements[element.index]?.id ?? element.id ?? `heq${index}`,
      node_i: element.node_i,
      node_j: element.node_j,
      node_k: element.node_k,
      node_l: element.node_l,
      thickness: finiteNumber(result.input.elements[element.index]?.thickness, currentHeatModel?.elements[index]?.thickness ?? 0.02),
      conductivity: finiteNumber(currentHeatModel?.elements[index]?.conductivity, options.conductivity),
      ...(materialIdFor(element.index, result, currentHeatModel) ? { material_id: materialIdFor(element.index, result, currentHeatModel) } : {}),
    })),
    ...(result.input.materials ? { materials: result.input.materials } : {}),
  };
}
