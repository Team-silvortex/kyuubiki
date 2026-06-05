"use client";

import type { Dispatch, SetStateAction } from "react";

import type {
  Beam1dJobInput,
  Frame2dJobInput,
  HeatBar1dJobInput,
  ThermalFrame2dJobInput,
  Torsion1dJobInput,
} from "@/lib/api";
import { MATERIAL_PRESETS } from "@/lib/materials";
import type { ParametricTrussConfig } from "@/lib/models";
import type { AxialFormState } from "@/components/workbench/workbench-defaults";

type FrameLikeEditDeps = {
  labels: {
    editMaterial: string;
    editMemberAction: string;
    editNodeAction: string;
  };
  recordHistory: (label: string) => void;
  resetResults: () => void;
  selectedElement: number | null;
  selectedNode: number | null;
  isTorsion: boolean;
  isHeatBar: boolean;
  isBeam: boolean;
  setActiveMaterial: Dispatch<SetStateAction<string>>;
  setAxialForm: Dispatch<SetStateAction<AxialFormState>>;
  setParametric: Dispatch<SetStateAction<ParametricTrussConfig>>;
  setTorsionModel: Dispatch<SetStateAction<Torsion1dJobInput>>;
  setHeatBarModel: Dispatch<SetStateAction<HeatBar1dJobInput>>;
  setBeamModel: Dispatch<SetStateAction<Beam1dJobInput>>;
  updateSelectedFrameNodeBase: (
    key: keyof Frame2dJobInput["nodes"][number] | keyof ThermalFrame2dJobInput["nodes"][number],
    value: number | boolean,
  ) => void;
  updateSelectedFrameElementBase: (
    key:
      | keyof Frame2dJobInput["elements"][number]
      | keyof ThermalFrame2dJobInput["elements"][number]
      | "distributed_load_y",
    value: number,
  ) => void;
  assignSelectedFrameElementMaterialBase: (materialId: string) => void;
};

export function createWorkbenchFrameLikeEditController(deps: FrameLikeEditDeps) {
  const {
    labels,
    recordHistory,
    resetResults,
    selectedElement,
    selectedNode,
    isTorsion,
    isHeatBar,
    isBeam,
    setActiveMaterial,
    setAxialForm,
    setParametric,
    setTorsionModel,
    setHeatBarModel,
    setBeamModel,
    updateSelectedFrameNodeBase,
    updateSelectedFrameElementBase,
    assignSelectedFrameElementMaterialBase,
  } = deps;

  const handleMaterialChange = (value: string) => {
    recordHistory(labels.editMaterial);
    const preset = MATERIAL_PRESETS.find((item) => item.value === value);
    setActiveMaterial(value);
    setAxialForm((current) => ({
      ...current,
      material: value,
      youngsModulusGpa: preset?.modulusGpa ?? current.youngsModulusGpa,
    }));
    setParametric((current: ParametricTrussConfig) => ({
      ...current,
      youngsModulusGpa: preset?.modulusGpa ?? current.youngsModulusGpa,
    }));
  };

  const updateSelectedFrameNode = (
    key: keyof Frame2dJobInput["nodes"][number] | keyof ThermalFrame2dJobInput["nodes"][number],
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(labels.editNodeAction);
    resetResults();

    if (isTorsion) {
      setTorsionModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === selectedNode
            ? {
                ...node,
                ...(key === "x" ? { x: Number(value) } : {}),
                ...(key === "moment_z" ? { torque_z: Number(value) } : {}),
                ...(key === "fix_rz" ? { fix_rz: Boolean(value) } : {}),
              }
            : node,
        ),
      }));
      return;
    }

    if (isHeatBar) {
      setHeatBarModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) =>
          index === selectedNode
            ? {
                ...node,
                ...(key === "x" ? { x: Number(value) } : {}),
                ...(key === "load_x" ? { heat_load: Number(value) } : {}),
                ...(key === "fix_x" ? { fix_temperature: Boolean(value) } : {}),
                ...(key === "temperature_delta" ? { temperature: Number(value) } : {}),
              }
            : node,
        ),
      }));
      return;
    }

    updateSelectedFrameNodeBase(key, value);
  };

  const updateSelectedFrameElement = (
    key:
      | keyof Frame2dJobInput["elements"][number]
      | keyof ThermalFrame2dJobInput["elements"][number]
      | "distributed_load_y",
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(labels.editMemberAction);
    resetResults();

    if (isTorsion) {
      setTorsionModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement
            ? {
                ...element,
                ...(key === "youngs_modulus" ? { shear_modulus: value } : {}),
                ...(key === "moment_of_inertia" ? { polar_moment: value } : {}),
                ...(key === "section_modulus" ? { section_modulus: value } : {}),
              }
            : element,
        ),
      }));
      return;
    }

    if (isHeatBar) {
      setHeatBarModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement
            ? {
                ...element,
                ...(key === "area" ? { area: value } : {}),
                ...(key === "youngs_modulus" ? { conductivity: value } : {}),
              }
            : element,
        ),
      }));
      return;
    }

    if (isBeam) {
      setBeamModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, [key]: value } : element,
        ),
      }));
      return;
    }

    updateSelectedFrameElementBase(key, value);
  };

  const assignSelectedFrameElementMaterial = (materialId: string) => {
    if (selectedElement === null || isTorsion) return;
    assignSelectedFrameElementMaterialBase(materialId);
  };

  return {
    handleMaterialChange,
    updateSelectedFrameNode,
    updateSelectedFrameElement,
    assignSelectedFrameElementMaterial,
  };
}
