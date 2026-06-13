"use client";

import type { WorkbenchStudyKind } from "@/lib/workbench/history";

export const WORKBENCH_STUDY_KINDS = [
  "axial_bar_1d",
  "heat_bar_1d",
  "electrostatic_plane_triangle_2d",
  "electrostatic_plane_quad_2d",
  "heat_plane_triangle_2d",
  "heat_plane_quad_2d",
  "thermal_bar_1d",
  "thermal_beam_1d",
  "thermal_frame_2d",
  "thermal_truss_2d",
  "thermal_truss_3d",
  "thermal_plane_triangle_2d",
  "thermal_plane_quad_2d",
  "spring_1d",
  "spring_2d",
  "spring_3d",
  "beam_1d",
  "torsion_1d",
  "truss_2d",
  "truss_3d",
  "plane_triangle_2d",
  "plane_quad_2d",
  "frame_2d",
] as const satisfies readonly WorkbenchStudyKind[];

type StudyKindResetFactoryArgs = {
  activeMaterial: string;
  setPlaneModel: (value: any) => void;
  setHeatBarModel: (value: any) => void;
  setHeatPlaneModel: (value: any) => void;
  setThermalBarModel: (value: any) => void;
  setThermalBeamModel: (value: any) => void;
  setThermalFrameModel: (value: any) => void;
  setThermalTrussModel: (value: any) => void;
  setThermalTruss3dModel: (value: any) => void;
  setSpringModel: (value: any) => void;
  setSpring2dModel: (value: any) => void;
  setSpring3dModel: (value: any) => void;
  setBeamModel: (value: any) => void;
  setTorsionModel: (value: any) => void;
  setFrameModel: (value: any) => void;
  setPlaneResultField: (value: any) => void;
  ensurePlaneModelMaterials: (model: any, materialValue: string) => any;
  ensureBeamModelMaterials: (model: any, materialValue: string) => any;
  ensureFrameModelMaterials: (model: any, materialValue: string) => any;
  defaultPlaneQuad: any;
  defaultElectrostaticPlaneQuad: any;
  defaultThermalPlaneQuad: any;
  defaultPlaneTriangle: any;
  defaultElectrostaticPlaneTriangle: any;
  defaultThermalPlaneTriangle: any;
  defaultHeatBar1d: any;
  defaultHeatPlaneQuad: any;
  defaultHeatPlaneTriangle: any;
  defaultThermalBar1d: any;
  defaultThermalBeam1d: any;
  defaultThermalFrame2d: any;
  defaultThermalTruss2d: any;
  defaultThermalTruss3d: any;
  defaultSpring1d: any;
  defaultSpring2d: any;
  defaultSpring3d: any;
  defaultBeam1d: any;
  defaultTorsion1d: any;
  defaultFrame2d: any;
};

type StudyKindSelectionArgs = {
  currentStudyKind: WorkbenchStudyKind;
  nextStudyKind: WorkbenchStudyKind;
  setStudyKind: (value: WorkbenchStudyKind) => void;
  resetHandlers: Partial<Record<WorkbenchStudyKind, () => void>>;
};

export function isWorkbenchStudyKind(value: unknown): value is WorkbenchStudyKind {
  return typeof value === "string" && WORKBENCH_STUDY_KINDS.includes(value as WorkbenchStudyKind);
}

export function createStudyKindResetHandlers({
  activeMaterial,
  setPlaneModel,
  setHeatBarModel,
  setHeatPlaneModel,
  setThermalBarModel,
  setThermalBeamModel,
  setThermalFrameModel,
  setThermalTrussModel,
  setThermalTruss3dModel,
  setSpringModel,
  setSpring2dModel,
  setSpring3dModel,
  setBeamModel,
  setTorsionModel,
  setFrameModel,
  setPlaneResultField,
  ensurePlaneModelMaterials,
  ensureBeamModelMaterials,
  ensureFrameModelMaterials,
  defaultPlaneQuad,
  defaultElectrostaticPlaneQuad,
  defaultThermalPlaneQuad,
  defaultPlaneTriangle,
  defaultElectrostaticPlaneTriangle,
  defaultThermalPlaneTriangle,
  defaultHeatBar1d,
  defaultHeatPlaneQuad,
  defaultHeatPlaneTriangle,
  defaultThermalBar1d,
  defaultThermalBeam1d,
  defaultThermalFrame2d,
  defaultThermalTruss2d,
  defaultThermalTruss3d,
  defaultSpring1d,
  defaultSpring2d,
  defaultSpring3d,
  defaultBeam1d,
  defaultTorsion1d,
  defaultFrame2d,
}: StudyKindResetFactoryArgs): Partial<Record<WorkbenchStudyKind, () => void>> {
  return {
    plane_quad_2d: () => setPlaneModel(ensurePlaneModelMaterials(defaultPlaneQuad, activeMaterial)),
    electrostatic_plane_quad_2d: () => setPlaneModel(defaultElectrostaticPlaneQuad),
    thermal_plane_quad_2d: () => setPlaneModel(ensurePlaneModelMaterials(defaultThermalPlaneQuad, activeMaterial)),
    plane_triangle_2d: () => setPlaneModel(ensurePlaneModelMaterials(defaultPlaneTriangle, activeMaterial)),
    electrostatic_plane_triangle_2d: () => setPlaneModel(defaultElectrostaticPlaneTriangle),
    thermal_plane_triangle_2d: () => setPlaneModel(ensurePlaneModelMaterials(defaultThermalPlaneTriangle, activeMaterial)),
    heat_bar_1d: () => setHeatBarModel(defaultHeatBar1d),
    heat_plane_quad_2d: () => {
      setHeatPlaneModel(defaultHeatPlaneQuad);
      setPlaneResultField("average_temperature");
    },
    heat_plane_triangle_2d: () => {
      setHeatPlaneModel(defaultHeatPlaneTriangle);
      setPlaneResultField("average_temperature");
    },
    thermal_bar_1d: () => setThermalBarModel(defaultThermalBar1d),
    thermal_beam_1d: () => setThermalBeamModel(ensureBeamModelMaterials(defaultThermalBeam1d, activeMaterial)),
    thermal_frame_2d: () => setThermalFrameModel(ensureFrameModelMaterials(defaultThermalFrame2d, activeMaterial)),
    thermal_truss_2d: () => setThermalTrussModel(defaultThermalTruss2d),
    thermal_truss_3d: () => setThermalTruss3dModel(defaultThermalTruss3d),
    spring_1d: () => setSpringModel(defaultSpring1d),
    spring_2d: () => setSpring2dModel(defaultSpring2d),
    spring_3d: () => setSpring3dModel(defaultSpring3d),
    beam_1d: () => setBeamModel(ensureBeamModelMaterials(defaultBeam1d, activeMaterial)),
    torsion_1d: () => setTorsionModel(defaultTorsion1d),
    frame_2d: () => setFrameModel(ensureFrameModelMaterials(defaultFrame2d, activeMaterial)),
  };
}

export function applyStudyKindSelection({
  currentStudyKind,
  nextStudyKind,
  setStudyKind,
  resetHandlers,
}: StudyKindSelectionArgs) {
  if (currentStudyKind !== nextStudyKind) {
    resetHandlers[nextStudyKind]?.();
  }
  setStudyKind(nextStudyKind);
}
