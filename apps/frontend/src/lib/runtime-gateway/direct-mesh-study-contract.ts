import {
  resolveAxialBarJobInput,
  resolveBeam1dJobInput,
  resolveElectrostaticPlaneQuad2dJobInput,
  resolveElectrostaticPlaneTriangle2dJobInput,
  resolveFrame2dJobInput,
  resolveHeatBar1dJobInput,
  resolveHeatPlaneQuad2dJobInput,
  resolveHeatPlaneTriangle2dJobInput,
  resolvePlaneQuad2dJobInput,
  resolvePlaneTriangle2dJobInput,
  resolveSpring1dJobInput,
  resolveSpring2dJobInput,
  resolveSpring3dJobInput,
  resolveThermalBar1dJobInput,
  resolveThermalBeam1dJobInput,
  resolveThermalFrame2dJobInput,
  resolveThermalPlaneQuad2dJobInput,
  resolveThermalPlaneTriangle2dJobInput,
  resolveThermalTruss2dJobInput,
  resolveThermalTruss3dJobInput,
  resolveTorsion1dJobInput,
  resolveTruss2dJobInput,
  resolveTruss3dJobInput,
  type AxialBarJobInput,
  type Beam1dJobInput,
  type ElectrostaticPlaneQuad2dJobInput,
  type ElectrostaticPlaneTriangle2dJobInput,
  type Frame2dJobInput,
  type HeatBar1dJobInput,
  type HeatPlaneQuad2dJobInput,
  type HeatPlaneTriangle2dJobInput,
  type PlaneQuad2dJobInput,
  type PlaneTriangle2dJobInput,
  type Spring1dJobInput,
  type Spring2dJobInput,
  type Spring3dJobInput,
  type ThermalBar1dJobInput,
  type ThermalBeam1dJobInput,
  type ThermalFrame2dJobInput,
  type ThermalPlaneQuad2dJobInput,
  type ThermalPlaneTriangle2dJobInput,
  type ThermalTruss2dJobInput,
  type ThermalTruss3dJobInput,
  type Torsion1dJobInput,
  type Truss2dJobInput,
  type Truss3dJobInput,
} from "@/lib/api";
import type { DirectMeshRpcMethod } from "@/lib/direct-mesh/rpc";

export type DirectMeshStudyKind =
  | "axial_bar_1d"
  | "thermal_bar_1d"
  | "heat_bar_1d"
  | "electrostatic_plane_triangle_2d"
  | "electrostatic_plane_quad_2d"
  | "heat_plane_triangle_2d"
  | "heat_plane_quad_2d"
  | "thermal_truss_2d"
  | "thermal_truss_3d"
  | "spring_1d"
  | "spring_2d"
  | "spring_3d"
  | "beam_1d"
  | "thermal_beam_1d"
  | "thermal_frame_2d"
  | "torsion_1d"
  | "truss_2d"
  | "truss_3d"
  | "plane_triangle_2d"
  | "thermal_plane_triangle_2d"
  | "plane_quad_2d"
  | "thermal_plane_quad_2d"
  | "frame_2d";

export type DirectMeshSolveBody = {
  endpoints?: string[];
  selection_mode?: "first_reachable" | "healthiest";
  study_kind: DirectMeshStudyKind;
  input: Record<string, unknown>;
};

export function directMeshMethodForStudyKind(
  kind: DirectMeshStudyKind,
): Exclude<DirectMeshRpcMethod, "ping" | "describe_agent"> {
  switch (kind) {
    case "axial_bar_1d": return "solve_bar_1d";
    case "thermal_bar_1d": return "solve_thermal_bar_1d";
    case "heat_bar_1d": return "solve_heat_bar_1d";
    case "electrostatic_plane_triangle_2d": return "solve_electrostatic_plane_triangle_2d";
    case "electrostatic_plane_quad_2d": return "solve_electrostatic_plane_quad_2d";
    case "heat_plane_triangle_2d": return "solve_heat_plane_triangle_2d";
    case "heat_plane_quad_2d": return "solve_heat_plane_quad_2d";
    case "thermal_truss_2d": return "solve_thermal_truss_2d";
    case "thermal_truss_3d": return "solve_thermal_truss_3d";
    case "spring_1d": return "solve_spring_1d";
    case "spring_2d": return "solve_spring_2d";
    case "spring_3d": return "solve_spring_3d";
    case "beam_1d": return "solve_beam_1d";
    case "thermal_beam_1d": return "solve_thermal_beam_1d";
    case "thermal_frame_2d": return "solve_thermal_frame_2d";
    case "torsion_1d": return "solve_torsion_1d";
    case "truss_2d": return "solve_truss_2d";
    case "truss_3d": return "solve_truss_3d";
    case "plane_triangle_2d": return "solve_plane_triangle_2d";
    case "thermal_plane_triangle_2d": return "solve_thermal_plane_triangle_2d";
    case "plane_quad_2d": return "solve_plane_quad_2d";
    case "thermal_plane_quad_2d": return "solve_thermal_plane_quad_2d";
    case "frame_2d": return "solve_frame_2d";
  }
}

export function normalizeDirectMeshStudyInput(kind: DirectMeshStudyKind, input: Record<string, unknown>) {
  switch (kind) {
    case "axial_bar_1d": return resolveAxialBarJobInput(input as AxialBarJobInput);
    case "thermal_bar_1d": return resolveThermalBar1dJobInput(input as ThermalBar1dJobInput);
    case "heat_bar_1d": return resolveHeatBar1dJobInput(input as HeatBar1dJobInput);
    case "electrostatic_plane_triangle_2d": return resolveElectrostaticPlaneTriangle2dJobInput(input as ElectrostaticPlaneTriangle2dJobInput);
    case "electrostatic_plane_quad_2d": return resolveElectrostaticPlaneQuad2dJobInput(input as ElectrostaticPlaneQuad2dJobInput);
    case "heat_plane_triangle_2d": return resolveHeatPlaneTriangle2dJobInput(input as HeatPlaneTriangle2dJobInput);
    case "heat_plane_quad_2d": return resolveHeatPlaneQuad2dJobInput(input as HeatPlaneQuad2dJobInput);
    case "thermal_truss_2d": return resolveThermalTruss2dJobInput(input as ThermalTruss2dJobInput);
    case "thermal_truss_3d": return resolveThermalTruss3dJobInput(input as ThermalTruss3dJobInput);
    case "spring_1d": return resolveSpring1dJobInput(input as Spring1dJobInput);
    case "spring_2d": return resolveSpring2dJobInput(input as Spring2dJobInput);
    case "spring_3d": return resolveSpring3dJobInput(input as Spring3dJobInput);
    case "beam_1d": return resolveBeam1dJobInput(input as Beam1dJobInput);
    case "thermal_beam_1d": return resolveThermalBeam1dJobInput(input as ThermalBeam1dJobInput);
    case "thermal_frame_2d": return resolveThermalFrame2dJobInput(input as ThermalFrame2dJobInput);
    case "torsion_1d": return resolveTorsion1dJobInput(input as Torsion1dJobInput);
    case "truss_2d": return resolveTruss2dJobInput(input as Truss2dJobInput);
    case "truss_3d": return resolveTruss3dJobInput(input as Truss3dJobInput);
    case "plane_triangle_2d": return resolvePlaneTriangle2dJobInput(input as PlaneTriangle2dJobInput);
    case "thermal_plane_triangle_2d": return resolveThermalPlaneTriangle2dJobInput(input as ThermalPlaneTriangle2dJobInput);
    case "plane_quad_2d": return resolvePlaneQuad2dJobInput(input as PlaneQuad2dJobInput);
    case "thermal_plane_quad_2d": return resolveThermalPlaneQuad2dJobInput(input as ThermalPlaneQuad2dJobInput);
    case "frame_2d": return resolveFrame2dJobInput(input as Frame2dJobInput);
  }
}
