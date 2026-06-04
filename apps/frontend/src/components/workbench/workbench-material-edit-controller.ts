import type { Dispatch, SetStateAction } from "react";
import type {
  Frame2dJobInput,
  PlaneQuad2dJobInput,
  PlaneTriangle2dJobInput,
  ThermalFrame2dJobInput,
  ThermalPlaneQuad2dJobInput,
  ThermalPlaneTriangle2dJobInput,
  ThermalTruss2dJobInput,
  ThermalTruss3dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
} from "@/lib/api";
import { parseMaterialLibrary } from "@/lib/materials";
import {
  addCustomMaterialToFrameModel,
  addCustomMaterialToPlaneModel,
  addCustomMaterialToTruss3dModel,
  addCustomMaterialToTrussModel,
  addPresetMaterialToFrameModel,
  addPresetMaterialToPlaneModel,
  addPresetMaterialToTruss3dModel,
  addPresetMaterialToTrussModel,
  applyMaterialToFrameModel,
  applyMaterialToPlaneModel,
  applyMaterialToTruss3dModel,
  applyMaterialToTrussModel,
  deleteMaterialFromFrameModel,
  deleteMaterialFromPlaneModel,
  deleteMaterialFromTruss3dModel,
  deleteMaterialFromTrussModel,
  mergeImportedMaterials,
  updateMaterialInFrameModel,
  updateMaterialInPlaneModel,
  updateMaterialInTruss3dModel,
  updateMaterialInTrussModel,
} from "@/lib/workbench/material-commands";
import type {
  PlaneStudyJobInput,
  StudyKind,
  ThermalFrameStudyJobInput,
  ThermalTruss2dStudyJobInput,
  ThermalTruss3dStudyJobInput,
} from "@/components/workbench/workbench-types";

type MaterialControllerLabels = {
  editMemberAction: string;
  editMaterial: string;
  importedMaterialLibrary: string;
  initialFailed: string;
};

type MaterialControllerDeps = {
  activeMaterial: string;
  labels: MaterialControllerLabels;
  recordHistory: (label: string) => void;
  resetResults: () => void;
  selectedElement: number | null;
  setFrameModel: Dispatch<SetStateAction<Frame2dJobInput>>;
  setHiddenMaterials: Dispatch<SetStateAction<Record<StudyKind, string[]>>>;
  setMessage: Dispatch<SetStateAction<string>>;
  setPlaneModel: Dispatch<SetStateAction<PlaneStudyJobInput>>;
  setThermalFrameModel: Dispatch<SetStateAction<ThermalFrameStudyJobInput>>;
  setThermalTruss3dModel: Dispatch<SetStateAction<ThermalTruss3dStudyJobInput>>;
  setThermalTrussModel: Dispatch<SetStateAction<ThermalTruss2dStudyJobInput>>;
  setTruss3dModel: Dispatch<SetStateAction<Truss3dJobInput>>;
  setTrussModel: Dispatch<SetStateAction<Truss2dJobInput>>;
  studyKind: StudyKind;
};

export function createWorkbenchMaterialEditController(deps: MaterialControllerDeps) {
  const {
    activeMaterial,
    labels,
    recordHistory,
    resetResults,
    selectedElement,
    setFrameModel,
    setHiddenMaterials,
    setMessage,
    setPlaneModel,
    setThermalFrameModel,
    setThermalTruss3dModel,
    setThermalTrussModel,
    setTruss3dModel,
    setTrussModel,
    studyKind,
  } = deps;

  const addMaterialToCurrentModel = () => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(labels.editMemberAction);
    resetResults();

    if (studyKind === "truss_2d") {
      setTrussModel((current) => addPresetMaterialToTrussModel(current, activeMaterial));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => addPresetMaterialToTruss3dModel(current, activeMaterial));
      return;
    }

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel(
          (current) => addPresetMaterialToFrameModel(current, activeMaterial) as ThermalFrameStudyJobInput,
        );
      } else {
        setFrameModel((current) => addPresetMaterialToFrameModel(current, activeMaterial));
      }
      return;
    }

    setPlaneModel((current) => addPresetMaterialToPlaneModel(current, activeMaterial));
  };

  const addCustomMaterialToCurrentModel = () => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(labels.editMaterial);
    resetResults();

    if (studyKind === "truss_2d") {
      setTrussModel((current) => addCustomMaterialToTrussModel(current));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => addCustomMaterialToTruss3dModel(current));
      return;
    }

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => addCustomMaterialToFrameModel(current) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => addCustomMaterialToFrameModel(current));
      }
      return;
    }

    setPlaneModel((current) => addCustomMaterialToPlaneModel(current));
  };

  const applyMaterialToCurrentModel = (materialId: string, mode: "selected" | "all") => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(labels.editMemberAction);
    resetResults();

    if (studyKind === "truss_2d") {
      setTrussModel((current) => applyMaterialToTrussModel(current, materialId, mode, selectedElement));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => applyMaterialToTruss3dModel(current, materialId, mode, selectedElement));
      return;
    }

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel(
          (current) => applyMaterialToFrameModel(current, materialId, mode, selectedElement) as ThermalFrameStudyJobInput,
        );
      } else {
        setFrameModel((current) => applyMaterialToFrameModel(current, materialId, mode, selectedElement));
      }
      return;
    }

    setPlaneModel((current) => applyMaterialToPlaneModel(current, materialId, mode, selectedElement));
  };

  const toggleMaterialVisibility = (materialId: string) => {
    setHiddenMaterials((current) => {
      const hidden = current[studyKind];
      const nextHidden = hidden.includes(materialId)
        ? hidden.filter((entry) => entry !== materialId)
        : [...hidden, materialId];
      return { ...current, [studyKind]: nextHidden };
    });
  };

  const importMaterials = async (file: File | undefined) => {
    if (!file || studyKind === "axial_bar_1d") return;

    try {
      const imported = parseMaterialLibrary(await file.text(), file.name);
      recordHistory(labels.editMaterial);
      resetResults();

      if (studyKind === "truss_2d") {
        setTrussModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      } else if (studyKind === "truss_3d") {
        setTruss3dModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      } else if (studyKind === "frame_2d") {
        setFrameModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      } else if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => ({ ...current, materials: mergeImportedMaterials(current.materials, imported) }));
      } else {
        setPlaneModel((current) => ({
          ...current,
          materials: mergeImportedMaterials(
            (
              current as
                | PlaneTriangle2dJobInput
                | PlaneQuad2dJobInput
                | ThermalPlaneTriangle2dJobInput
                | ThermalPlaneQuad2dJobInput
            ).materials,
            imported,
          ),
        }));
      }

      setMessage(labels.importedMaterialLibrary);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : labels.initialFailed);
    }
  };

  const updateCurrentMaterial = (
    materialId: string,
    field: "name" | "youngs_modulus" | "poisson_ratio",
    value: string | number,
  ) => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(labels.editMemberAction);
    resetResults();

    if (studyKind === "truss_2d") {
      setTrussModel((current) => updateMaterialInTrussModel(current, materialId, field, value));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => updateMaterialInTruss3dModel(current, materialId, field, value));
      return;
    }

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => updateMaterialInFrameModel(current, materialId, field, value) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => updateMaterialInFrameModel(current, materialId, field, value));
      }
      return;
    }

    setPlaneModel((current) => updateMaterialInPlaneModel(current, materialId, field, value));
  };

  const deleteCurrentMaterial = (materialId: string) => {
    if (studyKind === "axial_bar_1d") return;
    recordHistory(labels.editMemberAction);
    resetResults();

    if (studyKind === "truss_2d") {
      setTrussModel((current) => deleteMaterialFromTrussModel(current, materialId));
      return;
    }

    if (studyKind === "truss_3d") {
      setTruss3dModel((current) => deleteMaterialFromTruss3dModel(current, materialId));
      return;
    }

    if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
      if (studyKind === "thermal_frame_2d") {
        setThermalFrameModel((current) => deleteMaterialFromFrameModel(current, materialId) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => deleteMaterialFromFrameModel(current, materialId));
      }
      return;
    }

    setPlaneModel((current) => deleteMaterialFromPlaneModel(current, materialId));
  };

  return {
    addCustomMaterialToCurrentModel,
    addMaterialToCurrentModel,
    applyMaterialToCurrentModel,
    deleteCurrentMaterial,
    importMaterials,
    toggleMaterialVisibility,
    updateCurrentMaterial,
  };
}
