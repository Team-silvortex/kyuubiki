"use client";

import { useMemo } from "react";

import { buildWorkbenchStudySidebarData } from "@/components/workbench/workbench-study-sidebar-data";
import { createStudyKindResetHandlers } from "@/components/workbench/workbench-study-kind-controller";
import {
  buildStudyDomainOptions,
  buildStudyKindOptionGroups,
  classifyStudyKindFamily,
} from "@/lib/workbench/view-models";

export function useWorkbenchStudyShellController(props: Record<string, any>) {
  const studyKindOptionGroups = buildStudyKindOptionGroups({
    kinds: props.t.kinds,
    domains: props.t.studyDomains,
    families: props.t.studyFamilies,
  });
  const studyDomainOptions = buildStudyDomainOptions(props.t.studyDomains);
  const currentStudyFamily = classifyStudyKindFamily(props.workspaceState.studyKind);
  const currentStudyFamilyLabel = props.t.studyFamilies[currentStudyFamily];
  const currentStudyFamilyHint = props.t.familyHints[currentStudyFamily];

  const { thermalIntentValue, thermalBoundaryValue, studySummaryRows, studyControlsRows, truss3dTreeRows } =
    buildWorkbenchStudySidebarData({
      t: props.t,
      language: props.shellState.language,
      studyKind: props.workspaceState.studyKind,
      loadedModelName: props.workspaceState.loadedModelName,
      activeMaterial: props.workspaceState.activeMaterial,
      localMaterialLabel: props.localMaterialLabel,
      fixed: props.fixed,
      isAxial: props.studyResultDerived.isAxial,
      isSpring: props.studyResultDerived.isSpring,
      isSpring1d: props.studyResultDerived.isSpring1d,
      isSpring2d: props.studyResultDerived.isSpring2d,
      isSpring3d: props.studyResultDerived.isSpring3d,
      isBeam: props.studyResultDerived.isBeam,
      isTorsion: props.studyResultDerived.isTorsion,
      isTruss: props.studyResultDerived.isTruss,
      isTruss3d: props.studyResultDerived.isTruss3d,
      isFrameLike: props.studyResultDerived.isFrameLike,
      isFrame: props.studyResultDerived.isFrame,
      isPlane: props.studyResultDerived.isPlane,
      isHeatBar: props.studyResultDerived.isHeatBar,
      isHeatPlane: props.studyResultDerived.isHeatPlane,
      isHeatPlaneTriangle: props.studyResultDerived.isHeatPlaneTriangle,
      isHeatPlaneQuad: props.studyResultDerived.isHeatPlaneQuad,
      isThermal: props.studyResultDerived.isThermal,
      isThermalBar: props.studyResultDerived.isThermalBar,
      isThermalBeam: props.studyResultDerived.isThermalBeam,
      isThermalFrame: props.studyResultDerived.isThermalFrame,
      isThermalTruss2d: props.studyResultDerived.isThermalTruss2d,
      isThermalPlaneTriangle: props.studyResultDerived.isThermalPlaneTriangle,
      isThermalPlaneQuad: props.studyResultDerived.isThermalPlaneQuad,
      axialForm: props.workspaceState.axialForm,
      heatBarModel: props.workspaceState.heatBarModel,
      heatPlaneModel: props.workspaceState.heatPlaneModel,
      thermalBarModel: props.workspaceState.thermalBarModel,
      thermalBeamModel: props.workspaceState.thermalBeamModel,
      thermalFrameModel: props.workspaceState.thermalFrameModel,
      thermalTrussModel: props.workspaceState.thermalTrussModel,
      thermalTruss3dModel: props.workspaceState.thermalTruss3dModel,
      springModel: props.workspaceState.springModel,
      spring2dModel: props.workspaceState.spring2dModel,
      spring3dModel: props.workspaceState.spring3dModel,
      beamModel: props.workspaceState.beamModel,
      torsionModel: props.workspaceState.torsionModel,
      trussModel: props.workspaceState.trussModel,
      truss3dModel: props.workspaceState.truss3dModel,
      frameModel: props.workspaceState.frameModel,
      activePlaneInputModel: props.studyResultDerived.activePlaneInputModel,
      activeFrameLikeModel: props.studyResultDerived.activeFrameLikeModel,
      displayTruss3dElements: props.studyResultDerived.displayTruss3dElements,
      truss3dTreeNodes: props.studyResultDerived.isSpring3d
        ? props.workspaceState.spring3dModel.nodes
        : props.workspaceState.truss3dModel.nodes,
      selectedNode: props.workspaceState.selectedNode,
      selectedTruss3dNodes: props.workspaceState.selectedTruss3dNodes,
      memberDraftNodes: props.workspaceState.memberDraftNodes,
    });

  const studyKindResetHandlers = useMemo(
    () =>
      createStudyKindResetHandlers({
        activeMaterial: props.workspaceState.activeMaterial,
        setPlaneModel: props.workspaceState.setPlaneModel,
        setHeatBarModel: props.workspaceState.setHeatBarModel,
        setHeatPlaneModel: props.workspaceState.setHeatPlaneModel,
        setThermalBarModel: props.workspaceState.setThermalBarModel,
        setThermalBeamModel: props.workspaceState.setThermalBeamModel,
        setThermalFrameModel: props.workspaceState.setThermalFrameModel,
        setThermalTrussModel: props.workspaceState.setThermalTrussModel,
        setThermalTruss3dModel: props.workspaceState.setThermalTruss3dModel,
        setSpringModel: props.workspaceState.setSpringModel,
        setSpring2dModel: props.workspaceState.setSpring2dModel,
        setSpring3dModel: props.workspaceState.setSpring3dModel,
        setBeamModel: props.workspaceState.setBeamModel,
        setTorsionModel: props.workspaceState.setTorsionModel,
        setFrameModel: props.workspaceState.setFrameModel,
        setPlaneResultField: props.workspaceState.setPlaneResultField,
        ensurePlaneModelMaterials: props.ensurePlaneModelMaterials,
        ensureBeamModelMaterials: props.ensureBeamModelMaterials,
        ensureFrameModelMaterials: props.ensureFrameModelMaterials,
        defaultPlaneQuad: props.defaultPlaneQuad,
        defaultThermalPlaneQuad: props.defaultThermalPlaneQuad,
        defaultPlaneTriangle: props.defaultPlaneTriangle,
        defaultThermalPlaneTriangle: props.defaultThermalPlaneTriangle,
        defaultHeatBar1d: props.defaultHeatBar1d,
        defaultHeatPlaneQuad: props.defaultHeatPlaneQuad,
        defaultHeatPlaneTriangle: props.defaultHeatPlaneTriangle,
        defaultThermalBar1d: props.defaultThermalBar1d,
        defaultThermalBeam1d: props.defaultThermalBeam1d,
        defaultThermalFrame2d: props.defaultThermalFrame2d,
        defaultThermalTruss2d: props.defaultThermalTruss2d,
        defaultThermalTruss3d: props.defaultThermalTruss3d,
        defaultSpring1d: props.defaultSpring1d,
        defaultSpring2d: props.defaultSpring2d,
        defaultSpring3d: props.defaultSpring3d,
        defaultBeam1d: props.defaultBeam1d,
        defaultTorsion1d: props.defaultTorsion1d,
        defaultFrame2d: props.defaultFrame2d,
      }),
    [props.workspaceState.activeMaterial],
  );

  const canProjectHeatToThermo =
    (props.workspaceState.studyKind === "heat_bar_1d" && Boolean(props.studyResultDerived.heatBarResult)) ||
    (props.workspaceState.studyKind === "heat_plane_triangle_2d" &&
      Boolean(props.studyResultDerived.heatPlaneTriangleResult)) ||
    (props.workspaceState.studyKind === "heat_plane_quad_2d" && Boolean(props.studyResultDerived.heatPlaneQuadResult));

  const projectHeatToThermoStudy = () => {
    if (props.workspaceState.studyKind === "heat_bar_1d" && props.studyResultDerived.heatBarResult) {
      props.recordHistory(props.t.projectHeatToThermoAction);
      props.resetActiveResult();
      props.workspaceState.setThermalBarModel(
        props.buildThermalBarFromHeatResult(
          props.workspaceState.heatBarModel,
          props.studyResultDerived.heatBarResult,
          props.workspaceState.thermalBarModel,
        ),
      );
      props.workspaceState.setStudyKind("thermal_bar_1d");
      props.openWorkspaceStudy("controls");
      props.workspaceState.setMessage(props.t.projectedHeatToThermo);
      return "thermal_bar_1d" as const;
    }

    if (props.workspaceState.studyKind === "heat_plane_triangle_2d" && props.studyResultDerived.heatPlaneTriangleResult) {
      props.recordHistory(props.t.projectHeatToThermoAction);
      props.resetActiveResult();
      props.workspaceState.setPlaneModel(
        props.buildThermalPlaneTriangleFromHeatResult(
          props.workspaceState.heatPlaneModel,
          props.studyResultDerived.heatPlaneTriangleResult,
          props.workspaceState.planeModel,
          props.workspaceState.activeMaterial,
        ),
      );
      props.workspaceState.setPlaneResultField("average_temperature_delta");
      props.workspaceState.setStudyKind("thermal_plane_triangle_2d");
      props.openWorkspaceStudy("controls");
      props.workspaceState.setMessage(props.t.projectedHeatToThermo);
      return "thermal_plane_triangle_2d" as const;
    }

    if (props.workspaceState.studyKind === "heat_plane_quad_2d" && props.studyResultDerived.heatPlaneQuadResult) {
      props.recordHistory(props.t.projectHeatToThermoAction);
      props.resetActiveResult();
      props.workspaceState.setPlaneModel(
        props.buildThermalPlaneQuadFromHeatResult(
          props.workspaceState.heatPlaneModel,
          props.studyResultDerived.heatPlaneQuadResult,
          props.workspaceState.planeModel,
          props.workspaceState.activeMaterial,
        ),
      );
      props.workspaceState.setPlaneResultField("average_temperature_delta");
      props.workspaceState.setStudyKind("thermal_plane_quad_2d");
      props.openWorkspaceStudy("controls");
      props.workspaceState.setMessage(props.t.projectedHeatToThermo);
      return "thermal_plane_quad_2d" as const;
    }

    return null;
  };

  const handleLanguageChange = (nextLanguage: typeof props.shellState.language) => {
    props.shellState.applyLanguagePreference(nextLanguage);
  };

  return {
    canProjectHeatToThermo,
    currentStudyFamilyHint,
    currentStudyFamilyLabel,
    handleLanguageChange,
    projectHeatToThermoStudy,
    studyControlsRows,
    studyDomainOptions,
    studyKindOptionGroups,
    studyKindResetHandlers,
    studySummaryRows,
    thermalBoundaryValue,
    thermalIntentValue,
    truss3dTreeRows,
  };
}
