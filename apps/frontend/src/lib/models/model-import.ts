import {
  parseAxialBarV1,
  parseBeam1dV1,
  parseHeatBar1dV1,
  parseSpring1dV1,
  parseSpring2dV1,
  parseSpring3dV1,
  parseThermalBar1dV1,
  parseThermalBeam1dV1,
  parseTorsion1dV1,
} from "@/lib/models/model-import-line";
import {
  parseElectrostaticPlaneQuad2dV1,
  parseElectrostaticPlaneTriangle2dV1,
  parseFrame2dV1,
  parseHeatPlaneQuad2dV1,
  parseHeatPlaneTriangle2dV1,
  parsePlaneQuad2dV1,
  parsePlaneTriangle2dV1,
  parseThermalFrame2dV1,
  parseThermalPlaneQuad2dV1,
  parseThermalPlaneTriangle2dV1,
  parseThermalTruss2dV1,
  parseTruss2dV1,
} from "@/lib/models/model-import-planar";
import {
  parseThermalTruss3dV1,
  parseTruss3dV1,
} from "@/lib/models/model-import-spatial";
export type * from "@/lib/models/model-import-types";
import type { ImportedModel } from "@/lib/models/model-import-types";
import { assertSupportedVersion } from "@/lib/models/model-import-utils";

export function parsePlaygroundModel(text: string): ImportedModel {
  const raw = JSON.parse(text) as Record<string, unknown>;
  assertSupportedVersion(raw);

  const kind =
    typeof raw.kind === "string"
      ? raw.kind
      : Array.isArray(raw.nodes) || Array.isArray(raw.elements)
        ? "truss_2d"
        : "axial_bar_1d";

  switch (kind) {
    case "electrostatic_plane_triangle_2d":
      return parseElectrostaticPlaneTriangle2dV1(raw);
    case "plane_triangle_2d":
      return parsePlaneTriangle2dV1(raw);
    case "heat_plane_triangle_2d":
      return parseHeatPlaneTriangle2dV1(raw);
    case "thermal_plane_triangle_2d":
      return parseThermalPlaneTriangle2dV1(raw);
    case "electrostatic_plane_quad_2d":
      return parseElectrostaticPlaneQuad2dV1(raw);
    case "plane_quad_2d":
      return parsePlaneQuad2dV1(raw);
    case "heat_plane_quad_2d":
      return parseHeatPlaneQuad2dV1(raw);
    case "thermal_plane_quad_2d":
      return parseThermalPlaneQuad2dV1(raw);
    case "truss_3d":
      return parseTruss3dV1(raw);
    case "thermal_truss_3d":
      return parseThermalTruss3dV1(raw);
    case "frame_2d":
      return parseFrame2dV1(raw);
    case "thermal_frame_2d":
      return parseThermalFrame2dV1(raw);
    case "beam_1d":
      return parseBeam1dV1(raw);
    case "thermal_beam_1d":
      return parseThermalBeam1dV1(raw);
    case "torsion_1d":
      return parseTorsion1dV1(raw);
    case "spring_1d":
      return parseSpring1dV1(raw);
    case "thermal_bar_1d":
      return parseThermalBar1dV1(raw);
    case "heat_bar_1d":
      return parseHeatBar1dV1(raw);
    case "thermal_truss_2d":
      return parseThermalTruss2dV1(raw);
    case "spring_2d":
      return parseSpring2dV1(raw);
    case "spring_3d":
      return parseSpring3dV1(raw);
    case "truss_2d":
      return parseTruss2dV1(raw);
    default:
      return parseAxialBarV1(raw);
  }
}
