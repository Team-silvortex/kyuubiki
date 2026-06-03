import type {
  Spring3dJobInput,
  Spring3dResult,
  ThermalTruss3dJobInput,
  ThermalTruss3dResult,
  Truss3dJobInput,
  Truss3dResult,
} from "@/lib/api";
import type {
  DisplayTruss3dElement,
  DisplayTruss3dNode,
} from "@/components/workbench/workbench-defaults";

export function buildDisplayTruss3dNodes(
  model: Truss3dJobInput,
  result: Truss3dResult | null,
  windowNodes?: Truss3dResult["nodes"],
): DisplayTruss3dNode[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: node.y,
      z: node.z,
      ux: node.ux,
      uy: node.uy,
      uz: node.uz,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: node.y,
    z: node.z,
    ux: 0,
    uy: 0,
    uz: 0,
  }));
}

export function buildDisplayTruss3dElements(
  model: Truss3dJobInput,
  result: Truss3dResult | null,
  windowElements?: Truss3dResult["elements"],
): DisplayTruss3dElement[] {
  const elements = windowElements ?? result?.elements;
  if (elements) {
    return elements.map((element) => ({
      ...element,
      material_id: model.elements[element.index]?.material_id,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    const dy = (nodeJ?.y ?? 0) - (nodeI?.y ?? 0);
    const dz = (nodeJ?.z ?? 0) - (nodeI?.z ?? 0);
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy + dz * dz),
      strain: 0,
      stress: 0,
      axial_force: 0,
      material_id: element.material_id,
    };
  });
}

export function projectTruss3dPoint(
  node: { x: number; y: number; z: number },
  toSvgPoint: (node: { x: number; y: number }, bounds: { minX: number; maxX: number; minY: number; maxY: number; width: number; height: number }) => { x: number; y: number },
  bounds: { minX: number; maxX: number; minY: number; maxY: number; width: number; height: number },
) {
  const isoX = node.x - node.y * 0.55;
  const isoY = node.z + node.y * 0.35;
  return toSvgPoint({ x: isoX, y: isoY }, bounds);
}

export function buildDisplaySpring3dNodes(
  model: Spring3dJobInput,
  result: Spring3dResult | null,
  windowNodes?: Spring3dResult["nodes"],
): DisplayTruss3dNode[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: node.y,
      z: node.z,
      ux: node.ux,
      uy: node.uy,
      uz: node.uz,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: node.y,
    z: node.z,
    ux: 0,
    uy: 0,
    uz: 0,
  }));
}

export function buildDisplaySpring3dElements(
  model: Spring3dJobInput,
  result: Spring3dResult | null,
  windowElements?: Spring3dResult["elements"],
): DisplayTruss3dElement[] {
  const elements = windowElements ?? result?.elements;
  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.extension,
      stress: element.force,
      axial_force: element.force,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    const dy = (nodeJ?.y ?? 0) - (nodeI?.y ?? 0);
    const dz = (nodeJ?.z ?? 0) - (nodeI?.z ?? 0);
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy + dz * dz),
      strain: 0,
      stress: 0,
      axial_force: 0,
    };
  });
}

export function buildDisplayThermalTruss3dNodes(
  model: ThermalTruss3dJobInput,
  result: ThermalTruss3dResult | null,
  windowNodes?: ThermalTruss3dResult["nodes"],
): DisplayTruss3dNode[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: node.y,
      z: node.z,
      ux: node.ux,
      uy: node.uy,
      uz: node.uz,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: node.y,
    z: node.z,
    ux: 0,
    uy: 0,
    uz: 0,
  }));
}

export function buildDisplayThermalTruss3dElements(
  model: ThermalTruss3dJobInput,
  result: ThermalTruss3dResult | null,
  windowElements?: ThermalTruss3dResult["elements"],
): DisplayTruss3dElement[] {
  const elements = windowElements ?? result?.elements;
  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.total_strain,
      stress: element.stress,
      axial_force: element.axial_force,
      material_id: model.elements[element.index]?.material_id,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    const dy = (nodeJ?.y ?? 0) - (nodeI?.y ?? 0);
    const dz = (nodeJ?.z ?? 0) - (nodeI?.z ?? 0);
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy + dz * dz),
      strain: 0,
      stress: 0,
      axial_force: 0,
      material_id: element.material_id,
    };
  });
}
