"use client";

import { scientific } from "@/lib/workbench/helpers";

export function buildWorkbenchWorkflowLabels(t: Record<string, any>) {
  return {
    sectionTitle: t.sections.workflow,
    overviewPageLabel: t.workflowOverviewPage,
    catalogPageLabel: t.workflowCatalogPage,
    builderPageLabel: t.workflowBuilderPage,
    runsPageLabel: t.workflowRunsPage,
    overviewHint: t.workflowOverviewHint,
    catalogHint: t.workflowCatalogHint,
    builderHint: t.workflowBuilderHint,
    runsHint: t.workflowRunsHint,
    catalogTitle: t.workflowCatalogTitle,
    refreshLabel: t.workflowCatalogRefresh,
    runLabel: t.workflowCatalogRun,
    emptyCatalogLabel: t.workflowCatalogEmpty,
    noSelectionLabel: t.workflowNoSelection,
    nodesTitle: t.workflowNodesTitle,
    edgesTitle: t.workflowEdgesTitle,
    entryInputsTitle: t.workflowEntryInputsTitle,
    outputArtifactsTitle: t.workflowOutputArtifactsTitle,
    datasetContractTitle: t.workflowDatasetContractTitle,
    datasetValuesTitle: t.workflowDatasetValuesTitle,
    datasetValueLabel: t.workflowDatasetValueLabel,
    datasetSemanticTypeLabel: t.workflowDatasetSemanticTypeLabel,
    datasetEncodingLabel: t.workflowDatasetEncodingLabel,
    datasetShapeLabel: t.workflowDatasetShapeLabel,
    datasetAxesLabel: t.workflowDatasetAxesLabel,
    datasetSchemaLabel: t.workflowDatasetSchemaLabel,
    datasetClassLabel: t.workflowDatasetClassLabel,
    datasetNoneLabel: t.workflowDatasetNoneLabel,
    datasetDraftHint: t.workflowDatasetDraftHint,
    datasetEditorTitle: t.workflowDatasetEditorTitle,
    datasetValueSelectLabel: t.workflowDatasetValueSelectLabel,
    datasetUnitLabel: t.workflowDatasetUnitLabel,
    datasetMetadataLabel: t.workflowDatasetMetadataLabel,
    datasetPortMappingsTitle: t.workflowDatasetPortMappingsTitle,
    datasetEdgeMappingsTitle: t.workflowDatasetEdgeMappingsTitle,
    datasetDraftLocalLabel: t.workflowDatasetDraftLocalLabel,
    datasetUnassignedLabel: t.workflowDatasetUnassignedLabel,
    importGraphLabel: t.workflowImportGraphLabel,
    importDatasetContractLabel: t.workflowImportDatasetContractLabel,
    importSuccessLabel: t.workflowImportSuccessLabel,
    importInvalidGraphLabel: t.workflowImportInvalidGraphLabel,
    importInvalidDatasetLabel: t.workflowImportInvalidDatasetLabel,
    exportGraphLabel: t.workflowExportGraphLabel,
    exportDatasetContractLabel: t.workflowExportDatasetContractLabel,
    operatorLabel: t.workflowOperatorLabel,
    kindLabel: t.workflowKindLabel,
    progressLabel: t.workflowProgressLabel,
    currentNodeLabel: t.workflowCurrentNodeLabel,
    latestSummaryLabel: t.workflowLatestSummaryLabel,
    openRunLabel: t.workflowOpenRunLabel,
    emptyRunsLabel: t.workflowRunsEmpty,
    selectForBuilderLabel: t.workflowSelectForBuilder,
    statusReadyLabel: t.ready,
    statusBusyLabel: t.busy,
  };
}

export function buildWorkbenchMainShellSummaryProps(props: Record<string, any>) {
  const { studyResultDerived, workspaceState } = props;
  return {
    tipDisplacement: studyResultDerived.isAxial
      ? scientific(studyResultDerived.axialResult?.tip_displacement)
      : studyResultDerived.isHeatBar
        ? scientific(studyResultDerived.heatBarResult?.max_temperature)
        : studyResultDerived.isHeatPlane
          ? scientific(
              studyResultDerived.isHeatPlaneTriangle
                ? studyResultDerived.heatPlaneTriangleResult?.max_temperature
                : studyResultDerived.heatPlaneQuadResult?.max_temperature,
            )
          : studyResultDerived.isThermalTruss2d
            ? scientific(studyResultDerived.thermalTrussResult?.max_displacement)
            : workspaceState.studyKind === "thermal_truss_3d"
              ? scientific(studyResultDerived.thermalTruss3dResult?.max_displacement)
              : studyResultDerived.isTruss
                ? scientific(studyResultDerived.trussResult?.max_displacement)
                : studyResultDerived.isSpring3d
                  ? scientific(studyResultDerived.spring3dResult?.max_displacement)
                  : workspaceState.studyKind === "truss_3d"
                    ? scientific(studyResultDerived.truss3dResult?.max_displacement)
                    : studyResultDerived.isThermalBar
                      ? scientific(studyResultDerived.thermalBarResult?.max_displacement)
                      : studyResultDerived.isSpring
                        ? scientific(studyResultDerived.activeSpringResult?.max_displacement)
                        : studyResultDerived.isBeam
                          ? scientific(studyResultDerived.activeBeamLikeResult?.max_displacement)
                          : studyResultDerived.isTorsion
                            ? scientific(studyResultDerived.torsionResult?.max_rotation)
                            : studyResultDerived.isFrameLike
                              ? scientific(studyResultDerived.activeFrameLikeResult?.max_displacement)
                              : scientific(studyResultDerived.planeResult?.max_displacement),
    maxStressValue: scientific(
      studyResultDerived.isAxial
        ? studyResultDerived.axialResult?.max_stress
        : studyResultDerived.isHeatBar
          ? studyResultDerived.heatBarResult?.max_heat_flux
          : studyResultDerived.isHeatPlane
            ? studyResultDerived.isHeatPlaneTriangle
              ? studyResultDerived.heatPlaneTriangleResult?.max_heat_flux
              : studyResultDerived.heatPlaneQuadResult?.max_heat_flux
            : studyResultDerived.isThermalTruss2d
              ? studyResultDerived.thermalTrussResult?.max_stress
              : workspaceState.studyKind === "thermal_truss_3d"
                ? studyResultDerived.thermalTruss3dResult?.max_stress
                : studyResultDerived.isTruss
                  ? studyResultDerived.trussResult?.max_stress
                  : studyResultDerived.isSpring3d
                    ? studyResultDerived.spring3dResult?.max_force
                    : workspaceState.studyKind === "truss_3d"
                      ? studyResultDerived.truss3dResult?.max_stress
                      : studyResultDerived.isThermalBar
                        ? studyResultDerived.thermalBarResult?.max_stress
                        : studyResultDerived.isSpring
                          ? studyResultDerived.activeSpringResult?.max_force
                          : studyResultDerived.isBeam
                            ? studyResultDerived.activeBeamLikeResult?.max_stress
                            : studyResultDerived.isTorsion
                              ? studyResultDerived.torsionResult?.max_stress
                              : studyResultDerived.isFrameLike
                                ? studyResultDerived.activeFrameLikeResult?.max_stress
                                : studyResultDerived.planeResult?.max_stress,
    ),
    reactionValue: studyResultDerived.isAxial
      ? scientific(studyResultDerived.axialResult?.reaction_force)
      : studyResultDerived.isHeatBar
        ? scientific(studyResultDerived.heatBarResult?.max_heat_flux)
        : studyResultDerived.isThermalBar
          ? scientific(studyResultDerived.thermalBarResult?.max_axial_force)
          : studyResultDerived.isThermalTruss2d
            ? scientific(studyResultDerived.thermalTrussResult?.max_axial_force)
            : studyResultDerived.isThermalTruss3d
              ? scientific(studyResultDerived.thermalTruss3dResult?.max_axial_force)
              : studyResultDerived.isSpring
                ? scientific(studyResultDerived.activeSpringResult?.max_force)
                : studyResultDerived.isTorsion
                  ? scientific(studyResultDerived.torsionResult?.max_torque)
                  : studyResultDerived.isFrameLike
                    ? scientific(studyResultDerived.activeFrameLikeResult?.max_moment)
                    : studyResultDerived.isBeam
                      ? scientific(studyResultDerived.activeBeamLikeResult?.max_moment)
                      : "--",
    frameMaxRotationValue: studyResultDerived.isFrameLike
      ? scientific(studyResultDerived.activeFrameLikeResult?.max_rotation)
      : studyResultDerived.isBeam
        ? scientific(studyResultDerived.activeBeamLikeResult?.max_rotation)
        : studyResultDerived.isTorsion
          ? scientific(studyResultDerived.torsionResult?.max_rotation)
          : undefined,
    thermalPlaneMaxTemperatureDelta:
      studyResultDerived.planeResult && "max_temperature_delta" in studyResultDerived.planeResult
        ? studyResultDerived.planeResult.max_temperature_delta
        : undefined,
  };
}
