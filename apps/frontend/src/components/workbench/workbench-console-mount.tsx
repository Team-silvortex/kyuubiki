"use client";

import type { Dispatch, SetStateAction } from "react";
import type { WorkbenchAlertItem } from "./workbench-alert-strip";
import { buildWorkbenchConsoleSurfaceAlerts } from "./workbench-console-surface-alerts";
import type { WorkbenchNoticeItem, WorkbenchNoticeStateSetter } from "./workbench-notice-state";
import { WorkbenchConsole, type WorkbenchConsoleElement } from "./workbench-console";
import type { WorkbenchCopy } from "./workbench-copy";
import type { TrussDiagnostics } from "./workbench-defaults";
import type { WorkbenchRuntimeRecoveryState } from "./workbench-runtime-recovery";

type NodeSelectionData = {
  id?: string;
  index?: number;
  x?: number;
  y?: number;
  load_x?: number;
  load_y?: number;
  moment_z?: number;
};

type ConsoleMountProps = {
  sidebarSection: "study" | "model" | "workflow" | "library" | "system";
  t: Pick<
    WorkbenchCopy,
    | "nodeTable"
    | "report"
    | "dragNode"
    | "messages"
    | "noNodeSelected"
    | "loadCase"
    | "diagnostics"
    | "axialElements"
    | "frameElements"
    | "trussElements"
    | "spatialTrussElements"
    | "planeElements"
    | "span"
    | "maxTemperature"
    | "temperatureGradientY"
    | "maxStress"
    | "principalStress1"
    | "combinedStress"
    | "bendingStress"
    | "torsionStress"
    | "axialForce"
    | "stress"
    | "maxHeatFlux"
    | "maxInPlaneShear"
    | "maxMoment"
    | "maxTorque"
  >;
  message: string;
  importNotice?: WorkbenchNoticeItem | null;
  setImportNotice?: WorkbenchNoticeStateSetter | undefined;
  runtimeRecovery?: WorkbenchRuntimeRecoveryState;
  systemAlerts?: WorkbenchAlertItem[];
  setSystemAlerts?: Dispatch<SetStateAction<WorkbenchAlertItem[]>> | undefined;
  trussDiagnostics?: TrussDiagnostics | null;
  isPlane: boolean;
  isBeam: boolean;
  isTorsion: boolean;
  isFrameLike: boolean;
  isHeatBar: boolean;
  isThermalBar: boolean;
  isSpring1d: boolean;
  isSpring: boolean;
  isSpring2d: boolean;
  isSpring3d: boolean;
  isHeatPlane: boolean;
  isThermal: boolean;
  isThermalTruss3d: boolean;
  isTruss: boolean;
  isTruss3d: boolean;
  isAxial: boolean;
  selectedNodeIssues: unknown[];
  selectedNodeData: NodeSelectionData | null;
  selectedPlaneNodeData: NodeSelectionData | null;
  selectedBeamNodeData: NodeSelectionData | null;
  selectedTorsionNodeData: NodeSelectionData | null;
  selectedFrameNodeData: NodeSelectionData | null;
  selectedThermalNodeData: NodeSelectionData | null;
  selectedTruss3dNodeData: NodeSelectionData | null;
  spring3dModel: { nodes: Array<{ load_z?: number }> };
  axialElements: WorkbenchConsoleElement[];
  displayTruss3dElements: WorkbenchConsoleElement[];
  displayTrussElements: WorkbenchConsoleElement[];
  planeElements: WorkbenchConsoleElement[];
};

export function WorkbenchConsoleMount({
  sidebarSection,
  t,
  message,
  importNotice,
  setImportNotice,
  runtimeRecovery,
  systemAlerts = [],
  setSystemAlerts,
  trussDiagnostics,
  isPlane,
  isBeam,
  isTorsion,
  isFrameLike,
  isHeatBar,
  isThermalBar,
  isSpring1d,
  isSpring,
  isSpring2d,
  isSpring3d,
  isHeatPlane,
  isThermal,
  isThermalTruss3d,
  isTruss,
  isTruss3d,
  isAxial,
  selectedNodeIssues,
  selectedNodeData,
  selectedPlaneNodeData,
  selectedBeamNodeData,
  selectedTorsionNodeData,
  selectedFrameNodeData,
  selectedThermalNodeData,
  selectedTruss3dNodeData,
  spring3dModel,
  axialElements,
  displayTruss3dElements,
  displayTrussElements,
  planeElements,
}: ConsoleMountProps) {
  const selectedNodeId = isPlane
    ? selectedPlaneNodeData?.id ?? null
    : isBeam
      ? selectedBeamNodeData?.id ?? null
      : isTorsion
        ? selectedTorsionNodeData?.id ?? null
        : isFrameLike
          ? selectedFrameNodeData?.id ?? null
          : isHeatBar || isThermalBar
            ? selectedThermalNodeData?.id ?? null
            : isSpring3d || isThermalTruss3d
              ? selectedTruss3dNodeData?.id ?? null
              : selectedNodeData?.id ?? null;
  const selectedNodeX = isPlane
    ? selectedPlaneNodeData?.x
    : isBeam
      ? selectedBeamNodeData?.x
      : isTorsion
        ? selectedTorsionNodeData?.x
        : isFrameLike
          ? selectedFrameNodeData?.x
          : isHeatBar || isThermalBar
            ? selectedThermalNodeData?.x
            : isSpring3d || isThermalTruss3d
              ? selectedTruss3dNodeData?.x
              : selectedNodeData?.x;
  const selectedNodeY = isPlane
    ? selectedPlaneNodeData?.y
    : isBeam || isTorsion || isHeatBar || isThermalBar || isSpring1d
      ? 0
      : isFrameLike
        ? selectedFrameNodeData?.y
        : isSpring3d || isThermalTruss3d
          ? selectedTruss3dNodeData?.y
          : selectedNodeData?.y;
  const selectedNodeLoadY = isPlane
    ? selectedPlaneNodeData?.load_y
    : isBeam
      ? selectedBeamNodeData?.load_y
      : isTorsion
        ? selectedTorsionNodeData?.moment_z
        : isFrameLike
          ? selectedFrameNodeData?.load_y
          : isHeatBar || isThermalBar
            ? selectedThermalNodeData?.load_x
            : isSpring1d
              ? selectedNodeData?.load_x
              : isSpring2d
                ? selectedNodeData?.load_y
                : isSpring3d
                  ? (selectedTruss3dNodeData ? spring3dModel.nodes[selectedTruss3dNodeData.index ?? -1]?.load_z : undefined)
                  : selectedNodeData?.load_y;
  const selectedNodeIssueCount = isPlane ? null : selectedNodeIssues.length > 0 ? selectedNodeIssues.length : null;
  const elementTitle = isAxial
    ? t.axialElements
    : isBeam || isTorsion || isSpring || isHeatBar || isThermalBar
      ? t.frameElements
      : isTruss
        ? t.trussElements
        : isTruss3d
          ? t.spatialTrussElements
          : isFrameLike
            ? t.frameElements
            : t.planeElements;
  const stressLabel = isHeatPlane
    ? `${t.maxTemperature} / ${t.temperatureGradientY}`
    : isPlane
      ? `${t.maxStress} / ${t.principalStress1}`
      : isFrameLike
        ? t.combinedStress
        : isBeam
          ? t.bendingStress
          : isTorsion
            ? t.torsionStress
            : isHeatBar
              ? t.temperatureGradientY
              : isSpring || isThermalBar
                ? t.axialForce
                : t.stress;
  const axialForceLabel = isHeatPlane
    ? t.maxHeatFlux
    : isPlane
      ? t.maxInPlaneShear
      : isFrameLike || isBeam
        ? t.maxMoment
        : isTorsion
          ? t.maxTorque
          : isHeatBar
            ? t.maxHeatFlux
            : t.axialForce;
  const elements = (isAxial
    ? axialElements
    : isSpring3d || isThermalTruss3d
      ? displayTruss3dElements
      : isTruss || isSpring || isHeatBar || isThermal
        ? displayTrussElements
        : isTruss3d
          ? displayTruss3dElements
          : isFrameLike || isBeam
          ? displayTrussElements
          : planeElements) as WorkbenchConsoleElement[];
  const alerts: WorkbenchAlertItem[] = buildWorkbenchConsoleSurfaceAlerts({
    importNotice,
    setImportNotice,
    runtimeRecovery,
    systemAlerts,
    setSystemAlerts,
    trussDiagnostics,
    includeIntegrityAlerts: sidebarSection !== "model",
  });

  return (
    <div
      data-workbench-console="mount"
      data-workbench-panel="console"
      data-workbench-surface="built-in"
    >
      <WorkbenchConsole
        sidebarSection={sidebarSection}
        title={sidebarSection === "model" ? t.nodeTable : t.report}
        subtitle={message}
        modelMessageTitle={t.dragNode}
        reportMessageTitle={t.messages}
        message={message}
        alerts={alerts}
        dragNodeLabel={t.dragNode}
        noNodeSelectedLabel={t.noNodeSelected}
        loadCaseLabel={t.loadCase}
        diagnosticsLabel={t.diagnostics}
        selectedNodeId={selectedNodeId}
        selectedNodeX={selectedNodeX}
        selectedNodeY={selectedNodeY}
        selectedNodeLoadY={selectedNodeLoadY}
        selectedNodeIssueCount={selectedNodeIssueCount}
        elementTitle={elementTitle}
        spanLabel={t.span}
        stressLabel={stressLabel}
        axialForceLabel={axialForceLabel}
        isFrame={isFrameLike || isBeam || isTorsion}
        elements={elements}
      />
    </div>
  );
}
