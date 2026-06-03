import type {
  AxialBarResult,
  Beam1dResult,
  Frame2dResult,
  HeatBar1dResult,
  HeatPlaneQuad2dResult,
  HeatPlaneTriangle2dResult,
  PlaneQuad2dResult,
  PlaneTriangle2dResult,
  Spring1dResult,
  Spring2dResult,
  Spring3dResult,
  ThermalBar1dResult,
  ThermalBeam1dResult,
  ThermalFrame2dResult,
  ThermalPlaneQuad2dResult,
  ThermalPlaneTriangle2dResult,
  ThermalTruss2dResult,
  ThermalTruss3dResult,
  Torsion1dResult,
  Truss2dResult,
  Truss3dResult,
} from "@/lib/api";

export function toCsvRow(values: Array<string | number | boolean | null | undefined>) {
  return values
    .map((value) => {
      if (value === null || value === undefined) return "";
      const text = String(value);
      return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
    })
    .join(",");
}

export function isAxialResult(value: unknown): value is AxialBarResult {
  return typeof value === "object" && value !== null && "tip_displacement" in value;
}

export function isThermalBar1dResult(value: unknown): value is ThermalBar1dResult {
  return typeof value === "object" && value !== null && "max_temperature_delta" in value && "max_axial_force" in value && "nodes" in value && "elements" in value;
}

export function isHeatBar1dResult(value: unknown): value is HeatBar1dResult {
  return typeof value === "object" && value !== null && "max_temperature" in value && "max_heat_flux" in value && "nodes" in value && "elements" in value;
}

export function isThermalBeam1dResult(value: unknown): value is ThermalBeam1dResult {
  return typeof value === "object" && value !== null && "max_temperature_gradient" in value && "max_rotation" in value && "max_moment" in value && "nodes" in value && "elements" in value;
}

export function isThermalFrame2dResult(value: unknown): value is ThermalFrame2dResult {
  return typeof value === "object" && value !== null && "max_temperature_delta" in value && "max_temperature_gradient" in value && "max_rotation" in value && "nodes" in value && "elements" in value;
}

export function isThermalTruss2dResult(value: unknown): value is ThermalTruss2dResult {
  return typeof value === "object" && value !== null && "max_temperature_delta" in value && "max_axial_force" in value && "nodes" in value && "elements" in value && Array.isArray((value as ThermalTruss2dResult).nodes) && !(value as ThermalTruss2dResult).nodes.some((node) => "z" in node);
}

export function isThermalTruss3dResult(value: unknown): value is ThermalTruss3dResult {
  return typeof value === "object" && value !== null && "max_temperature_delta" in value && "max_axial_force" in value && "nodes" in value && "elements" in value && Array.isArray((value as ThermalTruss3dResult).nodes) && (value as ThermalTruss3dResult).nodes.some((node) => "z" in node);
}

export function isTruss3dResult(value: unknown): value is Truss3dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && Array.isArray((value as Truss3dResult).nodes) && (value as Truss3dResult).nodes.some((node) => "z" in node);
}

export function isTrussResult(value: unknown): value is Truss2dResult {
  return typeof value === "object" && value !== null && "nodes" in value && "elements" in value && !("tip_displacement" in value) && Array.isArray((value as Truss2dResult).nodes) && !(value as Truss2dResult).nodes.some((node) => "z" in node);
}

export function isFrame2dResult(value: unknown): value is Frame2dResult {
  return typeof value === "object" && value !== null && "max_rotation" in value && "max_moment" in value && "nodes" in value && "elements" in value;
}

export function isBeam1dResult(value: unknown): value is Beam1dResult {
  return typeof value === "object" && value !== null && "max_rotation" in value && "max_moment" in value && "nodes" in value && "elements" in value && Array.isArray((value as Beam1dResult).nodes) && !(value as Beam1dResult).nodes.some((node) => "y" in node);
}

export function isSpring1dResult(value: unknown): value is Spring1dResult {
  return typeof value === "object" && value !== null && "max_force" in value && "nodes" in value && "elements" in value && Array.isArray((value as Spring1dResult).nodes);
}

export function isSpring2dResult(value: unknown): value is Spring2dResult {
  return typeof value === "object" && value !== null && "max_force" in value && "nodes" in value && "elements" in value && Array.isArray((value as Spring2dResult).nodes) && (value as Spring2dResult).nodes.some((node) => "y" in node);
}

export function isSpring3dResult(value: unknown): value is Spring3dResult {
  return typeof value === "object" && value !== null && "max_force" in value && "nodes" in value && "elements" in value && Array.isArray((value as Spring3dResult).nodes) && (value as Spring3dResult).nodes.some((node) => "z" in node);
}

export function isTorsion1dResult(value: unknown): value is Torsion1dResult {
  return typeof value === "object" && value !== null && "max_torque" in value && "max_rotation" in value && "nodes" in value && "elements" in value && Array.isArray((value as Torsion1dResult).nodes);
}

export function serializeHeatPlaneResultCsv(
  result: HeatPlaneTriangle2dResult | HeatPlaneQuad2dResult,
) {
  const lines: string[] = [];
  lines.push("nodes");
  lines.push(toCsvRow(["index", "id", "x", "y", "temperature", "heat_load"]));
  result.nodes.forEach((node) =>
    lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.temperature, node.heat_load])),
  );
  lines.push("");
  lines.push("elements");
  lines.push(
    toCsvRow([
      "index",
      "id",
      "node_i",
      "node_j",
      "node_k",
      "node_l",
      "area",
      "average_temperature",
      "temperature_gradient_x",
      "temperature_gradient_y",
      "heat_flux_x",
      "heat_flux_y",
      "heat_flux_magnitude",
    ]),
  );
  result.elements.forEach((element) =>
    lines.push(
      toCsvRow([
        element.index,
        element.id,
        element.node_i,
        element.node_j,
        element.node_k,
        "node_l" in element && typeof element.node_l === "number" ? element.node_l : "",
        element.area,
        element.average_temperature,
        element.temperature_gradient_x,
        element.temperature_gradient_y,
        element.heat_flux_x,
        element.heat_flux_y,
        element.heat_flux_magnitude,
      ]),
    ),
  );
  return lines.join("\n");
}

export function serializePlaneResultCsv(
  result:
    | PlaneTriangle2dResult
    | PlaneQuad2dResult
    | ThermalPlaneTriangle2dResult
    | ThermalPlaneQuad2dResult,
) {
  const lines: string[] = [];
  lines.push("nodes");
  lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "displacement_magnitude"]));
  result.nodes.forEach((node) =>
    lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy, node.displacement_magnitude])),
  );
  lines.push("");
  lines.push("elements");
  lines.push(
    toCsvRow([
      "index",
      "id",
      "node_i",
      "node_j",
      "node_k",
      "area",
      "total_strain_x",
      "total_strain_y",
      "gamma_xy",
      "stress_x",
      "stress_y",
      "tau_xy",
      "principal_stress_1",
      "principal_stress_2",
      "max_in_plane_shear",
      "von_mises",
    ]),
  );
  result.elements.forEach((element) =>
    lines.push(
      toCsvRow([
        element.index,
        element.id,
        element.node_i,
        element.node_j,
        element.node_k,
        element.area,
        "strain_x" in element ? element.strain_x : element.total_strain_x,
        "strain_y" in element ? element.strain_y : element.total_strain_y,
        element.gamma_xy,
        element.stress_x,
        element.stress_y,
        element.tau_xy,
        element.principal_stress_1,
        element.principal_stress_2,
        element.max_in_plane_shear,
        element.von_mises,
      ]),
    ),
  );
  return lines.join("\n");
}
