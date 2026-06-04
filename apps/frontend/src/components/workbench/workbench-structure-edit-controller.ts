import type { Dispatch, SetStateAction } from "react";
import type {
  Frame2dJobInput,
  ThermalFrame2dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
} from "@/lib/api";
import type { ParametricTrussConfig } from "@/lib/models";
import {
  addFrame2dNode,
  assignFrame2dElementMaterial,
  deleteFrame2dElement,
  deleteFrame2dNode,
  toggleFrame2dMember,
  updateFrame2dElement,
  updateFrame2dNode,
} from "@/lib/workbench/frame2d-commands";
import { assignPlaneElementMaterial, updatePlaneElement, updatePlaneNode } from "@/lib/workbench/plane-commands";
import {
  addTruss2dNode,
  assignTruss2dElementMaterial,
  deleteTruss2dElement,
  deleteTruss2dNode,
  toggleTruss2dMember,
  updateTruss2dElement,
  updateTruss2dNode,
} from "@/lib/workbench/truss2d-commands";
import {
  applyTruss3dSelectedLoads,
  assignTruss3dElementMaterial,
  cloneTruss3dSelectedNodes,
  deleteTruss3dElementCommand,
  deleteTruss3dNodeCommand,
  nudgeTruss3dSelectedNodes,
  updateTruss3dElement,
  updateTruss3dSelectedNodes,
} from "@/lib/workbench/truss3d-commands";
import type {
  FrameStudyJobInput,
  HeatPlaneStudyJobInput,
  PlaneStudyJobInput,
  StudyKind,
  ThermalFrameStudyJobInput,
  ThermalTruss2dStudyJobInput,
  ThermalTruss3dStudyJobInput,
} from "@/components/workbench/workbench-types";

type StructureControllerLabels = {
  addNodeAction: string;
  branchCreated: string;
  deleteMemberAction: string;
  deleteNodeAction: string;
  editMemberAction: string;
  editNodeAction: string;
  memberCreated: string;
  memberDeleted: string;
  memberRemoved: string;
  nodeCreated: string;
  nodeDeleted: string;
  selectTwoNodes: string;
  spaceMemberDeleted: string;
  spaceNodeDeleted: string;
  toggleMemberAction: string;
};

type StructureControllerDeps = {
  activeFrameLikeModel: FrameStudyJobInput | ThermalFrameStudyJobInput;
  isFrameLike: boolean;
  isHeatPlane: boolean;
  isThermalFrame: boolean;
  isThermalTruss2d: boolean;
  isThermalTruss3d: boolean;
  labels: StructureControllerLabels;
  memberDraftNodes: number[];
  parametric: ParametricTrussConfig;
  recordHistory: (label: string) => void;
  resetResults: () => void;
  roundValue: (value: number) => number;
  selectedElement: number | null;
  selectedNode: number | null;
  selectedTruss3dNodes: number[];
  setFrameModel: Dispatch<SetStateAction<Frame2dJobInput>>;
  setHeatPlaneModel: Dispatch<SetStateAction<HeatPlaneStudyJobInput>>;
  setMemberDraftNodes: Dispatch<SetStateAction<number[]>>;
  setMessage: Dispatch<SetStateAction<string>>;
  setPlaneModel: Dispatch<SetStateAction<PlaneStudyJobInput>>;
  setSelectedElement: Dispatch<SetStateAction<number | null>>;
  setSelectedNode: Dispatch<SetStateAction<number | null>>;
  setSelectedTruss3dNodes: Dispatch<SetStateAction<number[]>>;
  setSidebarSection: Dispatch<SetStateAction<"study" | "model" | "workflow" | "library" | "system">>;
  setStudyKind: Dispatch<SetStateAction<StudyKind>>;
  setThermalFrameModel: Dispatch<SetStateAction<ThermalFrameStudyJobInput>>;
  setThermalTruss3dModel: Dispatch<SetStateAction<ThermalTruss3dStudyJobInput>>;
  setThermalTrussModel: Dispatch<SetStateAction<ThermalTruss2dStudyJobInput>>;
  setTruss3dModel: Dispatch<SetStateAction<Truss3dJobInput>>;
  setTrussModel: Dispatch<SetStateAction<Truss2dJobInput>>;
  studyKind: StudyKind;
  truss3dBatchLoadX: number;
  truss3dBatchLoadY: number;
  truss3dBatchLoadZ: number;
  truss3dModel: Truss3dJobInput;
  trussModel: Truss2dJobInput;
};

export function createWorkbenchStructureEditController(deps: StructureControllerDeps) {
  const {
    activeFrameLikeModel,
    isFrameLike,
    isHeatPlane,
    isThermalFrame,
    isThermalTruss2d,
    isThermalTruss3d,
    labels,
    memberDraftNodes,
    parametric,
    recordHistory,
    resetResults,
    roundValue,
    selectedElement,
    selectedNode,
    selectedTruss3dNodes,
    setFrameModel,
    setHeatPlaneModel,
    setMemberDraftNodes,
    setMessage,
    setPlaneModel,
    setSelectedElement,
    setSelectedNode,
    setSelectedTruss3dNodes,
    setSidebarSection,
    setStudyKind,
    setThermalFrameModel,
    setThermalTruss3dModel,
    setThermalTrussModel,
    setTruss3dModel,
    setTrussModel,
    studyKind,
    truss3dBatchLoadX,
    truss3dBatchLoadY,
    truss3dBatchLoadZ,
    truss3dModel,
    trussModel,
  } = deps;

  const updateSelectedNode = (key: keyof Truss2dJobInput["nodes"][number], value: number | boolean) => {
    if (selectedNode === null) return;
    recordHistory(labels.editNodeAction);
    resetResults();
    if (isThermalTruss2d) {
      setThermalTrussModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) => (index === selectedNode ? { ...node, [key]: value } : node)),
      }));
      return;
    }
    setTrussModel((current) => updateTruss2dNode(current, selectedNode, key, value));
  };

  const updateSelectedElement = (key: keyof Truss2dJobInput["elements"][number], value: number) => {
    if (selectedElement === null) return;
    recordHistory(labels.editMemberAction);
    resetResults();
    if (isThermalTruss2d) {
      setThermalTrussModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, [key]: value } : element,
        ),
      }));
      return;
    }
    setTrussModel((current) => updateTruss2dElement(current, selectedElement, key, value));
  };

  const assignSelectedElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    recordHistory(labels.editMemberAction);
    resetResults();
    if (isThermalTruss2d) {
      setThermalTrussModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, material_id: materialId } : element,
        ),
      }));
      return;
    }
    setTrussModel((current) => assignTruss2dElementMaterial(current, selectedElement, materialId));
  };

  const updateSelectedTruss3dNode = (key: keyof Truss3dJobInput["nodes"][number], value: number | boolean) => {
    if (selectedNode === null) return;
    recordHistory(labels.editNodeAction);
    resetResults();
    if (isThermalTruss3d) {
      setThermalTruss3dModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) => (index === selectedNode ? { ...node, [key]: value } : node)),
      }));
      return;
    }
    setTruss3dModel((current) => ({
      ...current,
      nodes: current.nodes.map((node, index) => (index === selectedNode ? { ...node, [key]: value } : node)),
    }));
  };

  const updateSelectedTruss3dNodes = (key: keyof Truss3dJobInput["nodes"][number], value: number | boolean) => {
    const targetIndices =
      selectedTruss3dNodes.length > 0 ? selectedTruss3dNodes : selectedNode !== null ? [selectedNode] : [];
    if (targetIndices.length === 0) return;
    recordHistory(labels.editNodeAction);
    resetResults();
    setTruss3dModel((current) => updateTruss3dSelectedNodes(current, selectedTruss3dNodes, selectedNode, key, value));
  };

  const nudgeSelectedTruss3dNodes = (axis: "x" | "y" | "z", delta: number) => {
    const targetIndices =
      selectedTruss3dNodes.length > 0 ? selectedTruss3dNodes : selectedNode !== null ? [selectedNode] : [];
    if (targetIndices.length === 0) return;
    recordHistory(labels.editNodeAction);
    resetResults();
    setTruss3dModel((current) =>
      nudgeTruss3dSelectedNodes(current, selectedTruss3dNodes, selectedNode, axis, delta, roundValue),
    );
  };

  const applySelectedTruss3dLoads = (mode: "apply" | "clear") => {
    const targetIndices =
      selectedTruss3dNodes.length > 0 ? selectedTruss3dNodes : selectedNode !== null ? [selectedNode] : [];
    if (targetIndices.length === 0) return;
    recordHistory(labels.editNodeAction);
    resetResults();
    setTruss3dModel((current) =>
      applyTruss3dSelectedLoads(current, selectedTruss3dNodes, selectedNode, mode, {
        x: truss3dBatchLoadX,
        y: truss3dBatchLoadY,
        z: truss3dBatchLoadZ,
      }),
    );
  };

  const cloneSelectedTruss3dNodes = (mirrorAxis: "x" | "y" | "z" | null = null) => {
    const targetIndices =
      selectedTruss3dNodes.length > 0 ? selectedTruss3dNodes : selectedNode !== null ? [selectedNode] : [];
    if (targetIndices.length === 0) return;
    recordHistory(labels.addNodeAction);
    resetResults();
    const nextState = cloneTruss3dSelectedNodes(truss3dModel, selectedTruss3dNodes, selectedNode, roundValue, mirrorAxis);
    setTruss3dModel(nextState.model);
    if (nextState.nextSelection.length > 0) {
      setSelectedTruss3dNodes(nextState.nextSelection);
      setSelectedNode(nextState.nextSelection[0] ?? null);
      setMemberDraftNodes([]);
      setSelectedElement(null);
    }
  };

  const updateSelectedTruss3dElement = (key: keyof Truss3dJobInput["elements"][number], value: number) => {
    if (selectedElement === null) return;
    recordHistory(labels.editMemberAction);
    resetResults();
    if (isThermalTruss3d) {
      setThermalTruss3dModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, [key]: value } : element,
        ),
      }));
      return;
    }
    setTruss3dModel((current) => updateTruss3dElement(current, selectedElement, key, value));
  };

  const assignSelectedTruss3dElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    recordHistory(labels.editMemberAction);
    resetResults();
    if (isThermalTruss3d) {
      setThermalTruss3dModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, material_id: materialId } : element,
        ),
      }));
      return;
    }
    setTruss3dModel((current) => assignTruss3dElementMaterial(current, selectedElement, materialId));
  };

  const updateSelectedPlaneNode = (
    key:
      | "x"
      | "y"
      | "load_x"
      | "load_y"
      | "fix_x"
      | "fix_y"
      | "temperature_delta"
      | "fix_temperature"
      | "temperature"
      | "heat_load",
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(labels.editNodeAction);
    resetResults();
    if (isHeatPlane) {
      setHeatPlaneModel((current: HeatPlaneStudyJobInput) => updatePlaneNode(current, selectedNode, key, value) as HeatPlaneStudyJobInput);
      return;
    }
    setPlaneModel((current) => updatePlaneNode(current, selectedNode, key, value));
  };

  const updateSelectedPlaneElement = (
    key: "thickness" | "youngs_modulus" | "poisson_ratio" | "thermal_expansion" | "conductivity",
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(labels.editMemberAction);
    resetResults();
    if (isHeatPlane) {
      setHeatPlaneModel((current: HeatPlaneStudyJobInput) => updatePlaneElement(current, selectedElement, key, value) as HeatPlaneStudyJobInput);
      return;
    }
    setPlaneModel((current) => updatePlaneElement(current, selectedElement, key, value));
  };

  const assignSelectedPlaneElementMaterial = (materialId: string) => {
    if (selectedElement === null || isHeatPlane) return;
    recordHistory(labels.editMemberAction);
    resetResults();
    setPlaneModel((current) => assignPlaneElementMaterial(current, selectedElement, materialId));
  };

  const updateSelectedFrameNode = (
    key: keyof Frame2dJobInput["nodes"][number] | keyof ThermalFrame2dJobInput["nodes"][number],
    value: number | boolean,
  ) => {
    if (selectedNode === null) return;
    recordHistory(labels.editNodeAction);
    resetResults();
    if (isThermalFrame) {
      setThermalFrameModel((current) => ({
        ...current,
        nodes: current.nodes.map((node, index) => (index === selectedNode ? { ...node, [key]: value } : node)),
      }));
      return;
    }
    setFrameModel((current) =>
      updateFrame2dNode(current, selectedNode, key as keyof Frame2dJobInput["nodes"][number], value),
    );
  };

  const updateSelectedFrameElement = (
    key: keyof Frame2dJobInput["elements"][number] | keyof ThermalFrame2dJobInput["elements"][number] | "distributed_load_y",
    value: number,
  ) => {
    if (selectedElement === null) return;
    recordHistory(labels.editMemberAction);
    resetResults();
    if (isThermalFrame) {
      setThermalFrameModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, [key]: value } : element,
        ),
      }));
      return;
    }
    setFrameModel((current) =>
      updateFrame2dElement(current, selectedElement, key as keyof Frame2dJobInput["elements"][number], value),
    );
  };

  const assignSelectedFrameElementMaterial = (materialId: string) => {
    if (selectedElement === null) return;
    recordHistory(labels.editMemberAction);
    resetResults();
    if (isThermalFrame) {
      setThermalFrameModel((current) => ({
        ...current,
        elements: current.elements.map((element, index) =>
          index === selectedElement ? { ...element, material_id: materialId } : element,
        ),
      }));
      return;
    }
    setFrameModel((current) => assignFrame2dElementMaterial(current, selectedElement, materialId));
  };

  const addNode = (connectToSelected: boolean) => {
    if (isFrameLike) {
      recordHistory(labels.addNodeAction);
      setStudyKind(studyKind);
      setSidebarSection("model");
      resetResults();
      const nextState = addFrame2dNode(activeFrameLikeModel, connectToSelected, selectedNode, roundValue);
      if (isThermalFrame) {
        setThermalFrameModel(nextState.model as ThermalFrameStudyJobInput);
      } else {
        setFrameModel(nextState.model);
      }
      setSelectedNode(nextState.nextSelectedNode);
      setSelectedElement(nextState.nextSelectedElement);
      setMemberDraftNodes([]);
      setMessage(nextState.createdBranch ? labels.branchCreated : labels.nodeCreated);
      return;
    }
    recordHistory(labels.addNodeAction);
    setStudyKind("truss_2d");
    setSidebarSection("model");
    resetResults();
    const nextState = addTruss2dNode(trussModel, connectToSelected, selectedNode, parametric, roundValue);
    setTrussModel(nextState.model);
    setSelectedNode(nextState.nextSelectedNode);
    setSelectedElement(nextState.nextSelectedElement);
    setMemberDraftNodes([]);
    setMessage(nextState.createdBranch ? labels.branchCreated : labels.nodeCreated);
  };

  const deleteSelectedNode = () => {
    if (selectedNode === null) return;
    recordHistory(labels.deleteNodeAction);
    resetResults();
    if (isFrameLike) {
      if (isThermalFrame) {
        setThermalFrameModel((current) => deleteFrame2dNode(current, selectedNode) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => deleteFrame2dNode(current, selectedNode));
      }
      setSelectedNode(null);
      setSelectedTruss3dNodes([]);
      setSelectedElement(null);
      setMemberDraftNodes([]);
      setMessage(labels.nodeDeleted);
      return;
    }
    setTrussModel((current) => deleteTruss2dNode(current, selectedNode));
    setSelectedNode(null);
    setSelectedTruss3dNodes([]);
    setSelectedElement(null);
    setMemberDraftNodes([]);
    setMessage(labels.nodeDeleted);
  };

  const deleteSelectedTruss3dNode = () => {
    if (selectedNode === null) return;
    recordHistory(labels.deleteNodeAction);
    resetResults();
    setTruss3dModel((current) => deleteTruss3dNodeCommand(current, selectedNode));
    setSelectedNode(null);
    setSelectedElement(null);
    setMemberDraftNodes([]);
    setMessage(labels.spaceNodeDeleted);
  };

  const toggleMemberFromDraft = () => {
    if (memberDraftNodes.length !== 2) {
      setMessage(labels.selectTwoNodes);
      return;
    }
    recordHistory(labels.toggleMemberAction);
    resetResults();
    if (isFrameLike) {
      const nextState = toggleFrame2dMember(activeFrameLikeModel, memberDraftNodes);
      if (!nextState.valid) return;
      if (isThermalFrame) {
        setThermalFrameModel(nextState.model as ThermalFrameStudyJobInput);
      } else {
        setFrameModel(nextState.model);
      }
      setSelectedElement(nextState.removedExisting ? null : nextState.nextSelectedElement);
      setSelectedTruss3dNodes([]);
      setMemberDraftNodes([]);
      setMessage(nextState.removedExisting ? labels.memberRemoved : labels.memberCreated);
      return;
    }
    const nextState = toggleTruss2dMember(trussModel, memberDraftNodes, parametric);
    if (!nextState.valid) return;
    setTrussModel(nextState.model);
    setSelectedElement(nextState.removedExisting ? null : nextState.nextSelectedElement);
    setSelectedTruss3dNodes([]);
    setMemberDraftNodes([]);
    setMessage(nextState.removedExisting ? labels.memberRemoved : labels.memberCreated);
  };

  const deleteSelectedElement = () => {
    if (selectedElement === null) return;
    recordHistory(labels.deleteMemberAction);
    resetResults();
    if (isFrameLike) {
      if (isThermalFrame) {
        setThermalFrameModel((current) => deleteFrame2dElement(current, selectedElement) as ThermalFrameStudyJobInput);
      } else {
        setFrameModel((current) => deleteFrame2dElement(current, selectedElement));
      }
      setSelectedElement(null);
      setMessage(labels.memberDeleted);
      return;
    }
    setTrussModel((current) => deleteTruss2dElement(current, selectedElement));
    setSelectedElement(null);
    setMessage(labels.memberDeleted);
  };

  const deleteSelectedTruss3dElement = () => {
    if (selectedElement === null) return;
    recordHistory(labels.deleteMemberAction);
    resetResults();
    setTruss3dModel((current) => deleteTruss3dElementCommand(current, selectedElement));
    setSelectedElement(null);
    setSelectedTruss3dNodes([]);
    setMessage(labels.spaceMemberDeleted);
  };

  return {
    addNode,
    applySelectedTruss3dLoads,
    assignSelectedElementMaterial,
    assignSelectedFrameElementMaterial,
    assignSelectedPlaneElementMaterial,
    assignSelectedTruss3dElementMaterial,
    cloneSelectedTruss3dNodes,
    deleteSelectedElement,
    deleteSelectedNode,
    deleteSelectedTruss3dElement,
    deleteSelectedTruss3dNode,
    nudgeSelectedTruss3dNodes,
    toggleMemberFromDraft,
    updateSelectedElement,
    updateSelectedFrameElement,
    updateSelectedFrameNode,
    updateSelectedNode,
    updateSelectedPlaneElement,
    updateSelectedPlaneNode,
    updateSelectedTruss3dElement,
    updateSelectedTruss3dNode,
    updateSelectedTruss3dNodes,
  };
}
