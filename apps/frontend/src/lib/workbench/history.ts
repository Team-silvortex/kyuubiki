import type { Dispatch, SetStateAction } from "react";
import type {
  PlaneTriangle2dJobInput,
  Truss2dJobInput,
  Truss3dJobInput,
} from "@/lib/api";
import type { ParametricPanelConfig, ParametricTrussConfig } from "@/lib/models";

export type WorkbenchStudyKind = "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d";
export type WorkbenchSidebarSection = "study" | "model" | "library" | "system";

export type WorkbenchAxialFormState = {
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  material: string;
  youngsModulusGpa: number;
};

export type WorkbenchSnapshot = {
  studyKind: WorkbenchStudyKind;
  axialForm: WorkbenchAxialFormState;
  trussModel: Truss2dJobInput;
  truss3dModel: Truss3dJobInput;
  planeModel: PlaneTriangle2dJobInput;
  parametric: ParametricTrussConfig;
  panelParametric: ParametricPanelConfig;
  activeMaterial: string;
  loadedModelName: string;
  sidebarSection: WorkbenchSidebarSection;
  selectedNode: number | null;
  selectedElement: number | null;
  memberDraftNodes: number[];
};

export type HistoryEntry = {
  label: string;
  snapshot: WorkbenchSnapshot;
};

export type AssistantTransactionEntry = {
  id: string;
  summary: string;
  createdAt: string;
  snapshot: WorkbenchSnapshot;
  executedActions: string[];
};

type SnapshotSetters = {
  setStudyKind: Dispatch<SetStateAction<WorkbenchStudyKind>>;
  setAxialForm: Dispatch<SetStateAction<WorkbenchAxialFormState>>;
  setTrussModel: Dispatch<SetStateAction<Truss2dJobInput>>;
  setTruss3dModel: Dispatch<SetStateAction<Truss3dJobInput>>;
  setPlaneModel: Dispatch<SetStateAction<PlaneTriangle2dJobInput>>;
  setParametric: Dispatch<SetStateAction<ParametricTrussConfig>>;
  setPanelParametric: Dispatch<SetStateAction<ParametricPanelConfig>>;
  setActiveMaterial: Dispatch<SetStateAction<string>>;
  setLoadedModelName: Dispatch<SetStateAction<string>>;
  setSidebarSection: Dispatch<SetStateAction<WorkbenchSidebarSection>>;
  setSelectedNode: Dispatch<SetStateAction<number | null>>;
  setSelectedElement: Dispatch<SetStateAction<number | null>>;
  setMemberDraftNodes: Dispatch<SetStateAction<number[]>>;
};

export function buildWorkbenchSnapshot(snapshot: WorkbenchSnapshot): WorkbenchSnapshot {
  return {
    ...snapshot,
    memberDraftNodes: [...snapshot.memberDraftNodes],
  };
}

export function restoreWorkbenchSnapshot(
  snapshot: WorkbenchSnapshot,
  setters: SnapshotSetters,
  onRestored?: () => void,
) {
  setters.setStudyKind(snapshot.studyKind);
  setters.setAxialForm(snapshot.axialForm);
  setters.setTrussModel(snapshot.trussModel);
  setters.setTruss3dModel(snapshot.truss3dModel);
  setters.setPlaneModel(snapshot.planeModel);
  setters.setParametric(snapshot.parametric);
  setters.setPanelParametric(snapshot.panelParametric);
  setters.setActiveMaterial(snapshot.activeMaterial);
  setters.setLoadedModelName(snapshot.loadedModelName);
  setters.setSidebarSection(snapshot.sidebarSection);
  setters.setSelectedNode(snapshot.selectedNode);
  setters.setSelectedElement(snapshot.selectedElement);
  setters.setMemberDraftNodes(snapshot.memberDraftNodes);
  onRestored?.();
}

export function pushHistoryEntry(
  current: HistoryEntry[],
  label: string,
  snapshot: WorkbenchSnapshot,
  maxEntries = 40,
) {
  return [...current.slice(-(maxEntries - 1)), { label, snapshot }];
}

export function stepHistory(
  source: HistoryEntry[],
  target: HistoryEntry[],
  currentSnapshot: WorkbenchSnapshot,
  maxEntries = 40,
) {
  const entry = source.at(-1) ?? null;
  if (!entry) {
    return {
      entry: null,
      nextSource: source,
      nextTarget: target,
    };
  }

  return {
    entry,
    nextSource: source.slice(0, -1),
    nextTarget: pushHistoryEntry(target, entry.label, currentSnapshot, maxEntries),
  };
}

export function createAssistantTransactionEntry(
  summary: string,
  executedActions: string[],
  snapshot: WorkbenchSnapshot,
): AssistantTransactionEntry {
  return {
    id: `assistant-${Date.now()}`,
    summary,
    createdAt: new Date().toISOString(),
    snapshot,
    executedActions,
  };
}
