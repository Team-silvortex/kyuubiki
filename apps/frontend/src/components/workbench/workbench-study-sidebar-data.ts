import type { WorkbenchCopy, WorkbenchLanguage } from "@/components/workbench/workbench-copy";
import type { DisplayTruss3dElement } from "@/components/workbench/workbench-defaults";
import type { StudyKind } from "@/components/workbench/workbench-types";
import {
  buildStudyControlsRows,
  buildStudySummaryRows,
  buildTruss3dTreeRows,
} from "@/lib/workbench/view-models";

type FrameLikeNode = { fix_x?: boolean; fix_y?: boolean; fix_rz?: boolean; load_y?: number; moment_z?: number; temperature_delta?: number };
type Truss2dNode = { fix_x: boolean; fix_y: boolean; load_x?: number; load_y?: number; temperature_delta?: number };
type Truss3dNode = { fix_x: boolean; fix_y: boolean; fix_z: boolean; load_x: number; load_y: number; load_z: number; temperature_delta?: number; x: number; y: number; z: number; id: string };
type HeatNode = { heat_load?: number; temperature?: number; fix_temperature?: boolean };
type ThermalPlaneNode = { temperature_delta?: number; fix_x?: boolean; fix_y?: boolean; load_y?: number };
type BeamNode = { load_y: number; moment_z: number; fix_y?: boolean; fix_rz?: boolean };
type TorsionNode = { torque_z: number };
type ThermalGradientElement = { distributed_load_y?: number; temperature_gradient_y?: number };
type PlaneElement = { thickness?: number };

type StudySidebarDataArgs = {
  t: WorkbenchCopy;
  language: WorkbenchLanguage;
  studyKind: StudyKind;
  loadedModelName: string;
  activeMaterial: string;
  localMaterialLabel: (materialId: string, language: WorkbenchLanguage) => string;
  fixed: (value: number | undefined, decimals?: number) => string;
  isAxial: boolean;
  isSpring: boolean;
  isSpring1d: boolean;
  isSpring2d: boolean;
  isSpring3d: boolean;
  isBeam: boolean;
  isTorsion: boolean;
  isTruss: boolean;
  isTruss3d: boolean;
  isFrameLike: boolean;
  isFrame: boolean;
  isPlane: boolean;
  isHeatBar: boolean;
  isHeatPlane: boolean;
  isHeatPlaneTriangle: boolean;
  isHeatPlaneQuad: boolean;
  isThermal: boolean;
  isThermalBar: boolean;
  isThermalBeam: boolean;
  isThermalFrame: boolean;
  isThermalTruss2d: boolean;
  isThermalPlaneTriangle: boolean;
  isThermalPlaneQuad: boolean;
  axialForm: { elements: number; tipForce: number };
  heatBarModel: { nodes: HeatNode[]; elements: unknown[] };
  heatPlaneModel: { nodes: HeatNode[]; elements: PlaneElement[] };
  thermalBarModel: { nodes: Array<{ temperature_delta: number; load_x: number; fix_x: boolean }>; elements: unknown[] };
  thermalBeamModel: { nodes: BeamNode[]; elements: ThermalGradientElement[] };
  thermalFrameModel: { nodes: FrameLikeNode[]; elements: ThermalGradientElement[] };
  thermalTrussModel: { nodes: Truss2dNode[]; elements: unknown[] };
  thermalTruss3dModel: { nodes: Truss3dNode[]; elements: unknown[] };
  springModel: { nodes: Array<{ load_x: number }>; elements: unknown[] };
  spring2dModel: { nodes: Array<{ load_x: number; load_y: number }>; elements: unknown[] };
  spring3dModel: { nodes: Array<{ load_x: number; load_y: number; load_z: number }>; elements: unknown[] };
  beamModel: { nodes: BeamNode[]; elements: Array<{ distributed_load_y?: number }> };
  torsionModel: { nodes: TorsionNode[]; elements: unknown[] };
  trussModel: { nodes: Array<{ load_y: number }>; elements: unknown[] };
  truss3dModel: { nodes: Truss3dNode[]; elements: Array<{ area?: number }> };
  frameModel: { nodes: FrameLikeNode[]; elements: unknown[] };
  activePlaneInputModel: { nodes: Array<ThermalPlaneNode | HeatNode>; elements: PlaneElement[] };
  activeFrameLikeModel: { nodes: FrameLikeNode[]; elements: unknown[] };
  displayTruss3dElements: DisplayTruss3dElement[];
  truss3dTreeNodes: Truss3dNode[];
  selectedNode: number | null;
  selectedTruss3dNodes: number[];
  memberDraftNodes: number[];
};

export function buildWorkbenchStudySidebarData(args: StudySidebarDataArgs) {
  const {
    t,
    language,
    studyKind,
    loadedModelName,
    activeMaterial,
    localMaterialLabel,
    fixed,
    isAxial,
    isSpring,
    isSpring1d,
    isSpring2d,
    isSpring3d,
    isBeam,
    isTorsion,
    isTruss,
    isTruss3d,
    isFrameLike,
    isFrame,
    isPlane,
    isHeatBar,
    isHeatPlane,
    isHeatPlaneTriangle,
    isHeatPlaneQuad,
    isThermal,
    isThermalBar,
    isThermalBeam,
    isThermalFrame,
    isThermalTruss2d,
    isThermalPlaneTriangle,
    isThermalPlaneQuad,
    axialForm,
    heatBarModel,
    heatPlaneModel,
    thermalBarModel,
    thermalBeamModel,
    thermalFrameModel,
    thermalTrussModel,
    thermalTruss3dModel,
    springModel,
    spring2dModel,
    spring3dModel,
    beamModel,
    torsionModel,
    trussModel,
    truss3dModel,
    frameModel,
    activePlaneInputModel,
    activeFrameLikeModel,
    displayTruss3dElements,
    truss3dTreeNodes,
    selectedNode,
    selectedTruss3dNodes,
    memberDraftNodes,
  } = args;

  const joinThermalIntent = (...parts: Array<string | null | undefined | false>) => parts.filter(Boolean).join(" + ");
  const countRestrainedFrameLikeNodes = (nodes: FrameLikeNode[]) =>
    nodes.reduce((sum, node) => sum + (node.fix_x || node.fix_y || node.fix_rz ? 1 : 0), 0);
  const countRestrainedTruss2dNodes = (nodes: Truss2dNode[]) =>
    nodes.reduce((sum, node) => sum + (node.fix_x || node.fix_y ? 1 : 0), 0);
  const countRestrainedTruss3dNodes = (nodes: Truss3dNode[]) =>
    nodes.reduce((sum, node) => sum + (node.fix_x || node.fix_y || node.fix_z ? 1 : 0), 0);

  const thermalPlaneHeatedNodeCount =
    isThermalPlaneTriangle || isThermalPlaneQuad
      ? activePlaneInputModel.nodes.reduce((sum, node) => {
          const thermalNode = node as ThermalPlaneNode;
          return sum + (typeof thermalNode.temperature_delta === "number" && Math.abs(thermalNode.temperature_delta) > 0 ? 1 : 0);
        }, 0)
      : 0;
  const thermalPlaneRestrainedNodeCount =
    isThermalPlaneTriangle || isThermalPlaneQuad
      ? activePlaneInputModel.nodes.reduce((sum, node) => {
          const supportNode = node as ThermalPlaneNode;
          return sum + (supportNode.fix_x || supportNode.fix_y ? 1 : 0);
        }, 0)
      : 0;

  const thermalIntentValue = isHeatBar
    ? joinThermalIntent(t.conductionField, heatBarModel.nodes.some((node) => Math.abs(node.heat_load ?? 0) > 0) && t.heatSourceField)
    : isHeatPlane
      ? joinThermalIntent(
          t.conductionField,
          activePlaneInputModel.nodes.some((node) => "heat_load" in node && Math.abs((node as HeatNode).heat_load ?? 0) > 0) && t.heatSourceField,
        )
      : isThermalBar
        ? joinThermalIntent(t.nodalTemperatureRise, t.thermalBarResponse)
        : isThermalBeam
          ? joinThermalIntent(t.memberTemperatureGradient, t.thermalBeamResponse)
          : isThermalFrame
            ? joinThermalIntent(
                thermalFrameModel.nodes.some((node) => Math.abs(node.temperature_delta ?? 0) > 0) && t.nodalTemperatureRise,
                thermalFrameModel.elements.some((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0) && t.memberTemperatureGradient,
                t.thermalFrameResponse,
              )
            : isThermalTruss2d
              ? joinThermalIntent(t.nodalTemperatureRise, t.thermalTrussResponse)
              : studyKind === "thermal_truss_3d"
                ? joinThermalIntent(t.nodalTemperatureRise, t.thermalTrussResponse)
                : isThermalPlaneTriangle || isThermalPlaneQuad
                  ? joinThermalIntent(t.nodalTemperatureRise, t.thermoelasticPlaneResponse)
                  : undefined;

  const materialLabelFor =
    typeof localMaterialLabel === "function"
      ? localMaterialLabel
      : (materialId: string) => materialId || "--";

  const thermalBoundaryValue = isHeatBar
    ? `${heatBarModel.nodes.filter((node) => node.fix_temperature).length} ${t.prescribedTemperatureNodes} · ${heatBarModel.nodes.filter((node) => Math.abs(node.heat_load ?? 0) > 0).length} ${t.sourceNodes}`
    : isHeatPlane
      ? `${activePlaneInputModel.nodes.reduce((sum, node) => sum + (("fix_temperature" in node && (node as HeatNode).fix_temperature) ? 1 : 0), 0)} ${t.prescribedTemperatureNodes} · ${activePlaneInputModel.nodes.reduce((sum, node) => sum + (("heat_load" in node && Math.abs((node as HeatNode).heat_load ?? 0) > 0) ? 1 : 0), 0)} ${t.sourceNodes}`
      : isThermalBar
        ? `${thermalBarModel.nodes.filter((node) => Math.abs(node.temperature_delta) > 0).length} ${t.heatedNodes} · ${thermalBarModel.nodes.filter((node) => node.fix_x).length} ${t.restrainedSupports}`
        : isThermalBeam
          ? `${thermalBeamModel.elements.filter((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0).length} ${t.gradientMembers} · ${countRestrainedFrameLikeNodes(thermalBeamModel.nodes)} ${t.restrainedSupports}`
          : isThermalFrame
            ? `${thermalFrameModel.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length} ${t.heatedNodes} · ${thermalFrameModel.elements.filter((element) => Math.abs(element.temperature_gradient_y ?? 0) > 0).length} ${t.gradientMembers} · ${countRestrainedFrameLikeNodes(thermalFrameModel.nodes)} ${t.restrainedSupports}`
            : isThermalTruss2d
              ? `${thermalTrussModel.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length} ${t.heatedNodes} · ${countRestrainedTruss2dNodes(thermalTrussModel.nodes)} ${t.restrainedSupports}`
              : studyKind === "thermal_truss_3d"
                ? `${thermalTruss3dModel.nodes.filter((node) => Math.abs(node.temperature_delta ?? 0) > 0).length} ${t.heatedNodes} · ${countRestrainedTruss3dNodes(thermalTruss3dModel.nodes)} ${t.restrainedSupports}`
                : isThermalPlaneTriangle || isThermalPlaneQuad
                  ? `${thermalPlaneHeatedNodeCount} ${t.heatedNodes} · ${thermalPlaneRestrainedNodeCount} ${t.restrainedSupports}`
                  : undefined;

  const studySummaryRows = buildStudySummaryRows({
    labels: {
      modelName: t.modelName,
      material: t.material,
      mesh: t.mesh,
      load: t.load,
      support: t.support,
    },
    loadedModelName,
    materialLabel: isSpring || isHeatBar || isThermalBar || isTorsion ? "--" : materialLabelFor(activeMaterial, language),
    meshValue: isAxial
      ? axialForm.elements
      : isHeatBar
        ? heatBarModel.elements.length
        : isThermalBar
          ? thermalBarModel.elements.length
          : isThermalBeam
            ? thermalBeamModel.elements.length
            : isThermalTruss2d
              ? thermalTrussModel.elements.length
              : isSpring1d
                ? springModel.elements.length
                : isSpring2d
                  ? spring2dModel.elements.length
                  : isSpring3d
                    ? spring3dModel.elements.length
                    : isBeam
                      ? beamModel.elements.length
                      : isTorsion
                        ? torsionModel.elements.length
                        : isTruss
                          ? trussModel.elements.length
                          : isTruss3d
                            ? studyKind === "thermal_truss_3d"
                              ? thermalTruss3dModel.elements.length
                              : truss3dModel.elements.length
                            : isFrame
                              ? activeFrameLikeModel.elements.length
                              : activePlaneInputModel.elements.length,
    loadValue:
      studyKind === "axial_bar_1d"
        ? `${fixed(axialForm.tipForce, 0)} N`
        : isHeatBar
          ? `${fixed(heatBarModel.nodes.reduce((sum, node) => sum + (node.heat_load ?? 0), 0), 0)} W · T ${fixed(heatBarModel.nodes.reduce((max, node) => Math.max(max, node.temperature ?? 0), 0), 1)} °`
          : isThermalBar
            ? `${fixed(thermalBarModel.nodes.reduce((sum, node) => sum + node.load_x, 0), 0)} N · ΔT ${fixed(thermalBarModel.nodes.reduce((sum, node) => sum + Math.abs(node.temperature_delta), 0), 1)} °`
            : isThermalBeam
              ? `${fixed(thermalBeamModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N · ${fixed(thermalBeamModel.nodes.reduce((sum, node) => sum + node.moment_z, 0), 0)} N·m · q ${fixed(thermalBeamModel.elements.reduce((sum, element) => sum + Math.abs(element.distributed_load_y ?? 0), 0), 0)} N/m · ΔTy ${fixed(thermalBeamModel.elements.reduce((sum, element) => sum + Math.abs(element.temperature_gradient_y ?? 0), 0), 1)} °`
              : isThermalTruss2d
                ? `${fixed(thermalTrussModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x ?? 0, node.load_y ?? 0), 0), 0)} N · ΔT ${fixed(thermalTrussModel.nodes.reduce((sum, node) => sum + Math.abs(node.temperature_delta ?? 0), 0), 1)} °`
                : isSpring1d
                  ? `${fixed(springModel.nodes.reduce((sum, node) => sum + node.load_x, 0), 0)} N`
                  : isSpring2d
                    ? `${fixed(spring2dModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x, node.load_y), 0), 0)} N`
                    : isSpring3d
                      ? `${fixed(spring3dModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x, node.load_y, node.load_z), 0), 0)} N`
                      : isBeam
                        ? `${fixed(beamModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N · ${fixed(beamModel.nodes.reduce((sum, node) => sum + node.moment_z, 0), 0)} N·m · ${fixed(beamModel.elements.reduce((sum, element) => sum + Math.abs(element.distributed_load_y ?? 0), 0), 0)} N/m`
                        : isTorsion
                          ? `${fixed(torsionModel.nodes.reduce((sum, node) => sum + Math.abs(node.torque_z), 0), 0)} N·m`
                          : isTruss
                            ? `${fixed(trussModel.nodes.reduce((sum, node) => sum + node.load_y, 0), 0)} N`
                            : isTruss3d
                              ? studyKind === "thermal_truss_3d"
                                ? `${fixed(thermalTruss3dModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x ?? 0, node.load_y ?? 0, node.load_z ?? 0), 0), 0)} N · ΔT ${fixed(thermalTruss3dModel.nodes.reduce((sum, node) => sum + Math.abs(node.temperature_delta ?? 0), 0), 1)} °`
                                : `${fixed(truss3dModel.nodes.reduce((sum, node) => sum + (node.load_z ?? 0), 0), 0)} N`
                              : isFrame
                                ? `${fixed(frameModel.nodes.reduce((sum, node) => sum + (node.load_y ?? 0), 0), 0)} N · ${fixed(frameModel.nodes.reduce((sum, node) => sum + (node.moment_z ?? 0), 0), 0)} N·m`
                                : isHeatPlane
                                  ? `${fixed(activePlaneInputModel.nodes.reduce((sum, node) => sum + ("heat_load" in node ? ((node as HeatNode).heat_load ?? 0) : 0), 0), 0)} W · T ${fixed(activePlaneInputModel.nodes.reduce((max, node) => Math.max(max, "temperature" in node ? ((node as HeatNode).temperature ?? 0) : 0), 0), 1)} °`
                                  : `${fixed(activePlaneInputModel.nodes.reduce((sum, node) => sum + ((node as ThermalPlaneNode).load_y ?? 0), 0), 0)} N`,
    supportValue:
      studyKind === "axial_bar_1d"
        ? "Node 0"
        : isHeatBar
          ? `${heatBarModel.nodes.filter((node) => node.fix_temperature).length} prescribed T`
          : isThermalBar
            ? "Restrained span"
            : isThermalBeam
              ? "Thermal cantilever"
              : isThermalTruss2d
                ? "Thermal truss anchors"
                : isSpring1d
                  ? "Axial anchor"
                  : isSpring2d
                    ? "Planar anchors"
                    : isSpring3d
                      ? "Spatial anchors"
                      : isTruss3d
                        ? "Fixed tripod"
                        : isTorsion
                          ? "Fixed shaft end"
                          : isHeatPlane
                            ? `${activePlaneInputModel.nodes.reduce((sum, node) => sum + (("fix_temperature" in node && (node as HeatNode).fix_temperature) ? 1 : 0), 0)} prescribed T`
                            : isFrameLike || isBeam
                              ? "Moment-resisting base"
                              : "Pinned base",
  });

  const studyControlsRows = buildStudyControlsRows({
    labels: {
      nodes: t.nodes,
      trussElements: t.trussElements,
      material: t.material,
      sourceModel: t.sourceModel,
      spatialTrussElements: t.spatialTrussElements,
      load: t.load,
      planeElements: t.planeElements,
      frameElements: t.frameElements,
      thickness: t.thickness,
      thermalIntent: t.thermalIntent,
      thermalBoundary: t.thermalBoundary,
    },
    studyKind,
    loadedModelName,
    materialLabel: materialLabelFor(activeMaterial, language),
    trussNodeCount: isFrameLike ? activeFrameLikeModel.nodes.length : trussModel.nodes.length,
    trussElementCount: isFrameLike ? activeFrameLikeModel.elements.length : trussModel.elements.length,
    truss3dNodeCount: isSpring3d ? spring3dModel.nodes.length : truss3dModel.nodes.length,
    truss3dElementCount: isSpring3d ? spring3dModel.elements.length : truss3dModel.elements.length,
    truss3dLoadValue: isSpring3d
      ? `${fixed(spring3dModel.nodes.reduce((sum, node) => sum + Math.hypot(node.load_x, node.load_y, node.load_z), 0), 0)} N`
      : `${fixed(truss3dModel.nodes.reduce((sum, node) => sum + (node.load_z ?? 0), 0), 0)} N`,
    planeNodeCount:
      studyKind === "frame_2d" || studyKind === "thermal_frame_2d"
        ? activeFrameLikeModel.nodes.length
        : studyKind === "beam_1d"
          ? beamModel.nodes.length
          : studyKind === "thermal_beam_1d"
            ? thermalBeamModel.nodes.length
            : studyKind === "torsion_1d"
              ? torsionModel.nodes.length
              : studyKind === "thermal_bar_1d"
                ? thermalBarModel.nodes.length
                : studyKind === "spring_1d"
                  ? springModel.nodes.length
                  : studyKind === "spring_2d"
                    ? spring2dModel.nodes.length
                    : studyKind === "spring_3d"
                      ? spring3dModel.nodes.length
                      : activePlaneInputModel.nodes.length,
    planeElementCount:
      studyKind === "frame_2d" || studyKind === "thermal_frame_2d"
        ? activeFrameLikeModel.elements.length
        : studyKind === "beam_1d"
          ? beamModel.elements.length
          : studyKind === "thermal_beam_1d"
            ? thermalBeamModel.elements.length
            : studyKind === "torsion_1d"
              ? torsionModel.elements.length
              : studyKind === "heat_bar_1d"
                ? heatBarModel.elements.length
                : studyKind === "thermal_bar_1d"
                  ? thermalBarModel.elements.length
                  : studyKind === "spring_1d"
                    ? springModel.elements.length
                    : studyKind === "spring_2d"
                      ? spring2dModel.elements.length
                      : studyKind === "spring_3d"
                        ? spring3dModel.elements.length
                        : activePlaneInputModel.elements.length,
    planeThicknessValue:
      studyKind === "frame_2d" ||
      studyKind === "thermal_frame_2d" ||
      studyKind === "beam_1d" ||
      studyKind === "thermal_beam_1d" ||
      studyKind === "torsion_1d" ||
      studyKind === "heat_bar_1d" ||
      studyKind === "thermal_bar_1d" ||
      studyKind === "spring_1d" ||
      studyKind === "spring_2d" ||
      studyKind === "spring_3d"
        ? "--"
        : fixed(activePlaneInputModel.elements[0]?.thickness, 3),
    thermalIntentValue,
    thermalBoundaryValue,
  });

  const truss3dTreeRows = buildTruss3dTreeRows({
    nodes: truss3dTreeNodes,
    elements: displayTruss3dElements,
    selectedNode,
    selectedTruss3dNodes,
    memberDraftNodes,
    fixed,
  });

  return {
    thermalIntentValue,
    thermalBoundaryValue,
    studySummaryRows,
    studyControlsRows,
    truss3dTreeRows,
  };
}
