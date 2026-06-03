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
import type { WorkbenchLanguage } from "@/components/workbench/workbench-copy";
import type { WorkflowSurfaceTab } from "@/components/workbench/workflow/workbench-workflow-types";

export type Language = WorkbenchLanguage;
export type Theme = "linen" | "marine" | "graphite";
export type SidebarSection = "study" | "model" | "workflow" | "library" | "system";
export type StudyKind =
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

export type PlaneStudyJobInput =
  | PlaneTriangle2dJobInput
  | PlaneQuad2dJobInput
  | ThermalPlaneTriangle2dJobInput
  | ThermalPlaneQuad2dJobInput;
export type HeatPlaneStudyJobInput = HeatPlaneTriangle2dJobInput | HeatPlaneQuad2dJobInput;
export type FrameStudyJobInput = Frame2dJobInput;
export type ThermalFrameStudyJobInput = ThermalFrame2dJobInput;
export type BeamStudyJobInput = Beam1dJobInput;
export type ThermalBeamStudyJobInput = ThermalBeam1dJobInput;
export type ThermalBarStudyJobInput = ThermalBar1dJobInput;
export type HeatBarStudyJobInput = HeatBar1dJobInput;
export type ThermalTruss2dStudyJobInput = ThermalTruss2dJobInput;
export type ThermalTruss3dStudyJobInput = ThermalTruss3dJobInput;
export type SpringStudyJobInput = Spring1dJobInput;
export type Spring2dStudyJobInput = Spring2dJobInput;
export type Spring3dStudyJobInput = Spring3dJobInput;

export type LineResultField =
  | "axial_stress"
  | "max_bending_stress"
  | "max_combined_stress"
  | "moment"
  | "shear_force"
  | "average_temperature_delta"
  | "temperature_gradient_y"
  | "thermal_curvature";
export type FrameResultField = Exclude<LineResultField, "shear_force">;
export type BeamResultField = Extract<
  LineResultField,
  "max_bending_stress" | "moment" | "shear_force" | "temperature_gradient_y" | "thermal_curvature"
>;
export type StudyPanelTab = "summary" | "controls";
export type ModelPanelTab = "tools" | "tree";
export type LibraryPanelTab = "jobs" | "results" | "models" | "projects" | "samples";
export type WorkflowPanelTab = WorkflowSurfaceTab;
export type ImmersiveToolTab = "node" | "props";
export type SystemDataTab = "jobs" | "results";
export type SystemPanelTab = "config" | "assistant" | "scripts" | "runtime" | "data";
export type AssistantMode = "local" | "llm";
export type SecurityEventWindow = "" | "1h" | "24h" | "7d" | "30d";

export const SECURITY_EVENT_WINDOW_MS: Record<Exclude<SecurityEventWindow, "">, number> = {
  "1h": 60 * 60 * 1_000,
  "24h": 24 * 60 * 60 * 1_000,
  "7d": 7 * 24 * 60 * 60 * 1_000,
  "30d": 30 * 24 * 60 * 60 * 1_000,
};
