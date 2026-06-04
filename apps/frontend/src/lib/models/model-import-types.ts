import type {
  Beam1dJobInput,
  Frame2dJobInput,
  HeatBar1dJobInput,
  HeatPlaneQuad2dJobInput,
  HeatPlaneTriangle2dJobInput,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  Spring1dJobInput,
  Spring2dJobInput,
  Spring3dJobInput,
  ThermalBar1dJobInput,
  ThermalBeam1dJobInput,
  ThermalFrame2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ThermalTruss2dJobInput,
  ThermalTruss3dJobInput,
  Torsion1dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
} from "@/lib/api";

export type ImportedAxialBarModel = {
  kind: "axial_bar_1d";
  name: string;
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  material: string;
  youngsModulusGpa: number;
};

export type ImportedThermalBar1dModel = { kind: "thermal_bar_1d"; name: string; model: ThermalBar1dJobInput };
export type ImportedHeatBar1dModel = { kind: "heat_bar_1d"; name: string; model: HeatBar1dJobInput };
export type ImportedHeatPlaneTriangle2dModel = { kind: "heat_plane_triangle_2d"; name: string; model: HeatPlaneTriangle2dJobInput };
export type ImportedHeatPlaneQuad2dModel = { kind: "heat_plane_quad_2d"; name: string; model: HeatPlaneQuad2dJobInput };
export type ImportedThermalBeam1dModel = { kind: "thermal_beam_1d"; name: string; material: string; youngsModulusGpa: number; model: ThermalBeam1dJobInput };
export type ImportedThermalFrame2dModel = { kind: "thermal_frame_2d"; name: string; material: string; youngsModulusGpa: number; model: ThermalFrame2dJobInput };
export type ImportedThermalTruss2dModel = { kind: "thermal_truss_2d"; name: string; material: string; youngsModulusGpa: number; model: ThermalTruss2dJobInput };
export type ImportedTruss2dModel = { kind: "truss_2d"; name: string; material: string; youngsModulusGpa: number; model: Truss2dJobInput };
export type ImportedPlaneTriangle2dModel = { kind: "plane_triangle_2d"; name: string; material: string; youngsModulusGpa: number; model: PlaneTriangle2dJobInput };
export type ImportedThermalPlaneTriangle2dModel = { kind: "thermal_plane_triangle_2d"; name: string; material: string; youngsModulusGpa: number; model: ThermalPlaneTriangle2dJobInput };
export type ImportedPlaneQuad2dModel = { kind: "plane_quad_2d"; name: string; material: string; youngsModulusGpa: number; model: PlaneQuad2dJobInput };
export type ImportedThermalPlaneQuad2dModel = { kind: "thermal_plane_quad_2d"; name: string; material: string; youngsModulusGpa: number; model: ThermalPlaneQuad2dJobInput };
export type ImportedTruss3dModel = { kind: "truss_3d"; name: string; material: string; youngsModulusGpa: number; model: Truss3dJobInput };
export type ImportedThermalTruss3dModel = { kind: "thermal_truss_3d"; name: string; material: string; youngsModulusGpa: number; model: ThermalTruss3dJobInput };
export type ImportedFrame2dModel = { kind: "frame_2d"; name: string; material: string; youngsModulusGpa: number; model: Frame2dJobInput };
export type ImportedBeam1dModel = { kind: "beam_1d"; name: string; material: string; youngsModulusGpa: number; model: Beam1dJobInput };
export type ImportedTorsion1dModel = { kind: "torsion_1d"; name: string; model: Torsion1dJobInput };
export type ImportedSpring1dModel = { kind: "spring_1d"; name: string; model: Spring1dJobInput };
export type ImportedSpring2dModel = { kind: "spring_2d"; name: string; model: Spring2dJobInput };
export type ImportedSpring3dModel = { kind: "spring_3d"; name: string; model: Spring3dJobInput };

export type ImportedModel =
  | ImportedAxialBarModel
  | ImportedHeatBar1dModel
  | ImportedHeatPlaneTriangle2dModel
  | ImportedHeatPlaneQuad2dModel
  | ImportedThermalBar1dModel
  | ImportedThermalBeam1dModel
  | ImportedThermalFrame2dModel
  | ImportedThermalTruss2dModel
  | ImportedSpring1dModel
  | ImportedSpring2dModel
  | ImportedSpring3dModel
  | ImportedTruss2dModel
  | ImportedPlaneTriangle2dModel
  | ImportedThermalPlaneTriangle2dModel
  | ImportedPlaneQuad2dModel
  | ImportedThermalPlaneQuad2dModel
  | ImportedTruss3dModel
  | ImportedThermalTruss3dModel
  | ImportedFrame2dModel
  | ImportedBeam1dModel
  | ImportedTorsion1dModel;
