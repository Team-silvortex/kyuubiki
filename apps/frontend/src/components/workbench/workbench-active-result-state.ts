"use client";

import {
  analyzeTrussModel,
  getTrussBounds,
  summarizeTrussStability,
} from "@/components/workbench/workbench-truss-helpers";
import { analyzeWorkbenchComponentIntegrity, mergeWorkbenchDiagnostics } from "@/components/workbench/workbench-component-integrity";
import {
  buildDisplayBeamElements,
  buildDisplayBeamNodes,
  buildDisplaySpring2dElements,
  buildDisplaySpring2dNodes,
  buildDisplaySpringElements,
  buildDisplaySpringNodes,
  buildDisplayTorsionElements,
  buildDisplayTorsionNodes,
} from "@/components/workbench/workbench-display-line-helpers";
import {
  buildDisplayThermalBeamElements,
  buildDisplayThermalBeamNodes,
} from "@/components/workbench/workbench-display-line-helpers";
import {
  buildDisplayFrameElements,
  buildDisplayFrameNodes,
  buildDisplayHeatBarElements,
  buildDisplayHeatBarNodes,
  buildDisplayThermalBarElements,
  buildDisplayThermalBarNodes,
  buildDisplayThermalFrameElements,
  buildDisplayThermalFrameNodes,
  buildDisplayThermalTrussElements,
  buildDisplayThermalTrussNodes,
  buildDisplayTrussElements,
  buildDisplayTrussNodes,
} from "@/components/workbench/workbench-display-planar-helpers";
import {
  buildDisplaySpring3dElements,
  buildDisplaySpring3dNodes,
  buildDisplayThermalTruss3dElements,
  buildDisplayThermalTruss3dNodes,
  buildDisplayTruss3dElements,
  buildDisplayTruss3dNodes,
} from "@/components/workbench/workbench-display-spatial-helpers";

export function buildWorkbenchActiveResultState(props: Record<string, any>) {
  const {
    result,
    studyKind,
    job,
    resultWindow,
    t,
    selectedNode,
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
    trussModel,
    truss3dModel,
    planeModel,
    frameModel,
    beamModel,
    torsionModel,
    isAxial,
    isHeatBar,
    isHeatPlaneTriangle,
    isHeatPlaneQuad,
    isHeatPlane,
    isThermalBar,
    isThermalBeam,
    isThermalFrame,
    isThermalTruss2d,
    isThermalTruss3d,
    isSpring1d,
    isSpring2d,
    isSpring3d,
    isFrame,
    isPlane,
    isTruss,
    isBeam,
    isTorsion,
    isSpring,
  } = props;

  const axialResult = isAxial && props.isAxialResult(result) ? result : null;
  const heatBarResult = isHeatBar && props.isHeatBar1dResult(result) ? result : null;
  const heatPlaneTriangleResult = isHeatPlaneTriangle && props.isHeatPlaneTriangle2dResult(result) ? result : null;
  const heatPlaneQuadResult = isHeatPlaneQuad && props.isHeatPlaneQuad2dResult(result) ? result : null;
  const thermalBarResult = isThermalBar && props.isThermalBar1dResult(result) ? result : null;
  const thermalBeamResult = isThermalBeam && props.isThermalBeam1dResult(result) ? result : null;
  const thermalFrameResult = isThermalFrame && props.isThermalFrame2dResult(result) ? result : null;
  const thermalTrussResult = isThermalTruss2d && props.isThermalTruss2dResult(result) ? result : null;
  const thermalTruss3dResult = isThermalTruss3d && props.isThermalTruss3dResult(result) ? result : null;
  const springResult = isSpring1d && props.isSpring1dResult(result) ? result : null;
  const spring2dResult = isSpring2d && props.isSpring2dResult(result) ? result : null;
  const spring3dResult = isSpring3d && props.isSpring3dResult(result) ? result : null;
  const activeSpringResult = isSpring1d ? springResult : isSpring2d ? spring2dResult : spring3dResult;
  const activeSpringModel = isSpring1d ? springModel : isSpring2d ? spring2dModel : spring3dModel;
  const trussResult = studyKind === "truss_2d" && props.isTrussResult(result) ? result : null;
  const truss3dResult = studyKind === "truss_3d" && props.isTruss3dResult(result) ? result : null;
  const beamResult = studyKind === "beam_1d" && props.isBeam1dResult(result) ? result : null;
  const activeBeamLikeResult = isThermalBeam ? thermalBeamResult : beamResult;
  const activeBeamLikeModel = isThermalBeam ? thermalBeamModel : beamModel;
  const torsionResult = isTorsion && props.isTorsion1dResult(result) ? result : null;
  const frameResult = isFrame && props.isFrame2dResult(result) ? result : null;
  const activeFrameLikeResult = isThermalFrame ? thermalFrameResult : frameResult;
  const activeFrameLikeModel = isThermalFrame ? thermalFrameModel : frameModel;
  const planeResult = !isHeatPlane && isPlane && props.isPlaneResult(result) ? result : null;
  const activePlaneInputModel = isHeatPlane ? heatPlaneModel : planeModel;
  const activeResultWindow =
    resultWindow && job?.job_id === resultWindow.jobId && studyKind === resultWindow.studyKind ? resultWindow : null;
  const trussDiagnostics = mergeWorkbenchDiagnostics(
    isTruss ? analyzeTrussModel(trussModel, t, selectedNode) : null,
    analyzeWorkbenchComponentIntegrity({
      beamModel,
      frameModel,
      heatBarModel,
      heatPlaneModel,
      planeModel,
      springModel,
      spring2dModel,
      spring3dModel,
      studyKind,
      thermalBarModel,
      thermalBeamModel,
      thermalFrameModel,
      thermalPlaneModel: planeModel,
      thermalTrussModel,
      thermalTruss3dModel,
      torsionModel,
      trussModel,
      truss3dModel,
    }, t),
  );
  const trussStability = isTruss && trussDiagnostics ? summarizeTrussStability(trussModel, trussDiagnostics) : null;
  const axialNodes = axialResult?.nodes ?? [];
  const axialElements = axialResult?.elements ?? [];
  const axialLength = axialResult?.input.length ?? axialForm.length;
  const axialScale = axialResult?.max_displacement ? 140 / axialResult.max_displacement : 1;

  const planeWindowNodes =
    activeResultWindow?.studyKind === "heat_plane_triangle_2d" ||
    activeResultWindow?.studyKind === "heat_plane_quad_2d" ||
    activeResultWindow?.studyKind === "plane_triangle_2d" ||
    activeResultWindow?.studyKind === "plane_quad_2d" ||
    activeResultWindow?.studyKind === "thermal_plane_triangle_2d" ||
    activeResultWindow?.studyKind === "thermal_plane_quad_2d"
      ? activeResultWindow.nodes
      : undefined;
  const planeWindowElements =
    activeResultWindow?.studyKind === "heat_plane_triangle_2d" ||
    activeResultWindow?.studyKind === "heat_plane_quad_2d" ||
    activeResultWindow?.studyKind === "plane_triangle_2d" ||
    activeResultWindow?.studyKind === "plane_quad_2d" ||
    activeResultWindow?.studyKind === "thermal_plane_triangle_2d" ||
    activeResultWindow?.studyKind === "thermal_plane_quad_2d"
      ? activeResultWindow.elements
      : undefined;
  const trussWindowNodes = activeResultWindow?.studyKind === "truss_2d" ? activeResultWindow.nodes : undefined;
  const trussWindowElements = activeResultWindow?.studyKind === "truss_2d" ? activeResultWindow.elements : undefined;
  const thermalTrussWindowNodes =
    activeResultWindow?.studyKind === "thermal_truss_2d" ? activeResultWindow.nodes : undefined;
  const thermalTrussWindowElements =
    activeResultWindow?.studyKind === "thermal_truss_2d" ? activeResultWindow.elements : undefined;
  const truss3dWindowNodes =
    activeResultWindow?.studyKind === "truss_3d" ||
    activeResultWindow?.studyKind === "thermal_truss_3d" ||
    activeResultWindow?.studyKind === "spring_3d"
      ? activeResultWindow.nodes
      : undefined;
  const truss3dWindowElements =
    activeResultWindow?.studyKind === "truss_3d" ||
    activeResultWindow?.studyKind === "thermal_truss_3d" ||
    activeResultWindow?.studyKind === "spring_3d"
      ? activeResultWindow.elements
      : undefined;
  const beamWindowNodes = activeResultWindow?.studyKind === "beam_1d" ? activeResultWindow.nodes : undefined;
  const beamWindowElements = activeResultWindow?.studyKind === "beam_1d" ? activeResultWindow.elements : undefined;
  const thermalBeamWindowNodes =
    activeResultWindow?.studyKind === "thermal_beam_1d" ? activeResultWindow.nodes : undefined;
  const thermalBeamWindowElements =
    activeResultWindow?.studyKind === "thermal_beam_1d" ? activeResultWindow.elements : undefined;
  const torsionWindowNodes = activeResultWindow?.studyKind === "torsion_1d" ? activeResultWindow.nodes : undefined;
  const torsionWindowElements =
    activeResultWindow?.studyKind === "torsion_1d" ? activeResultWindow.elements : undefined;
  const heatBarWindowNodes = activeResultWindow?.studyKind === "heat_bar_1d" ? activeResultWindow.nodes : undefined;
  const heatBarWindowElements =
    activeResultWindow?.studyKind === "heat_bar_1d" ? activeResultWindow.elements : undefined;
  const thermalWindowNodes =
    activeResultWindow?.studyKind === "thermal_bar_1d" ? activeResultWindow.nodes : undefined;
  const thermalWindowElements =
    activeResultWindow?.studyKind === "thermal_bar_1d" ? activeResultWindow.elements : undefined;
  const springWindowNodes = activeResultWindow?.studyKind === "spring_1d" ? activeResultWindow.nodes : undefined;
  const springWindowElements =
    activeResultWindow?.studyKind === "spring_1d" ? activeResultWindow.elements : undefined;
  const spring2dWindowNodes = activeResultWindow?.studyKind === "spring_2d" ? activeResultWindow.nodes : undefined;
  const spring2dWindowElements =
    activeResultWindow?.studyKind === "spring_2d" ? activeResultWindow.elements : undefined;
  const spring3dWindowNodes = activeResultWindow?.studyKind === "spring_3d" ? activeResultWindow.nodes : undefined;
  const spring3dWindowElements =
    activeResultWindow?.studyKind === "spring_3d" ? activeResultWindow.elements : undefined;
  const frameWindowNodes = activeResultWindow?.studyKind === "frame_2d" ? activeResultWindow.nodes : undefined;
  const frameWindowElements =
    activeResultWindow?.studyKind === "frame_2d" ? activeResultWindow.elements : undefined;
  const thermalFrameWindowNodes =
    activeResultWindow?.studyKind === "thermal_frame_2d" ? activeResultWindow.nodes : undefined;
  const thermalFrameWindowElements =
    activeResultWindow?.studyKind === "thermal_frame_2d" ? activeResultWindow.elements : undefined;

  const nextDisplayTrussNodes = isThermalFrame
    ? buildDisplayThermalFrameNodes(thermalFrameModel, thermalFrameResult, thermalFrameWindowNodes)
    : isFrame
      ? buildDisplayFrameNodes(frameModel, frameResult, frameWindowNodes)
      : isThermalBeam
        ? buildDisplayThermalBeamNodes(thermalBeamModel, thermalBeamResult, thermalBeamWindowNodes)
        : isHeatBar
          ? buildDisplayHeatBarNodes(heatBarModel, heatBarResult, heatBarWindowNodes)
          : isThermalBar
            ? buildDisplayThermalBarNodes(thermalBarModel, thermalBarResult, thermalWindowNodes)
            : isThermalTruss2d
              ? buildDisplayThermalTrussNodes(thermalTrussModel, thermalTrussResult, thermalTrussWindowNodes)
              : isSpring1d
                ? buildDisplaySpringNodes(springModel, springResult, springWindowNodes)
                : isSpring2d
                  ? buildDisplaySpring2dNodes(spring2dModel, spring2dResult, spring2dWindowNodes)
                  : isBeam
                    ? buildDisplayBeamNodes(beamModel, beamResult, beamWindowNodes)
                    : isTorsion
                      ? buildDisplayTorsionNodes(torsionModel, torsionResult, torsionWindowNodes)
                      : buildDisplayTrussNodes(trussModel, trussResult, trussWindowNodes);
  const nextDisplayTrussElements = isThermalFrame
    ? buildDisplayThermalFrameElements(thermalFrameModel, thermalFrameResult, thermalFrameWindowElements)
    : isFrame
      ? buildDisplayFrameElements(frameModel, frameResult, frameWindowElements)
      : isThermalBeam
        ? buildDisplayThermalBeamElements(thermalBeamModel, thermalBeamResult, thermalBeamWindowElements)
        : isHeatBar
          ? buildDisplayHeatBarElements(heatBarModel, heatBarResult, heatBarWindowElements)
          : isThermalBar
            ? buildDisplayThermalBarElements(thermalBarModel, thermalBarResult, thermalWindowElements)
            : isThermalTruss2d
              ? buildDisplayThermalTrussElements(thermalTrussModel, thermalTrussResult, thermalTrussWindowElements)
              : isSpring1d
                ? buildDisplaySpringElements(springModel, springResult, springWindowElements)
                : isSpring2d
                  ? buildDisplaySpring2dElements(spring2dModel, spring2dResult, spring2dWindowElements)
                  : isBeam
                    ? buildDisplayBeamElements(beamModel, beamResult, beamWindowElements)
                    : isTorsion
                      ? buildDisplayTorsionElements(torsionModel, torsionResult, torsionWindowElements)
                      : buildDisplayTrussElements(trussModel, trussResult, trussWindowElements);
  const trussBounds = getTrussBounds(nextDisplayTrussNodes);

  const displayTruss3dNodes = isSpring3d
    ? buildDisplaySpring3dNodes(spring3dModel, spring3dResult, spring3dWindowNodes)
    : isThermalTruss3d
      ? buildDisplayThermalTruss3dNodes(thermalTruss3dModel, thermalTruss3dResult, truss3dWindowNodes)
      : buildDisplayTruss3dNodes(truss3dModel, truss3dResult, truss3dWindowNodes);
  const displayTruss3dElements = isSpring3d
    ? buildDisplaySpring3dElements(spring3dModel, spring3dResult, spring3dWindowElements)
    : isThermalTruss3d
      ? buildDisplayThermalTruss3dElements(thermalTruss3dModel, thermalTruss3dResult, truss3dWindowElements)
      : buildDisplayTruss3dElements(truss3dModel, truss3dResult, truss3dWindowElements);

  const activePlaneResult = isHeatPlane
    ? (isHeatPlaneTriangle ? heatPlaneTriangleResult : heatPlaneQuadResult)
    : planeResult;
  const planeNodes =
    (planeWindowNodes ?? activePlaneResult?.nodes)?.map((node: any, index: number) => ({
      ...activePlaneInputModel.nodes[node.index ?? index],
      ...node,
      index,
      ux: typeof node.ux === "number" ? node.ux : 0,
      uy: typeof node.uy === "number" ? node.uy : 0,
      fix_x: !isHeatPlane ? activePlaneInputModel.nodes[node.index ?? index]?.fix_x ?? false : false,
      fix_y: !isHeatPlane ? activePlaneInputModel.nodes[node.index ?? index]?.fix_y ?? false : false,
      load_x: !isHeatPlane ? activePlaneInputModel.nodes[node.index ?? index]?.load_x ?? 0 : 0,
      load_y: !isHeatPlane ? activePlaneInputModel.nodes[node.index ?? index]?.load_y ?? 0 : 0,
      fix_temperature: isHeatPlane ? activePlaneInputModel.nodes[node.index ?? index]?.fix_temperature ?? false : undefined,
      temperature: isHeatPlane ? activePlaneInputModel.nodes[node.index ?? index]?.temperature ?? 0 : undefined,
      heat_load: isHeatPlane ? activePlaneInputModel.nodes[node.index ?? index]?.heat_load ?? 0 : undefined,
    })) ??
    activePlaneInputModel.nodes.map((node: any, index: number) => ({
      ...node,
      index,
      ux: 0,
      uy: 0,
      displacement_magnitude: 0,
      fix_x: !isHeatPlane && "fix_x" in node ? node.fix_x : false,
      fix_y: !isHeatPlane && "fix_y" in node ? node.fix_y : false,
      load_x: !isHeatPlane && "load_x" in node ? node.load_x : 0,
      load_y: !isHeatPlane && "load_y" in node ? node.load_y : 0,
    }));
  const planeElements =
    (planeWindowElements ?? activePlaneResult?.elements)?.map((element: any) => ({
      ...activePlaneInputModel.elements[element.index],
      ...element,
      material_id:
        "material_id" in activePlaneInputModel.elements[element.index]
          ? activePlaneInputModel.elements[element.index]?.material_id
          : undefined,
    })) ??
    activePlaneInputModel.elements.map((element: any, index: number) => ({
      ...element,
      index,
      area: 0,
      average_temperature: 0,
      temperature_gradient_x: 0,
      temperature_gradient_y: 0,
      heat_flux_x: 0,
      heat_flux_y: 0,
      heat_flux_magnitude: 0,
      strain_x: 0,
      strain_y: 0,
      average_temperature_delta: 0,
      thermal_strain: 0,
      mechanical_strain_x: 0,
      mechanical_strain_y: 0,
      total_strain_x: 0,
      total_strain_y: 0,
      gamma_xy: 0,
      stress_x: 0,
      stress_y: 0,
      tau_xy: 0,
      principal_stress_1: 0,
      principal_stress_2: 0,
      max_in_plane_shear: 0,
      von_mises: 0,
    }));
  const planeBounds = getTrussBounds(planeNodes);

  return {
    axialResult,
    heatBarResult,
    heatPlaneTriangleResult,
    heatPlaneQuadResult,
    thermalBarResult,
    thermalBeamResult,
    thermalFrameResult,
    thermalTrussResult,
    thermalTruss3dResult,
    springResult,
    spring2dResult,
    spring3dResult,
    activeSpringResult,
    activeSpringModel,
    trussResult,
    truss3dResult,
    beamResult,
    activeBeamLikeResult,
    activeBeamLikeModel,
    torsionResult,
    frameResult,
    activeFrameLikeResult,
    activeFrameLikeModel,
    planeResult,
    activePlaneInputModel,
    activeResultWindow,
    trussDiagnostics,
    trussStability,
    axialNodes,
    axialElements,
    axialLength,
    axialScale,
    planeWindowNodes,
    planeWindowElements,
    trussWindowNodes,
    trussWindowElements,
    thermalTrussWindowNodes,
    thermalTrussWindowElements,
    truss3dWindowNodes,
    truss3dWindowElements,
    beamWindowNodes,
    beamWindowElements,
    thermalBeamWindowNodes,
    thermalBeamWindowElements,
    torsionWindowNodes,
    torsionWindowElements,
    heatBarWindowNodes,
    heatBarWindowElements,
    thermalWindowNodes,
    thermalWindowElements,
    springWindowNodes,
    springWindowElements,
    spring2dWindowNodes,
    spring2dWindowElements,
    spring3dWindowNodes,
    spring3dWindowElements,
    frameWindowNodes,
    frameWindowElements,
    thermalFrameWindowNodes,
    thermalFrameWindowElements,
    displayTrussNodes: nextDisplayTrussNodes,
    displayTrussElements: nextDisplayTrussElements,
    trussBounds,
    displayTruss3dNodes,
    displayTruss3dElements,
    planeNodes,
    planeElements,
    planeBounds,
  };
}
