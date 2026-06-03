"use client";

import type { WorkbenchStudyKind } from "@/lib/workbench/history";

type ScriptStateControllerDeps = {
  action: string;
  payload: Record<string, unknown>;
  language: "en" | "zh" | "ja" | "es";
  setStudyKind: (value: WorkbenchStudyKind) => void;
  setParametric: (updater: (current: any) => any) => void;
  setPanelParametric: (updater: (current: any) => any) => void;
  setTrussModel: (value: any) => void;
  setTruss3dModel: (value: any) => void;
  setPlaneModel: (value: any) => void;
  setFrameModel: (value: any) => void;
  setBeamModel: (value: any) => void;
  setSelectedNode: (value: number | null) => void;
  setSelectedElement: (value: number | null) => void;
  setSelectedTruss3dNodes: (value: number[]) => void;
  setMemberDraftNodes: (value: number[]) => void;
  setTruss3dLinkMode: (value: boolean) => void;
  setTruss3dFocusRequestVersion: (updater: (current: number) => number) => void;
  setTruss3dResetRequestVersion: (updater: (current: number) => number) => void;
  setTruss3dShowGrid: (value: boolean) => void;
  setTruss3dShowLabels: (value: boolean) => void;
  setTruss3dShowNodes: (value: boolean) => void;
  setImmersiveToolDrawerOpen: (value: boolean) => void;
  setImmersiveHelpDrawerOpen: (value: boolean) => void;
  setTruss3dBoxSelectMode: (value: boolean) => void;
  immersiveViewport: boolean;
  recordHistory: (label: string) => void;
  importActionLabel: string;
  editParametricLabel: string;
  resolveTruss2dJobInput: (payload: any) => any;
  resolveTruss3dJobInput: (payload: any) => any;
  resolvePlaneQuad2dJobInput: (payload: any) => any;
  resolvePlaneTriangle2dJobInput: (payload: any) => any;
  ensureFrameModelMaterials: (model: any, materialValue: string) => any;
  ensureBeamModelMaterials: (model: any, materialValue: string) => any;
  activeMaterial: string;
  resetActiveResult: () => void;
  projectHeatToThermoStudy: () => WorkbenchStudyKind | null;
  toggleImmersiveViewport: () => Promise<void>;
  handleUndo: () => void;
  handleRedo: () => void;
  runAnalysis: () => void;
  cancelCurrentJob: () => void;
  setTruss3dViewPreset: (value: "iso" | "front" | "right" | "top") => void;
  setTruss3dProjectionMode: (value: "ortho" | "persp") => void;
};

export async function handleWorkbenchScriptStateAction({
  action,
  payload,
  language,
  setStudyKind,
  setParametric,
  setPanelParametric,
  setTrussModel,
  setTruss3dModel,
  setPlaneModel,
  setFrameModel,
  setBeamModel,
  setSelectedNode,
  setSelectedElement,
  setSelectedTruss3dNodes,
  setMemberDraftNodes,
  setTruss3dLinkMode,
  setTruss3dFocusRequestVersion,
  setTruss3dResetRequestVersion,
  setTruss3dShowGrid,
  setTruss3dShowLabels,
  setTruss3dShowNodes,
  setImmersiveToolDrawerOpen,
  setImmersiveHelpDrawerOpen,
  setTruss3dBoxSelectMode,
  immersiveViewport,
  recordHistory,
  importActionLabel,
  editParametricLabel,
  resolveTruss2dJobInput,
  resolveTruss3dJobInput,
  resolvePlaneQuad2dJobInput,
  resolvePlaneTriangle2dJobInput,
  ensureFrameModelMaterials,
  ensureBeamModelMaterials,
  activeMaterial,
  resetActiveResult,
  projectHeatToThermoStudy,
  toggleImmersiveViewport,
  handleUndo,
  handleRedo,
  runAnalysis,
  cancelCurrentJob,
  setTruss3dViewPreset,
  setTruss3dProjectionMode,
}: ScriptStateControllerDeps): Promise<Record<string, unknown> | null> {
  switch (action) {
    case "state/setParametric": {
      recordHistory(editParametricLabel);
      setParametric((current) => ({ ...current, ...(payload as Record<string, unknown>) }));
      return { ok: true, action };
    }
    case "state/setPanelParametric": {
      recordHistory(editParametricLabel);
      setPanelParametric((current) => ({ ...current, ...(payload as Record<string, unknown>) }));
      return { ok: true, action };
    }
    case "state/replaceTruss2dModel": {
      recordHistory(importActionLabel);
      setStudyKind("truss_2d");
      setTrussModel(resolveTruss2dJobInput(payload));
      resetActiveResult();
      return { ok: true, action };
    }
    case "state/replaceTruss3dModel": {
      recordHistory(importActionLabel);
      setStudyKind("truss_3d");
      setTruss3dModel(resolveTruss3dJobInput(payload));
      resetActiveResult();
      return { ok: true, action };
    }
    case "state/replacePlaneModel": {
      recordHistory(importActionLabel);
      const payloadRecord = payload as Record<string, unknown>;
      const nextStudyKind = payloadRecord.study_kind === "plane_quad_2d" ? "plane_quad_2d" : "plane_triangle_2d";
      setStudyKind(nextStudyKind);
      setPlaneModel(
        nextStudyKind === "plane_quad_2d"
          ? resolvePlaneQuad2dJobInput(payload)
          : resolvePlaneTriangle2dJobInput(payload),
      );
      resetActiveResult();
      return { ok: true, action, studyKind: nextStudyKind };
    }
    case "state/projectHeatToThermo": {
      const projectedStudyKind = projectHeatToThermoStudy();
      if (!projectedStudyKind) {
        throw new Error(
          language === "zh"
            ? "当前研究没有可映射的热结果，或暂不支持映射到力-热研究。"
            : language === "ja"
              ? "現在の study には投影できる熱結果がないか、この熱→熱応力マッピングはまだ未対応です。"
              : "The current study does not have a usable thermal result, or this thermo-mechanical projection is not supported yet.",
        );
      }
      return { ok: true, action, studyKind: projectedStudyKind };
    }
    case "state/replaceFrameModel": {
      recordHistory(importActionLabel);
      setStudyKind("frame_2d");
      setFrameModel(ensureFrameModelMaterials(payload, activeMaterial));
      resetActiveResult();
      return { ok: true, action, studyKind: "frame_2d" };
    }
    case "state/replaceBeamModel": {
      recordHistory(importActionLabel);
      setStudyKind("beam_1d");
      setBeamModel(ensureBeamModelMaterials(payload, activeMaterial));
      resetActiveResult();
      return { ok: true, action, studyKind: "beam_1d" };
    }
    case "selection/set": {
      setSelectedNode(typeof payload.nodeIndex === "number" ? payload.nodeIndex : null);
      setSelectedElement(typeof payload.elementIndex === "number" ? payload.elementIndex : null);
      return { ok: true, action };
    }
    case "selection/set3d": {
      if (Array.isArray(payload.nodeIndices)) {
        setSelectedTruss3dNodes(payload.nodeIndices.filter((entry): entry is number => typeof entry === "number"));
      }
      if (typeof payload.anchorNodeIndex === "number" || payload.anchorNodeIndex === null) {
        setSelectedNode(typeof payload.anchorNodeIndex === "number" ? payload.anchorNodeIndex : null);
      }
      if (Array.isArray(payload.memberDraftNodeIndices)) {
        setMemberDraftNodes(payload.memberDraftNodeIndices.filter((entry): entry is number => typeof entry === "number"));
      }
      if (typeof payload.linkMode === "boolean") {
        setTruss3dLinkMode(payload.linkMode);
      }
      return { ok: true, action };
    }
    case "job/run": {
      runAnalysis();
      return { ok: true, action };
    }
    case "job/cancel": {
      cancelCurrentJob();
      return { ok: true, action };
    }
    case "history/undo": {
      handleUndo();
      return { ok: true, action };
    }
    case "history/redo": {
      handleRedo();
      return { ok: true, action };
    }
    case "viewport/toggleImmersive": {
      await toggleImmersiveViewport();
      return { ok: true, action };
    }
    case "viewport/set3dView": {
      if (payload.preset === "iso" || payload.preset === "front" || payload.preset === "right" || payload.preset === "top") {
        setTruss3dViewPreset(payload.preset);
      }
      if (payload.projection === "ortho" || payload.projection === "persp") {
        setTruss3dProjectionMode(payload.projection);
      }
      return { ok: true, action };
    }
    case "viewport/focus3d": {
      setTruss3dFocusRequestVersion((current) => current + 1);
      return { ok: true, action };
    }
    case "viewport/reset3d": {
      setTruss3dResetRequestVersion((current) => current + 1);
      return { ok: true, action };
    }
    case "viewport/toggleFlags": {
      if (typeof payload.grid === "boolean") {
        setTruss3dShowGrid(payload.grid);
      }
      if (typeof payload.labels === "boolean") {
        setTruss3dShowLabels(payload.labels);
      }
      if (typeof payload.nodes === "boolean") {
        setTruss3dShowNodes(payload.nodes);
      }
      return { ok: true, action };
    }
    case "viewport/setUiState": {
      if (typeof payload.immersiveViewport === "boolean" && payload.immersiveViewport !== immersiveViewport) {
        await toggleImmersiveViewport();
      }
      if (typeof payload.toolDrawerOpen === "boolean") {
        setImmersiveToolDrawerOpen(payload.toolDrawerOpen);
      }
      if (typeof payload.helpDrawerOpen === "boolean") {
        setImmersiveHelpDrawerOpen(payload.helpDrawerOpen);
      }
      if (typeof payload.boxSelectMode === "boolean") {
        setTruss3dBoxSelectMode(payload.boxSelectMode);
      }
      if (typeof payload.linkMode === "boolean") {
        setTruss3dLinkMode(payload.linkMode);
      }
      return { ok: true, action };
    }
    default:
      return null;
  }
}
