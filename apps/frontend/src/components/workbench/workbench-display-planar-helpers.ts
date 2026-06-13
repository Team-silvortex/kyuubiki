import type {
  ElectrostaticPlaneQuad2dJobInput,
  ElectrostaticPlaneQuad2dResult,
  ElectrostaticPlaneTriangle2dJobInput,
  ElectrostaticPlaneTriangle2dResult,
  Frame2dJobInput,
  Frame2dResult,
  HeatBar1dJobInput,
  HeatBar1dResult,
  ThermalBar1dJobInput,
  ThermalBar1dResult,
  ThermalFrame2dJobInput,
  ThermalFrame2dResult,
  ThermalTruss2dJobInput,
  ThermalTruss2dResult,
  Truss2dJobInput,
  Truss2dResult,
} from "@/lib/api";
import type {
  DisplayTrussElement,
  DisplayTrussNode,
  PlaneElementDisplay,
  PlaneNodeDisplay,
} from "@/components/workbench/workbench-defaults";

export function buildDisplayTrussNodes(
  model: Truss2dJobInput,
  result: Truss2dResult | null,
  windowNodes?: Truss2dResult["nodes"],
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

export function buildDisplayTrussElements(
  model: Truss2dJobInput,
  result: Truss2dResult | null,
  windowElements?: Truss2dResult["elements"],
): DisplayTrussElement[] {
  const elements = windowElements ?? result?.elements;
  if (elements) {
    return elements.map((element) => ({
      index: element.index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: element.length,
      strain: element.strain,
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
    return {
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.sqrt(dx * dx + dy * dy),
      strain: 0,
      stress: 0,
      axial_force: 0,
      material_id: element.material_id,
    };
  });
}

export function buildDisplayFrameNodes(
  model: Frame2dJobInput,
  result: Frame2dResult | null,
  windowNodes?: Frame2dResult["nodes"],
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

export function buildDisplayThermalFrameNodes(
  model: ThermalFrame2dJobInput,
  result: ThermalFrame2dResult | null,
  windowNodes?: ThermalFrame2dResult["nodes"],
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

export function buildDisplayThermalBarNodes(
  model: ThermalBar1dJobInput,
  result: ThermalBar1dResult | null,
  windowNodes?: ThermalBar1dResult["nodes"],
): DisplayTrussNode[] {
  const source =
    windowNodes ??
    result?.nodes ??
    model.nodes.map((node, index) => ({
      index,
      id: node.id,
      x: node.x,
      ux: 0,
      temperature_delta: node.temperature_delta,
    }));

  return source.map((node, index) => ({
    index: node.index ?? index,
    id: model.nodes[node.index ?? index]?.id ?? node.id,
    x: model.nodes[node.index ?? index]?.x ?? node.x,
    y: 0,
    ux: node.ux ?? 0,
    uy: 0,
    fix_x: model.nodes[node.index ?? index]?.fix_x ?? false,
    fix_y: true,
    load_x: model.nodes[node.index ?? index]?.load_x ?? 0,
    load_y: 0,
  }));
}

export function buildDisplayThermalBarElements(
  model: ThermalBar1dJobInput,
  result: ThermalBar1dResult | null,
  windowElements?: ThermalBar1dResult["elements"],
): DisplayTrussElement[] {
  const source =
    windowElements ??
    result?.elements ??
    model.elements.map((element, index) => ({
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(model.nodes[element.node_j].x - model.nodes[element.node_i].x),
      stress: 0,
      axial_force: 0,
      average_temperature_delta: 0,
    } as ThermalBar1dResult["elements"][number]));

  return source.map((element, index) => ({
    index: element.index ?? index,
    id: model.elements[element.index ?? index]?.id ?? element.id,
    node_i: element.node_i,
    node_j: element.node_j,
    length: element.length,
    strain: element.total_strain ?? 0,
    stress: element.stress,
    axial_force: element.axial_force,
    axial_stress: element.stress,
    axial_force_i: element.axial_force,
    shear_force_i: 0,
    moment_i: 0,
    axial_force_j: element.axial_force,
    shear_force_j: 0,
    moment_j: 0,
  }));
}

export function buildDisplayHeatBarNodes(
  model: HeatBar1dJobInput,
  result: HeatBar1dResult | null,
  windowNodes?: HeatBar1dResult["nodes"],
): DisplayTrussNode[] {
  const source =
    windowNodes ??
    result?.nodes ??
    model.nodes.map((node, index) => ({
      index,
      id: node.id,
      x: node.x,
      temperature: node.temperature,
      heat_load: node.heat_load,
    }));

  return source.map((node, index) => ({
    index: node.index ?? index,
    id: model.nodes[node.index ?? index]?.id ?? node.id,
    x: model.nodes[node.index ?? index]?.x ?? node.x,
    y: 0,
    ux: 0,
    uy: 0,
    fix_x: model.nodes[node.index ?? index]?.fix_temperature ?? false,
    fix_y: true,
    load_x: model.nodes[node.index ?? index]?.heat_load ?? 0,
    load_y: 0,
    temperature_delta: node.temperature ?? model.nodes[node.index ?? index]?.temperature ?? 0,
  }));
}

export function buildDisplayHeatBarElements(
  model: HeatBar1dJobInput,
  result: HeatBar1dResult | null,
  windowElements?: HeatBar1dResult["elements"],
): DisplayTrussElement[] {
  const source =
    windowElements ??
    result?.elements ??
    model.elements.map((element, index) => ({
      index,
      id: element.id,
      node_i: element.node_i,
      node_j: element.node_j,
      length: Math.abs(model.nodes[element.node_j].x - model.nodes[element.node_i].x),
      average_temperature: 0,
      temperature_gradient: 0,
      heat_flux: 0,
    }));

  return source.map((element, index) => ({
    index: element.index ?? index,
    id: model.elements[element.index ?? index]?.id ?? element.id,
    node_i: element.node_i,
    node_j: element.node_j,
    length: element.length,
    strain: element.temperature_gradient ?? 0,
    stress: element.heat_flux ?? 0,
    axial_force: element.heat_flux ?? 0,
    axial_stress: element.heat_flux ?? 0,
    average_temperature_delta: element.average_temperature ?? 0,
    axial_force_i: element.heat_flux ?? 0,
    shear_force_i: 0,
    moment_i: 0,
    axial_force_j: element.heat_flux ?? 0,
    shear_force_j: 0,
    moment_j: 0,
  }));
}

export function buildDisplayElectrostaticPlaneNodes(
  model: ElectrostaticPlaneTriangle2dJobInput | ElectrostaticPlaneQuad2dJobInput,
  result: ElectrostaticPlaneTriangle2dResult | ElectrostaticPlaneQuad2dResult | null,
  windowNodes?: ElectrostaticPlaneTriangle2dResult["nodes"] | ElectrostaticPlaneQuad2dResult["nodes"],
): PlaneNodeDisplay[] {
  const nodes = windowNodes ?? result?.nodes;
  if (nodes) {
    return nodes.map((node, index) => ({
      index: typeof node.index === "number" ? node.index : index,
      id: node.id,
      x: node.x,
      y: node.y,
      ux: 0,
      uy: 0,
      potential: node.potential,
      charge_density: node.charge_density,
      fix_potential: model.nodes[node.index]?.fix_potential ?? false,
      fix_x: false,
      fix_y: false,
      load_x: 0,
      load_y: 0,
    }));
  }
  return model.nodes.map((node, index) => ({ index, id: node.id, x: node.x, y: node.y, ux: 0, uy: 0, potential: node.potential ?? 0, charge_density: node.charge_density ?? 0, fix_potential: node.fix_potential, fix_x: false, fix_y: false, load_x: 0, load_y: 0 }));
}

export function buildDisplayElectrostaticPlaneElements(
  model: ElectrostaticPlaneTriangle2dJobInput | ElectrostaticPlaneQuad2dJobInput,
  result: ElectrostaticPlaneTriangle2dResult | ElectrostaticPlaneQuad2dResult | null,
  windowElements?: ElectrostaticPlaneTriangle2dResult["elements"] | ElectrostaticPlaneQuad2dResult["elements"],
): PlaneElementDisplay[] {
  const elements = windowElements ?? result?.elements;
  if (elements) {
    return elements.map((element) => ({ ...element, material_id: model.elements[element.index]?.material_id }));
  }
  return model.elements.map((element, index) => ({ index, id: element.id, node_i: element.node_i, node_j: element.node_j, node_k: element.node_k, node_l: "node_l" in element ? element.node_l : undefined, average_potential: 0, potential_gradient_x: 0, potential_gradient_y: 0, electric_field_x: 0, electric_field_y: 0, electric_field_magnitude: 0, electric_flux_density_x: 0, electric_flux_density_y: 0, electric_flux_density_magnitude: 0, material_id: element.material_id }));
}

export function buildDisplayThermalTrussNodes(
  model: ThermalTruss2dJobInput,
  result: ThermalTruss2dResult | null,
  windowNodes?: ThermalTruss2dResult["nodes"],
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

export function buildDisplayThermalTrussElements(
  model: ThermalTruss2dJobInput,
  result: ThermalTruss2dResult | null,
  windowElements?: ThermalTruss2dResult["elements"],
): DisplayTrussElement[] {
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
      axial_stress: element.stress,
      axial_force_i: element.axial_force,
      shear_force_i: 0,
      moment_i: 0,
      axial_force_j: element.axial_force,
      shear_force_j: 0,
      moment_j: 0,
      material_id: model.elements[element.index]?.material_id,
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
      shear_force_i: 0,
      moment_i: 0,
      axial_force_j: 0,
      shear_force_j: 0,
      moment_j: 0,
      material_id: element.material_id,
    };
  });
}

export function buildDisplayFrameElements(
  model: Frame2dJobInput,
  result: Frame2dResult | null,
  windowElements?: Frame2dResult["elements"],
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
      stress: element.max_combined_stress,
      axial_force: Math.max(Math.abs(element.moment_i), Math.abs(element.moment_j)),
      axial_stress: element.axial_stress,
      max_bending_stress: element.max_bending_stress,
      max_combined_stress: element.max_combined_stress,
      axial_force_i: element.axial_force_i,
      shear_force_i: element.shear_force_i,
      moment_i: element.moment_i,
      axial_force_j: element.axial_force_j,
      shear_force_j: element.shear_force_j,
      moment_j: element.moment_j,
      material_id: model.elements[element.index]?.material_id,
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
      material_id: element.material_id,
    };
  });
}

export function buildDisplayThermalFrameElements(
  model: ThermalFrame2dJobInput,
  result: ThermalFrame2dResult | null,
  windowElements?: ThermalFrame2dResult["elements"],
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
      stress: element.max_combined_stress,
      axial_force: Math.max(Math.abs(element.moment_i), Math.abs(element.moment_j)),
      axial_stress: element.axial_stress,
      max_bending_stress: element.max_bending_stress,
      max_combined_stress: element.max_combined_stress,
      axial_force_i: element.axial_force_i,
      shear_force_i: element.shear_force_i,
      moment_i: element.moment_i,
      axial_force_j: element.axial_force_j,
      shear_force_j: element.shear_force_j,
      moment_j: element.moment_j,
      material_id: model.elements[element.index]?.material_id,
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
      material_id: element.material_id,
    };
  });
}
