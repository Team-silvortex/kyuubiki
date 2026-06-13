"use client";

import { createMaterialDefinition } from "@/lib/materials";
import { parsePlaygroundModel } from "@/lib/models";
import {
  ensureFrameModelMaterials,
  ensurePlaneModelMaterials,
  ensureTruss3dModelMaterials,
  ensureTrussModelMaterials,
} from "@/lib/workbench/material-commands";

type ImportedWorkbenchModel = ReturnType<typeof parsePlaygroundModel>;

type ModelLoadEffects = {
  setLoadedModelName: (value: string) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  setModelVersions: (value: any[]) => void;
  setStudyKind: (value: any) => void;
  setAxialForm: (value: any) => void;
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
  setTrussModel: (value: any) => void;
  setTruss3dModel: (value: any) => void;
  setPlaneModel: (value: any) => void;
  setFrameModel: (value: any) => void;
  setBeamModel: (value: any) => void;
  setTorsionModel: (value: any) => void;
  setPlaneResultField: (value: any) => void;
  setParametric: (updater: (current: any) => any) => void;
  setActiveMaterial: (value: string) => void;
};

function ensureBeamModelMaterials<T extends { materials?: Array<{ id: string }>; elements: Array<{ material_id?: string | undefined }> }>(
  model: T,
  materialValue: string,
): T {
  const existingMaterials = model.materials?.length
    ? model.materials
    : [createMaterialDefinition(materialValue, 1, { id: "mat-1" })];
  const defaultMaterialId = existingMaterials[0]?.id ?? "mat-1";
  return {
    ...model,
    materials: existingMaterials,
    elements: model.elements.map((element) => ({
      ...element,
      material_id: element.material_id ?? defaultMaterialId,
    })),
  };
}

export function applyImportedWorkbenchModel(imported: ImportedWorkbenchModel, effects: ModelLoadEffects) {
  effects.setLoadedModelName(imported.name);
  effects.setSelectedModelId(null);
  effects.setSelectedVersionId(null);
  effects.setModelVersions([]);

  if (imported.kind === "truss_2d") {
    effects.setStudyKind("truss_2d");
    effects.setTrussModel(ensureTrussModelMaterials(imported.model, imported.material));
    effects.setActiveMaterial(imported.material);
    effects.setParametric((current) => ({
      ...current,
      youngsModulusGpa: imported.youngsModulusGpa,
    }));
    return;
  }

  if (imported.kind === "spring_1d") {
    effects.setStudyKind("spring_1d");
    effects.setSpringModel(imported.model);
    return;
  }

  if (imported.kind === "heat_bar_1d") {
    effects.setStudyKind("heat_bar_1d");
    effects.setHeatBarModel(imported.model);
    return;
  }

  if (imported.kind === "heat_plane_triangle_2d" || imported.kind === "heat_plane_quad_2d") {
    effects.setStudyKind(imported.kind);
    effects.setHeatPlaneModel(imported.model);
    effects.setPlaneResultField("average_temperature");
    return;
  }

  if (imported.kind === "electrostatic_plane_triangle_2d" || imported.kind === "electrostatic_plane_quad_2d") {
    effects.setStudyKind(imported.kind);
    effects.setPlaneModel(ensurePlaneModelMaterials(imported.model, imported.material));
    effects.setActiveMaterial(imported.material);
    return;
  }

  if (imported.kind === "thermal_bar_1d") {
    effects.setStudyKind("thermal_bar_1d");
    effects.setThermalBarModel(imported.model);
    return;
  }

  if (imported.kind === "thermal_beam_1d") {
    effects.setStudyKind("thermal_beam_1d");
    effects.setThermalBeamModel(ensureBeamModelMaterials(imported.model, imported.material));
    effects.setActiveMaterial(imported.material);
    return;
  }

  if (imported.kind === "thermal_frame_2d") {
    effects.setStudyKind("thermal_frame_2d");
    effects.setThermalFrameModel(ensureFrameModelMaterials(imported.model, imported.material));
    effects.setActiveMaterial(imported.material);
    return;
  }

  if (imported.kind === "thermal_truss_2d") {
    effects.setStudyKind("thermal_truss_2d");
    effects.setThermalTrussModel(imported.model);
    return;
  }

  if (imported.kind === "thermal_truss_3d") {
    effects.setStudyKind("thermal_truss_3d");
    effects.setThermalTruss3dModel(imported.model);
    return;
  }

  if (imported.kind === "spring_2d") {
    effects.setStudyKind("spring_2d");
    effects.setSpring2dModel(imported.model);
    return;
  }

  if (imported.kind === "spring_3d") {
    effects.setStudyKind("spring_3d");
    effects.setSpring3dModel(imported.model);
    return;
  }

  if (imported.kind === "truss_3d") {
    effects.setStudyKind("truss_3d");
    effects.setTruss3dModel(ensureTruss3dModelMaterials(imported.model, imported.material));
    effects.setActiveMaterial(imported.material);
    return;
  }

  if (imported.kind === "frame_2d") {
    effects.setStudyKind("frame_2d");
    effects.setFrameModel(ensureFrameModelMaterials(imported.model, imported.material));
    effects.setActiveMaterial(imported.material);
    return;
  }

  if (imported.kind === "beam_1d") {
    effects.setStudyKind("beam_1d");
    effects.setBeamModel(ensureBeamModelMaterials(imported.model, imported.material));
    effects.setActiveMaterial(imported.material);
    return;
  }

  if (imported.kind === "torsion_1d") {
    effects.setStudyKind("torsion_1d");
    effects.setTorsionModel(imported.model);
    return;
  }

  if (
    imported.kind === "plane_triangle_2d" ||
    imported.kind === "plane_quad_2d" ||
    imported.kind === "thermal_plane_triangle_2d" ||
    imported.kind === "thermal_plane_quad_2d"
  ) {
    effects.setStudyKind(imported.kind);
    effects.setPlaneModel(ensurePlaneModelMaterials(imported.model, imported.material));
    effects.setActiveMaterial(imported.material);
    return;
  }

  effects.setStudyKind("axial_bar_1d");
  effects.setAxialForm({
    length: imported.length,
    area: imported.area,
    elements: imported.elements,
    tipForce: imported.tipForce,
    material: imported.material,
    youngsModulusGpa: imported.youngsModulusGpa,
  });
  effects.setActiveMaterial(imported.material);
}
