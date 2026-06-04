"use client";

import type { MutableRefObject, PointerEvent as ReactPointerEvent } from "react";
import { fromSvgPoint } from "@/components/workbench/workbench-truss-helpers";
import { updateFrame2dNode } from "@/lib/workbench/frame2d-commands";
import type { WorkbenchStudyKind } from "@/lib/workbench/history";
import { toggleDraftSelection } from "@/lib/workbench/truss2d-commands";
import {
  addTruss3dNodeCommand,
  completeTruss3dLinkCommand,
  merge3dBoxSelection,
  updateTruss3dNodePositionCommand,
} from "@/lib/workbench/truss3d-commands";
import type {
  Frame2dJobInput,
  ThermalFrame2dJobInput,
} from "@/lib/api";

type TrussGestureControllerDeps = {
  studyKind: string;
  isFrameLike: boolean;
  isBeam: boolean;
  isTorsion: boolean;
  isHeatBar: boolean;
  isThermal: boolean;
  isThermalBar: boolean;
  truss3dLinkMode: boolean;
  truss3dModel: any;
  trussBounds: { minX: number; minY: number; maxX: number; maxY: number; width: number; height: number };
  roundValue: (value: number) => number;
  selectedNode: number | null;
  selectedTruss3dNodes: number[];
  memberDraftNodes: number[];
  draggingNode: number | null;
  dragHistoryCapturedRef: MutableRefObject<boolean>;
  dragFrameRef: MutableRefObject<number | null>;
  pendingDragPointRef: MutableRefObject<{ x: number; y: number } | null>;
  setStudyKind: (value: WorkbenchStudyKind) => void;
  setSidebarSection: (value: "study" | "model" | "workflow" | "library" | "system") => void;
  setSelectedNode: (value: number | null) => void;
  setSelectedTruss3dNodes: (value: number[]) => void;
  setSelectedElement: (value: number | null) => void;
  setMemberDraftNodes: (value: number[] | ((current: number[]) => number[])) => void;
  setMessage: (value: string) => void;
  setTruss3dLinkMode: (value: boolean | ((current: boolean) => boolean)) => void;
  setDraggingNode: (value: number | null) => void;
  setTruss3dModel: (value: any) => void;
  setTrussModel: (value: any) => void;
  setFrameModel: (value: any) => void;
  setThermalFrameModel: (value: any) => void;
  resetResults: () => void;
  recordHistory: (label: string) => void;
  labels: {
    addNodeAction: string;
    branchCreated: string;
    spaceNodeCreated: string;
    linkModeEnabled: string;
    linkModeDisabled: string;
    toggleMemberAction: string;
    memberRemoved: string;
    linkModeCompleted: string;
    selectTwoNodes: string;
    dragNodeAction: string;
  };
};

export function createWorkbenchTrussGestureController({
  studyKind,
  isFrameLike,
  isBeam,
  isTorsion,
  isHeatBar,
  isThermal,
  isThermalBar,
  truss3dLinkMode,
  truss3dModel,
  trussBounds,
  roundValue,
  selectedNode,
  selectedTruss3dNodes,
  memberDraftNodes,
  draggingNode,
  dragHistoryCapturedRef,
  dragFrameRef,
  pendingDragPointRef,
  setStudyKind,
  setSidebarSection,
  setSelectedNode,
  setSelectedTruss3dNodes,
  setSelectedElement,
  setMemberDraftNodes,
  setMessage,
  setTruss3dLinkMode,
  setDraggingNode,
  setTruss3dModel,
  setTrussModel,
  setFrameModel,
  setThermalFrameModel,
  resetResults,
  recordHistory,
  labels,
}: TrussGestureControllerDeps) {
  const addTruss3dNode = (connectToSelected: boolean) => {
    recordHistory(labels.addNodeAction);
    setStudyKind("truss_3d");
    setSidebarSection("model");
    resetResults();
    const nextState = addTruss3dNodeCommand(truss3dModel, connectToSelected, selectedNode, roundValue);
    setTruss3dModel(nextState.model);
    setSelectedNode(nextState.nextSelectedNode);
    setSelectedTruss3dNodes([nextState.nextSelectedNode]);
    setSelectedElement(nextState.nextSelectedElement);
    setMemberDraftNodes([]);
    setMessage(nextState.createdBranch ? labels.branchCreated : labels.spaceNodeCreated);
  };

  const toggleDraftNode = (index: number) => {
    setSelectedNode(index);
    setSelectedElement(null);
    setMemberDraftNodes((current) => toggleDraftSelection(current, index));
  };

  const toggleTruss3dLinkMode = () => {
    setTruss3dLinkMode((current) => {
      const next = !current;
      setMemberDraftNodes([]);
      setMessage(next ? labels.linkModeEnabled : labels.linkModeDisabled);
      return next;
    });
  };

  const completeTruss3dLink = (firstNode: number, secondNode: number) => {
    if (firstNode === secondNode) {
      setSelectedNode(firstNode);
      setSelectedTruss3dNodes([firstNode]);
      setMemberDraftNodes([firstNode]);
      return;
    }

    recordHistory(labels.toggleMemberAction);
    resetResults();
    const nextState = completeTruss3dLinkCommand(truss3dModel, firstNode, secondNode);
    if (nextState.repeatedNode) return;
    setTruss3dModel(nextState.model);
    setSelectedElement(nextState.removedExisting ? null : nextState.nextSelectedElement);
    setSelectedNode(secondNode);
    setSelectedTruss3dNodes([secondNode]);
    setMemberDraftNodes([secondNode]);
    setMessage(nextState.removedExisting ? labels.memberRemoved : labels.linkModeCompleted);
  };

  const handleTruss3dNodePick = (index: number) => {
    if (!truss3dLinkMode) {
      setSelectedTruss3dNodes([index]);
      toggleDraftNode(index);
      return;
    }

    setSelectedElement(null);
    setSelectedNode(index);
    setSelectedTruss3dNodes([index]);

    if (memberDraftNodes.length === 1) {
      completeTruss3dLink(memberDraftNodes[0], index);
      return;
    }

    setMemberDraftNodes([index]);
  };

  const handleTruss3dNodesBoxSelect = (indices: number[], append: boolean) => {
    const nextSelection = merge3dBoxSelection(selectedTruss3dNodes, indices, append);
    setSelectedTruss3dNodes(nextSelection);
    setSelectedElement(null);
    setMemberDraftNodes([]);
    setSelectedNode(nextSelection[0] ?? null);
  };

  const toggleTruss3dMemberFromDraft = () => {
    if (memberDraftNodes.length !== 2) {
      setMessage(labels.selectTwoNodes);
      return;
    }
    const [nodeI, nodeJ] = memberDraftNodes;
    completeTruss3dLink(nodeI, nodeJ);
  };

  const startTrussNodeDrag = (index: number) => {
    if (isBeam || isTorsion || isHeatBar || isThermalBar) {
      setSelectedNode(index);
      setSelectedElement(null);
      setMemberDraftNodes([]);
      return;
    }
    dragHistoryCapturedRef.current = false;
    setDraggingNode(index);
    toggleDraftNode(index);
  };

  const handleTrussPointerMove = (event: ReactPointerEvent<SVGSVGElement>) => {
    if (draggingNode === null || (studyKind !== "truss_2d" && studyKind !== "frame_2d" && studyKind !== "thermal_frame_2d")) return;
    if (!dragHistoryCapturedRef.current) {
      recordHistory(labels.dragNodeAction);
      dragHistoryCapturedRef.current = true;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const position = fromSvgPoint(event.clientX, event.clientY, rect, trussBounds, roundValue);
    pendingDragPointRef.current = position;

    if (dragFrameRef.current !== null) {
      return;
    }

    dragFrameRef.current = window.requestAnimationFrame(() => {
      dragFrameRef.current = null;
      const nextPoint = pendingDragPointRef.current;
      if (!nextPoint) return;

      resetResults();
      if (studyKind === "frame_2d" || studyKind === "thermal_frame_2d") {
        if (studyKind === "thermal_frame_2d") {
          setThermalFrameModel((current: ThermalFrame2dJobInput) =>
            updateFrame2dNode(updateFrame2dNode(current, draggingNode, "x", nextPoint.x), draggingNode, "y", nextPoint.y) as ThermalFrame2dJobInput,
          );
        } else {
          setFrameModel((current: Frame2dJobInput) =>
            updateFrame2dNode(updateFrame2dNode(current, draggingNode, "x", nextPoint.x), draggingNode, "y", nextPoint.y),
          );
        }
        return;
      }

      setTrussModel((current: any) => ({
        ...current,
        nodes: current.nodes.map((node: any, index: number) =>
          index === draggingNode ? { ...node, x: nextPoint.x, y: nextPoint.y } : node,
        ),
      }));
    });
  };

  const stopDraggingNode = () => {
    setDraggingNode(null);
    dragHistoryCapturedRef.current = false;
    pendingDragPointRef.current = null;
    if (dragFrameRef.current !== null) {
      window.cancelAnimationFrame(dragFrameRef.current);
      dragFrameRef.current = null;
    }
  };

  const updateTruss3dNodePosition = (index: number, position: { x: number; y: number; z: number }) => {
    resetResults();
    setTruss3dModel((current: any) => updateTruss3dNodePositionCommand(current, index, position, roundValue));
  };

  return {
    addTruss3dNode,
    completeTruss3dLink,
    handleTruss3dNodePick,
    handleTruss3dNodesBoxSelect,
    handleTrussPointerMove,
    startTrussNodeDrag,
    stopDraggingNode,
    toggleDraftNode,
    toggleTruss3dLinkMode,
    toggleTruss3dMemberFromDraft,
    updateTruss3dNodePosition,
  };
}
