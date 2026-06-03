import type {
  AxialBarResult,
  Beam1dResult,
  Frame2dResult,
  HeatBar1dResult,
  HeatPlaneQuad2dResult,
  HeatPlaneTriangle2dResult,
  JobEnvelope,
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
import {
  isAxialResult,
  isBeam1dResult,
  isFrame2dResult,
  isHeatBar1dResult,
  isSpring1dResult,
  isSpring2dResult,
  isSpring3dResult,
  isThermalBar1dResult,
  isThermalBeam1dResult,
  isThermalFrame2dResult,
  isThermalTruss2dResult,
  isThermalTruss3dResult,
  isTorsion1dResult,
  isTruss3dResult,
  isTrussResult,
  serializeHeatPlaneResultCsv,
  serializePlaneResultCsv,
  toCsvRow,
} from "./result-csv-shared";

type StudyKind =
  | "axial_bar_1d"
  | "heat_bar_1d"
  | "heat_plane_triangle_2d"
  | "heat_plane_quad_2d"
  | "thermal_bar_1d"
  | "thermal_beam_1d"
  | "thermal_frame_2d"
  | "thermal_truss_2d"
  | "thermal_truss_3d"
  | "thermal_plane_triangle_2d"
  | "thermal_plane_quad_2d"
  | "spring_1d"
  | "spring_2d"
  | "spring_3d"
  | "beam_1d"
  | "torsion_1d"
  | "truss_2d"
  | "truss_3d"
  | "plane_triangle_2d"
  | "plane_quad_2d"
  | "frame_2d";

export function serializeResultCsv(
  studyKind: StudyKind,
  job: JobEnvelope["job"] | null,
  result:
    | AxialBarResult
    | HeatBar1dResult
    | HeatPlaneTriangle2dResult
    | HeatPlaneQuad2dResult
    | ThermalBar1dResult
    | ThermalBeam1dResult
    | ThermalFrame2dResult
    | ThermalTruss2dResult
    | ThermalTruss3dResult
    | ThermalPlaneTriangle2dResult
    | ThermalPlaneQuad2dResult
    | Spring1dResult
    | Spring2dResult
    | Spring3dResult
    | Beam1dResult
    | Torsion1dResult
    | Truss2dResult
    | Truss3dResult
    | PlaneTriangle2dResult
    | PlaneQuad2dResult
    | Frame2dResult
    | null,
) {
  if (!result) return "";

  const lines: string[] = [];
  lines.push("meta");
  lines.push(toCsvRow(["study_kind", studyKind]));
  lines.push(toCsvRow(["job_id", job?.job_id]));
  lines.push(toCsvRow(["status", job?.status]));
  lines.push(toCsvRow(["worker_id", job?.worker_id]));
  lines.push("");

  if (isAxialResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "x", "displacement"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.x, node.displacement])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "x1", "x2", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.x1, element.x2, element.strain, element.stress, element.axial_force])),
    );
    return lines.join("\n");
  }

  if (isThermalBar1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "ux", "temperature_delta"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.ux, node.temperature_delta])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature_delta",
        "thermal_strain",
        "mechanical_strain",
        "total_strain",
        "stress",
        "axial_force",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.average_temperature_delta,
          element.thermal_strain,
          element.mechanical_strain,
          element.total_strain,
          element.stress,
          element.axial_force,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isHeatBar1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "temperature", "heat_load"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.temperature, node.heat_load])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature",
        "temperature_gradient",
        "heat_flux",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.average_temperature,
          element.temperature_gradient,
          element.heat_flux,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (studyKind === "heat_plane_triangle_2d" || studyKind === "heat_plane_quad_2d") {
    const heatPlaneResult = result as HeatPlaneTriangle2dResult | HeatPlaneQuad2dResult;
    return `${lines.join("\n")}\n${serializeHeatPlaneResultCsv(heatPlaneResult)}`;
  }

  if (isThermalTruss3dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "z", "ux", "uy", "uz", "temperature_delta"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.z, node.ux, node.uy, node.uz, node.temperature_delta])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature_delta",
        "thermal_strain",
        "mechanical_strain",
        "total_strain",
        "stress",
        "axial_force",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.average_temperature_delta,
          element.thermal_strain,
          element.mechanical_strain,
          element.total_strain,
          element.stress,
          element.axial_force,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isTruss3dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "z", "ux", "uy", "uz"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.z, node.ux, node.uy, node.uz])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.strain, element.stress, element.axial_force])),
    );
    return lines.join("\n");
  }

  if (isThermalTruss2dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "temperature_delta"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy, node.temperature_delta])));
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature_delta",
        "thermal_strain",
        "mechanical_strain",
        "total_strain",
        "stress",
        "axial_force",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.average_temperature_delta, element.thermal_strain, element.mechanical_strain, element.total_strain, element.stress, element.axial_force]),
      ),
    );
    return lines.join("\n");
  }

  if (isTrussResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "strain", "stress", "axial_force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.strain, element.stress, element.axial_force])),
    );
    return lines.join("\n");
  }

  if (isFrame2dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "rz", "displacement_magnitude"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy, node.rz, node.displacement_magnitude])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "axial_force_i",
        "shear_force_i",
        "moment_i",
        "axial_force_j",
        "shear_force_j",
        "moment_j",
        "axial_stress",
        "max_bending_stress",
        "max_combined_stress",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.axial_force_i,
          element.shear_force_i,
          element.moment_i,
          element.axial_force_j,
          element.shear_force_j,
          element.moment_j,
          element.axial_stress,
          element.max_bending_stress,
          element.max_combined_stress,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isThermalFrame2dResult(result)) {
    const thermalFrameResult = result as ThermalFrame2dResult;
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy", "rz", "displacement_magnitude", "temperature_delta"]));
    thermalFrameResult.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy, node.rz, node.displacement_magnitude, node.temperature_delta])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "average_temperature_delta",
        "thermal_strain",
        "mechanical_strain",
        "total_strain",
        "temperature_gradient_y",
        "thermal_curvature",
        "axial_force_i",
        "shear_force_i",
        "moment_i",
        "axial_force_j",
        "shear_force_j",
        "moment_j",
        "axial_stress",
        "max_bending_stress",
        "max_combined_stress",
      ]),
    );
    thermalFrameResult.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.average_temperature_delta,
          element.thermal_strain,
          element.mechanical_strain,
          element.total_strain,
          element.temperature_gradient_y,
          element.thermal_curvature,
          element.axial_force_i,
          element.shear_force_i,
          element.moment_i,
          element.axial_force_j,
          element.shear_force_j,
          element.moment_j,
          element.axial_stress,
          element.max_bending_stress,
          element.max_combined_stress,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isBeam1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "uy", "rz", "displacement_magnitude"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.uy, node.rz, node.displacement_magnitude])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "shear_force_i",
        "moment_i",
        "shear_force_j",
        "moment_j",
        "max_bending_stress",
      ]),
    );
    result.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.shear_force_i,
          element.moment_i,
          element.shear_force_j,
          element.moment_j,
          element.max_bending_stress,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isThermalBeam1dResult(result)) {
    const thermalBeamResult = result as ThermalBeam1dResult;
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "uy", "rz", "displacement_magnitude"]));
    thermalBeamResult.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.uy, node.rz, node.displacement_magnitude])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(
      toCsvRow([
        "index",
        "id",
        "node_i",
        "node_j",
        "length",
        "temperature_gradient_y",
        "thermal_curvature",
        "shear_force_i",
        "moment_i",
        "shear_force_j",
        "moment_j",
        "max_bending_stress",
      ]),
    );
    thermalBeamResult.elements.forEach((element) =>
      lines.push(
        toCsvRow([
          element.index,
          element.id,
          element.node_i,
          element.node_j,
          element.length,
          element.temperature_gradient_y,
          element.thermal_curvature,
          element.shear_force_i,
          element.moment_i,
          element.shear_force_j,
          element.moment_j,
          element.max_bending_stress,
        ]),
      ),
    );
    return lines.join("\n");
  }

  if (isTorsion1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "rz"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.rz])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "twist", "torque", "shear_stress"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.twist, element.torque, element.shear_stress])),
    );
    return lines.join("\n");
  }

  if (isSpring1dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "ux"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.ux])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "extension", "force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.extension, element.force])),
    );
    return lines.join("\n");
  }

  if (isSpring2dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "ux", "uy"]));
    result.nodes.forEach((node) => lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.ux, node.uy])));
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "extension", "force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.extension, element.force])),
    );
    return lines.join("\n");
  }

  if (isSpring3dResult(result)) {
    lines.push("nodes");
    lines.push(toCsvRow(["index", "id", "x", "y", "z", "ux", "uy", "uz"]));
    result.nodes.forEach((node) =>
      lines.push(toCsvRow([node.index, node.id, node.x, node.y, node.z, node.ux, node.uy, node.uz])),
    );
    lines.push("");
    lines.push("elements");
    lines.push(toCsvRow(["index", "id", "node_i", "node_j", "length", "extension", "force"]));
    result.elements.forEach((element) =>
      lines.push(toCsvRow([element.index, element.id, element.node_i, element.node_j, element.length, element.extension, element.force])),
    );
    return lines.join("\n");
  }

  const plane = result as PlaneTriangle2dResult | PlaneQuad2dResult | ThermalPlaneTriangle2dResult | ThermalPlaneQuad2dResult;
  return `${lines.join("\n")}\n${serializePlaneResultCsv(plane)}`;
}
