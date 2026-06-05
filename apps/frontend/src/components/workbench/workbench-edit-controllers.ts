"use client";

import { createWorkbenchFrameLikeEditController } from "@/components/workbench/workbench-frame-like-edit-controller";
import { createWorkbenchMaterialEditController } from "@/components/workbench/workbench-material-edit-controller";
import { createWorkbenchStructureEditController } from "@/components/workbench/workbench-structure-edit-controller";
import { createWorkbenchTrussGestureController } from "@/components/workbench/workbench-truss-gesture-controller";
import { findNearestConnectableNode } from "@/components/workbench/workbench-truss-helpers";

export function useWorkbenchEditControllers(props: Record<string, any>) {
  const materialController = createWorkbenchMaterialEditController({
    activeMaterial: props.activeMaterial,
    labels: {
      editMemberAction: props.t.editMemberAction,
      editMaterial: props.t.editMaterial,
      importedMaterialLibrary:
        props.language === "zh"
          ? "外部材料库已导入。"
          : props.language === "ja"
            ? "外部材料ライブラリを取り込みました。"
            : "Imported external material library.",
      initialFailed: props.t.initialFailed,
    },
    recordHistory: props.recordHistory,
    resetResults: props.resetResults,
    selectedElement: props.selectedElement,
    setFrameModel: props.setFrameModel,
    setHiddenMaterials: props.setHiddenMaterials,
    setMessage: props.setMessage,
    setPlaneModel: props.setPlaneModel,
    setThermalFrameModel: props.setThermalFrameModel,
    setThermalTruss3dModel: props.setThermalTruss3dModel,
    setThermalTrussModel: props.setThermalTrussModel,
    setTruss3dModel: props.setTruss3dModel,
    setTrussModel: props.setTrussModel,
    studyKind: props.studyKind,
  });

  const structureController = createWorkbenchStructureEditController({
    activeFrameLikeModel: props.activeFrameLikeModel,
    isFrameLike: props.isFrameLike,
    isHeatPlane: props.isHeatPlane,
    isThermalFrame: props.isThermalFrame,
    isThermalTruss2d: props.isThermalTruss2d,
    isThermalTruss3d: props.isThermalTruss3d,
    labels: {
      addNodeAction: props.t.addNodeAction,
      branchCreated: props.t.branchCreated,
      deleteMemberAction: props.t.deleteMemberAction,
      deleteNodeAction: props.t.deleteNodeAction,
      editMemberAction: props.t.editMemberAction,
      editNodeAction: props.t.editNodeAction,
      memberCreated: props.t.memberCreated,
      memberDeleted: props.t.memberDeleted,
      memberRemoved: props.t.memberRemoved,
      nodeCreated: props.t.nodeCreated,
      nodeDeleted: props.t.nodeDeleted,
      selectTwoNodes: props.t.selectTwoNodes,
      spaceMemberDeleted: props.t.spaceMemberDeleted,
      spaceNodeDeleted: props.t.spaceNodeDeleted,
      toggleMemberAction: props.t.toggleMemberAction,
    },
    memberDraftNodes: props.memberDraftNodes,
    parametric: props.parametric,
    recordHistory: props.recordHistory,
    resetResults: props.resetResults,
    roundValue: props.round,
    selectedElement: props.selectedElement,
    selectedNode: props.selectedNode,
    selectedTruss3dNodes: props.selectedTruss3dNodes,
    setFrameModel: props.setFrameModel,
    setHeatPlaneModel: props.setHeatPlaneModel,
    setMemberDraftNodes: props.setMemberDraftNodes,
    setMessage: props.setMessage,
    setPlaneModel: props.setPlaneModel,
    setSelectedElement: props.setSelectedElement,
    setSelectedNode: props.setSelectedNode,
    setSelectedTruss3dNodes: props.setSelectedTruss3dNodes,
    setSidebarSection: props.setSidebarSection,
    setStudyKind: props.setStudyKind,
    setThermalFrameModel: props.setThermalFrameModel,
    setThermalTruss3dModel: props.setThermalTruss3dModel,
    setThermalTrussModel: props.setThermalTrussModel,
    setTruss3dModel: props.setTruss3dModel,
    setTrussModel: props.setTrussModel,
    studyKind: props.studyKind,
    truss3dBatchLoadX: props.truss3dBatchLoadX,
    truss3dBatchLoadY: props.truss3dBatchLoadY,
    truss3dBatchLoadZ: props.truss3dBatchLoadZ,
    truss3dModel: props.truss3dModel,
    trussModel: props.trussModel,
  });

  const gestureController = createWorkbenchTrussGestureController({
    studyKind: props.studyKind,
    isFrameLike: props.isFrameLike,
    isBeam: props.isBeam,
    isTorsion: props.isTorsion,
    isHeatBar: props.isHeatBar,
    isThermal: props.isThermal,
    isThermalBar: props.isThermalBar,
    truss3dLinkMode: props.truss3dLinkMode,
    truss3dModel: props.truss3dModel,
    trussBounds: props.trussBounds,
    roundValue: props.round,
    selectedNode: props.selectedNode,
    selectedTruss3dNodes: props.selectedTruss3dNodes,
    memberDraftNodes: props.memberDraftNodes,
    draggingNode: props.draggingNode,
    dragHistoryCapturedRef: props.dragHistoryCapturedRef,
    dragFrameRef: props.dragFrameRef,
    pendingDragPointRef: props.pendingDragPointRef,
    setStudyKind: props.setStudyKind,
    setSidebarSection: props.setSidebarSection,
    setSelectedNode: props.setSelectedNode,
    setSelectedTruss3dNodes: props.setSelectedTruss3dNodes,
    setSelectedElement: props.setSelectedElement,
    setMemberDraftNodes: props.setMemberDraftNodes,
    setMessage: props.setMessage,
    setTruss3dLinkMode: props.setTruss3dLinkMode,
    setDraggingNode: props.setDraggingNode,
    setTruss3dModel: props.setTruss3dModel,
    setTrussModel: props.setTrussModel,
    setFrameModel: props.setFrameModel,
    setThermalFrameModel: props.setThermalFrameModel,
    resetResults: props.resetResults,
    recordHistory: props.recordHistory,
    labels: {
      addNodeAction: props.t.addNodeAction,
      branchCreated: props.t.spaceBranchCreated,
      spaceNodeCreated: props.t.spaceNodeCreated,
      linkModeEnabled: props.t.linkModeEnabled,
      linkModeDisabled: props.t.linkModeDisabled,
      toggleMemberAction: props.t.toggleMemberAction,
      memberRemoved: props.t.memberRemoved,
      linkModeCompleted: props.t.linkModeCompleted,
      selectTwoNodes: props.t.selectTwoNodes,
      dragNodeAction: props.t.dragNodeAction,
    },
  });

  const frameLikeController = createWorkbenchFrameLikeEditController({
    labels: {
      editMaterial: props.t.editMaterial,
      editMemberAction: props.t.editMemberAction,
      editNodeAction: props.t.editNodeAction,
    },
    recordHistory: props.recordHistory,
    resetResults: props.resetResults,
    selectedElement: props.selectedElement,
    selectedNode: props.selectedNode,
    isTorsion: props.isTorsion,
    isHeatBar: props.isHeatBar,
    isBeam: props.isBeam,
    setActiveMaterial: props.setActiveMaterial,
    setAxialForm: props.setAxialForm,
    setParametric: props.setParametric,
    setTorsionModel: props.setTorsionModel,
    setHeatBarModel: props.setHeatBarModel,
    setBeamModel: props.setBeamModel,
    updateSelectedFrameNodeBase: structureController.updateSelectedFrameNode,
    updateSelectedFrameElementBase: structureController.updateSelectedFrameElement,
    assignSelectedFrameElementMaterialBase: structureController.assignSelectedFrameElementMaterial,
  });

  const applyTrussSuggestion = (suggestion: any) => {
    props.recordHistory(props.t.applySuggestionAction);
    props.resetResults();
    props.setStudyKind("truss_2d");
    props.setSidebarSection("model");
    props.setSelectedElement(null);
    props.setSelectedNode(suggestion.nodeIndex);
    props.setMemberDraftNodes([]);

    if (suggestion.kind === "fix_support") {
      props.setTrussModel((current: any) => ({
        ...current,
        nodes: current.nodes.map((node: any, index: number) =>
          index === suggestion.nodeIndex
            ? { ...node, [suggestion.axis === "x" ? "fix_x" : "fix_y"]: true }
            : node,
        ),
      }));
      props.setMessage(suggestion.axis === "x" ? props.t.suggestionAppliedSupportX : props.t.suggestionAppliedSupportY);
      return;
    }

    let connected = false;
    props.setTrussModel((current: any) => {
      const nearestIndex = findNearestConnectableNode(current, suggestion.nodeIndex);
      if (nearestIndex === null) return current;
      connected = true;
      const material = current.materials?.[0];
      return {
        ...current,
        elements: [
          ...current.elements,
          {
            id: `e${current.elements.length}`,
            node_i: suggestion.nodeIndex,
            node_j: nearestIndex,
            area: props.parametric.area,
            youngs_modulus: material?.youngs_modulus ?? props.parametric.youngsModulusGpa * 1.0e9,
            material_id: material?.id,
          },
        ],
      };
    });
    props.setMessage(connected ? props.t.suggestionAppliedLink : props.t.suggestionNoLinkTarget);
  };

  return {
    ...materialController,
    ...structureController,
    ...gestureController,
    ...frameLikeController,
    applyTrussSuggestion,
  };
}
