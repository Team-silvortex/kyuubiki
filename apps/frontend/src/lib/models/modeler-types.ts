export type ParametricTrussConfig = {
  bays: number;
  span: number;
  height: number;
  area: number;
  youngsModulusGpa: number;
  loadY: number;
};

export type ParametricPanelConfig = {
  width: number;
  height: number;
  divisionsX: number;
  divisionsY: number;
  thickness: number;
  youngsModulusGpa: number;
  poissonRatio: number;
  loadY: number;
};

export type ParametricPanelElementKind = "triangle" | "quad";

export const MODEL_SCHEMA_VERSION = "kyuubiki.model/v1";

export type StudyKind =
  | "axial_bar_1d"
  | "heat_bar_1d"
  | "electrostatic_plane_triangle_2d"
  | "electrostatic_plane_quad_2d"
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
