import type {
  HeatBar1dJobInput,
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalBar1dJobInput,
  ThermalBeam1dJobInput,
  ThermalFrame2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ThermalTruss2dJobInput,
  ThermalTruss3dJobInput,
} from "@/lib/api";
import type { StudyKind } from "@/lib/models/modeler-types";

export function classifyStudyDomain(kind: StudyKind): "mechanical" | "thermal" | "thermo_mechanical" {
  switch (kind) {
    case "heat_bar_1d":
    case "heat_plane_triangle_2d":
    case "heat_plane_quad_2d":
      return "thermal";
    case "thermal_bar_1d":
    case "thermal_beam_1d":
    case "thermal_frame_2d":
    case "thermal_truss_2d":
    case "thermal_truss_3d":
    case "thermal_plane_triangle_2d":
    case "thermal_plane_quad_2d":
      return "thermo_mechanical";
    default:
      return "mechanical";
  }
}

export function classifyStudyFamily(kind: StudyKind): "axial_and_springs" | "beams_and_frames" | "trusses" | "planes" {
  switch (kind) {
    case "axial_bar_1d":
    case "heat_bar_1d":
    case "thermal_bar_1d":
    case "spring_1d":
    case "spring_2d":
    case "spring_3d":
      return "axial_and_springs";
    case "beam_1d":
    case "thermal_beam_1d":
    case "torsion_1d":
    case "frame_2d":
    case "thermal_frame_2d":
      return "beams_and_frames";
    case "truss_2d":
    case "truss_3d":
    case "thermal_truss_2d":
    case "thermal_truss_3d":
      return "trusses";
    default:
      return "planes";
  }
}

export function buildAnalysisMetadata(
  kind: StudyKind,
  payload: {
    heatBar?: HeatBar1dJobInput;
    thermalBar?: ThermalBar1dJobInput;
    thermalBeam?: ThermalBeam1dJobInput;
    thermalFrame?: ThermalFrame2dJobInput;
    thermalTruss?: ThermalTruss2dJobInput;
    thermalTruss3d?: ThermalTruss3dJobInput;
    plane?:
      | PlaneTriangle2dJobInput
      | PlaneQuad2dJobInput
      | ThermalPlaneTriangle2dJobInput
      | ThermalPlaneQuad2dJobInput
      | HeatPlaneTriangle2dJobInput
      | HeatPlaneQuad2dJobInput;
  },
) {
  const base = {
    domain: classifyStudyDomain(kind),
    family: classifyStudyFamily(kind),
  };

  if (kind === "heat_bar_1d" && payload.heatBar) {
    return {
      ...base,
      thermal_intent: ["conduction_field", ...(payload.heatBar.nodes.some((node) => Math.abs(node.heat_load ?? 0) > 0) ? ["heat_source_field"] : [])],
      thermal_boundary: {
        prescribed_temperature_nodes: payload.heatBar.nodes.filter((node) => node.fix_temperature).length,
        source_nodes: payload.heatBar.nodes.filter((node) => Math.abs(node.heat_load ?? 0) > 0).length,
      },
    };
  }

  if ((kind === "heat_plane_triangle_2d" || kind === "heat_plane_quad_2d") && payload.plane) {
    const nodes = payload.plane.nodes as HeatPlaneTriangle2dJobInput["nodes"] | HeatPlaneQuad2dJobInput["nodes"];
    return {
      ...base,
      thermal_intent: ["conduction_field", ...(nodes.some((node) => Math.abs(node.heat_load ?? 0) > 0) ? ["heat_source_field"] : [])],
      thermal_boundary: {
        prescribed_temperature_nodes: nodes.filter((node) => node.fix_temperature).length,
        source_nodes: nodes.filter((node) => Math.abs(node.heat_load ?? 0) > 0).length,
      },
    };
  }

  if (kind === "thermal_bar_1d" && payload.thermalBar) {
    return {
      ...base,
      thermal_intent: ["nodal_temperature_rise", "restrained_bar_response"],
      thermal_boundary: {
        heated_nodes: payload.thermalBar.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length,
        restrained_supports: payload.thermalBar.nodes.filter((node) => node.fix_x).length,
      },
    };
  }

  if (kind === "thermal_beam_1d" && payload.thermalBeam) {
    return {
      ...base,
      thermal_intent: ["member_temperature_gradient", "beam_thermal_response"],
      thermal_boundary: {
        gradient_members: payload.thermalBeam.elements.filter((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0).length,
        restrained_supports: payload.thermalBeam.nodes.filter((node) => node.fix_y || node.fix_rz).length,
      },
    };
  }

  if (kind === "thermal_frame_2d" && payload.thermalFrame) {
    return {
      ...base,
      thermal_intent: [
        ...(payload.thermalFrame.nodes.some((node) => Math.abs(node.temperature_delta ?? 0) > 0) ? ["nodal_temperature_rise"] : []),
        ...(payload.thermalFrame.elements.some((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0) ? ["member_temperature_gradient"] : []),
        "frame_thermal_response",
      ],
      thermal_boundary: {
        heated_nodes: payload.thermalFrame.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length,
        gradient_members: payload.thermalFrame.elements.filter((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0).length,
        restrained_supports: payload.thermalFrame.nodes.filter((node) => node.fix_x || node.fix_y || node.fix_rz).length,
      },
    };
  }

  if (kind === "thermal_truss_2d" && payload.thermalTruss) {
    return {
      ...base,
      thermal_intent: ["nodal_temperature_rise", "truss_thermal_response"],
      thermal_boundary: {
        heated_nodes: payload.thermalTruss.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length,
        restrained_supports: payload.thermalTruss.nodes.filter((node) => node.fix_x || node.fix_y).length,
      },
    };
  }

  if (kind === "thermal_truss_3d" && payload.thermalTruss3d) {
    return {
      ...base,
      thermal_intent: ["nodal_temperature_rise", "truss_thermal_response"],
      thermal_boundary: {
        heated_nodes: payload.thermalTruss3d.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length,
        restrained_supports: payload.thermalTruss3d.nodes.filter((node) => node.fix_x || node.fix_y || node.fix_z).length,
      },
    };
  }

  if ((kind === "thermal_plane_triangle_2d" || kind === "thermal_plane_quad_2d") && payload.plane) {
    const nodes = payload.plane.nodes as ThermalPlaneTriangle2dJobInput["nodes"] | ThermalPlaneQuad2dJobInput["nodes"];
    return {
      ...base,
      thermal_intent: ["nodal_temperature_rise", "thermoelastic_plane_response"],
      thermal_boundary: {
        heated_nodes: nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length,
        restrained_supports: nodes.filter((node) => node.fix_x || node.fix_y).length,
      },
    };
  }

  return base;
}
