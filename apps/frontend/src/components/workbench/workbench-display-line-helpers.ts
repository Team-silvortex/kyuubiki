import type {
  Beam1dJobInput,
  Beam1dResult,
  Spring1dJobInput,
  Spring1dResult,
  Spring2dJobInput,
  Spring2dResult,
  ThermalBeam1dJobInput,
  ThermalBeam1dResult,
  Torsion1dJobInput,
  Torsion1dResult,
} from "@/lib/api";
import type {
  DisplayTrussElement,
  DisplayTrussNode,
} from "@/components/workbench/workbench-defaults";

export function buildDisplayBeamNodes(
  model: Beam1dJobInput,
  result: Beam1dResult | null,
  windowNodes?: Beam1dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: 0,
      ux: 0,
      uy: node.uy,
      fix_x: false,
      fix_y: model.nodes[node.index]?.fix_y ?? false,
      load_x: 0,
      load_y: model.nodes[node.index]?.load_y ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: false,
    fix_y: node.fix_y,
    load_x: 0,
    load_y: node.load_y,
  }));
}

export function buildDisplayThermalBeamNodes(
  model: ThermalBeam1dJobInput,
  result: ThermalBeam1dResult | null,
  windowNodes?: ThermalBeam1dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: 0,
      ux: 0,
      uy: node.uy,
      fix_x: false,
      fix_y: model.nodes[node.index]?.fix_y ?? false,
      load_x: 0,
      load_y: model.nodes[node.index]?.load_y ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: false,
    fix_y: node.fix_y,
    load_x: 0,
    load_y: node.load_y,
  }));
}

export function buildDisplayTorsionNodes(
  model: Torsion1dJobInput,
  result: Torsion1dResult | null,
  windowNodes?: Torsion1dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: 0,
      ux: 0,
      uy: 0,
      fix_x: false,
      fix_y: true,
      load_x: 0,
      load_y: model.nodes[node.index]?.torque_z ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: false,
    fix_y: true,
    load_x: 0,
    load_y: node.torque_z,
  }));
}

export function buildDisplaySpringNodes(
  model: Spring1dJobInput,
  result: Spring1dResult | null,
  windowNodes?: Spring1dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: 0,
      ux: node.ux,
      uy: 0,
      fix_x: model.nodes[node.index]?.fix_x ?? false,
      fix_y: true,
      load_x: model.nodes[node.index]?.load_x ?? 0,
      load_y: 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: node.fix_x,
    fix_y: true,
    load_x: node.load_x,
    load_y: 0,
  }));
}

export function buildDisplayBeamElements(
  model: Beam1dJobInput,
  result: Beam1dResult | null,
  windowElements?: Beam1dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;
  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: 0,
      stress: element.max_bending_stress,
      axial_force: Math.max(Math.abs(element.moment_i), Math.abs(element.moment_j)),
      max_bending_stress: element.max_bending_stress,
      shear_force_i: element.shear_force_i,
      moment_i: element.moment_i,
      shear_force_j: element.shear_force_j,
      moment_j: element.moment_j,
      material_id: model.elements[element.index]?.material_id,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(dx),
      strain: 0,
      stress: 0,
      axial_force: 0,
      material_id: element.material_id,
    };
  });
}

export function buildDisplayThermalBeamElements(
  model: ThermalBeam1dJobInput,
  result: ThermalBeam1dResult | null,
  windowElements?: ThermalBeam1dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;
  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.thermal_curvature,
      stress: element.max_bending_stress,
      axial_force: Math.max(Math.abs(element.moment_i), Math.abs(element.moment_j)),
      temperature_gradient_y: element.temperature_gradient_y,
      thermal_curvature: element.thermal_curvature,
      max_bending_stress: element.max_bending_stress,
      shear_force_i: element.shear_force_i,
      moment_i: element.moment_i,
      shear_force_j: element.shear_force_j,
      moment_j: element.moment_j,
      material_id: model.elements[element.index]?.material_id,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(dx),
      strain: 0,
      stress: 0,
      axial_force: 0,
      temperature_gradient_y: element.temperature_gradient_y,
      thermal_curvature: 0,
      max_bending_stress: 0,
      shear_force_i: 0,
      moment_i: 0,
      shear_force_j: 0,
      moment_j: 0,
      material_id: element.material_id,
    };
  });
}

export function buildDisplayTorsionElements(
  model: Torsion1dJobInput,
  result: Torsion1dResult | null,
  windowElements?: Torsion1dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;
  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.twist,
      stress: element.shear_stress,
      axial_force: Math.abs(element.torque),
      max_bending_stress: element.shear_stress,
      moment_i: element.torque,
      moment_j: element.torque,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(dx),
      strain: 0,
      stress: 0,
      axial_force: 0,
      max_bending_stress: 0,
      moment_i: 0,
      moment_j: 0,
    };
  });
}

export function buildDisplaySpringElements(
  model: Spring1dJobInput,
  result: Spring1dResult | null,
  windowElements?: Spring1dResult["elements"],
): DisplayTrussElement[] {
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
      axial_stress: element.force,
      axial_force_i: element.force,
      axial_force_j: element.force,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(dx),
      strain: 0,
      stress: 0,
      axial_force: 0,
      axial_stress: 0,
      axial_force_i: 0,
      axial_force_j: 0,
    };
  });
}

export function buildDisplaySpring2dNodes(
  model: Spring2dJobInput,
  result: Spring2dResult | null,
  windowNodes?: Spring2dResult["nodes"],
): DisplayTrussNode[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: node.y,
      ux: node.ux,
      uy: node.uy,
      fix_x: model.nodes[node.index]?.fix_x ?? false,
      fix_y: model.nodes[node.index]?.fix_y ?? false,
      load_x: model.nodes[node.index]?.load_x ?? 0,
      load_y: model.nodes[node.index]?.load_y ?? 0,
    }));
  }

  return model.nodes.map((node, index) => ({
    index,
    id: node.id,
    x: node.x,
    y: node.y,
    ux: 0,
    uy: 0,
    fix_x: node.fix_x,
    fix_y: node.fix_y,
    load_x: node.load_x,
    load_y: node.load_y,
  }));
}

export function buildDisplaySpring2dElements(
  model: Spring2dJobInput,
  result: Spring2dResult | null,
  windowElements?: Spring2dResult["elements"],
): DisplayTrussElement[] {
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
      axial_stress: element.force,
      axial_force_i: element.force,
      axial_force_j: element.force,
    }));
  }

  return model.elements.map((element, index) => {
    const nodeI = model.nodes[element.node_i];
    const nodeJ = model.nodes[element.node_j];
    const dx = (nodeJ?.x ?? 0) - (nodeI?.x ?? 0);
    const dy = (nodeJ?.y ?? 0) - (nodeI?.y ?? 0);
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy),
      strain: 0,
      stress: 0,
      axial_force: 0,
      axial_stress: 0,
      axial_force_i: 0,
      axial_force_j: 0,
    };
  });
}
