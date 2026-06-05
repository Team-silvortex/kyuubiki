"use client";

import {
  buildWorkbenchHotspotData,
  buildWorkbenchSecurityUi,
  buildWorkbenchSelectionData,
} from "@/components/workbench/workbench-inspector-derived";
import { lineResultFieldValue, planeResultFieldValue } from "@/components/workbench/workbench-result-helpers";

export function buildWorkbenchResultDerivedData(props: Record<string, any>) {
  const {
    t,
    isHeatPlane,
    isSpring,
    isThermalBar,
    isThermalTruss2d,
    isThermalTruss3d,
    isBeam,
    isTorsion,
    isFrameLike,
    isThermalFrame,
    isThermalBeam,
    planeElements,
    planeResultField,
    displayTrussElements,
    activeLineResultField,
    selectedElement,
    planeHotspotLimit,
    selectedNode,
    displayTrussNodes,
    displayTruss3dNodes,
    displayTruss3dElements,
    planeNodes,
    thermalFrameModel,
    thermalFrameResult,
    frameModel,
    frameResult,
    thermalBeamModel,
    thermalBeamResult,
    beamModel,
    beamResult,
    torsionModel,
    torsionResult,
    isHeatBar,
    heatBarModel,
    heatBarResult,
    thermalBarModel,
    thermalBarResult,
    activeSpringModel,
    trussDiagnostics,
    language,
  } = props;

  const planeResultFieldMax = Math.max(
    ...planeElements.map((element: any) => planeResultFieldValue(element, planeResultField)),
    0,
  );

  const frameResultFieldMax = Math.max(
    ...displayTrussElements.map((element: any) => lineResultFieldValue(element, activeLineResultField)),
    0,
  );

  const planeResultFieldLabel =
    planeResultField === "average_temperature"
      ? t.maxTemperature
      : planeResultField === "average_temperature_delta"
        ? t.temperatureDelta
        : planeResultField === "temperature_gradient_x"
          ? t.temperatureGradientX
          : planeResultField === "temperature_gradient_y"
            ? t.temperatureGradientY
            : planeResultField === "heat_flux_x"
              ? t.heatFluxX
              : planeResultField === "heat_flux_y"
                ? t.heatFluxY
                : planeResultField === "heat_flux_magnitude"
                  ? t.maxHeatFlux
                  : planeResultField === "thermal_strain"
                    ? t.thermalStrain
                    : planeResultField === "mechanical_strain"
                      ? t.mechanicalStrain
                      : planeResultField === "principal_stress_1"
                        ? t.planeViewPrincipal1
                        : planeResultField === "max_in_plane_shear"
                          ? t.planeViewMaxShear
                          : t.planeViewVonMises;

  const planeLegendText =
    planeResultField === "average_temperature"
      ? `${t.maxTemperature} · ${t.planeResultLegend}`
      : planeResultField === "average_temperature_delta"
        ? `${t.temperatureDelta} · ${t.planeResultLegend}`
        : planeResultField === "temperature_gradient_x"
          ? `${t.temperatureGradientX} · ${t.planeResultLegend}`
          : planeResultField === "temperature_gradient_y"
            ? `${t.temperatureGradientY} · ${t.planeResultLegend}`
            : planeResultField === "heat_flux_x"
              ? `${t.heatFluxX} · ${t.planeResultLegend}`
              : planeResultField === "heat_flux_y"
                ? `${t.heatFluxY} · ${t.planeResultLegend}`
                : planeResultField === "heat_flux_magnitude"
                  ? `${t.maxHeatFlux} · ${t.planeResultLegend}`
                  : planeResultField === "thermal_strain"
                    ? `${t.thermalStrain} · ${t.planeResultLegend}`
                    : planeResultField === "mechanical_strain"
                      ? `${t.mechanicalStrain} · ${t.planeResultLegend}`
                      : planeResultField === "principal_stress_1"
                        ? `${t.planeViewPrincipal1} · ${t.planeResultLegend}`
                        : planeResultField === "max_in_plane_shear"
                          ? `${t.planeViewMaxShear} · ${t.planeResultLegend}`
                          : `${t.planeViewVonMises} · ${t.planeResultLegend}`;

  const frameResultFieldLabel =
    activeLineResultField === "average_temperature_delta"
      ? t.temperatureDelta
      : activeLineResultField === "temperature_gradient_y"
        ? t.temperatureGradientY
        : activeLineResultField === "thermal_curvature"
          ? t.thermalCurvature
          : activeLineResultField === "axial_stress"
            ? isSpring || isThermalBar || isThermalTruss2d || isThermalTruss3d
              ? t.axialForce
              : t.stress
            : activeLineResultField === "shear_force"
              ? t.shearForce
              : activeLineResultField === "max_bending_stress"
                ? isTorsion
                  ? t.torsionStress
                  : t.bendingStress
                : activeLineResultField === "moment"
                  ? isTorsion
                    ? t.maxTorque
                    : t.maxMoment
                  : t.combinedStress;

  const frameLegendText = `${frameResultFieldLabel} · ${t.planeResultLegend}`;

  const frameTreeValueLabel =
    activeLineResultField === "average_temperature_delta"
      ? t.temperatureDelta
      : activeLineResultField === "temperature_gradient_y"
        ? t.temperatureGradientY
        : activeLineResultField === "thermal_curvature"
          ? t.thermalCurvature
          : activeLineResultField === "axial_stress"
            ? isSpring || isThermalBar || isThermalTruss2d || isThermalTruss3d
              ? t.axialForce
              : t.stress
            : activeLineResultField === "shear_force"
              ? t.shearForce
              : activeLineResultField === "max_bending_stress"
                ? isTorsion
                  ? t.torsionStress
                  : t.bendingStress
                : activeLineResultField === "moment"
                  ? isTorsion
                    ? t.maxTorque
                    : t.maxMoment
                  : t.combinedStress;

  const {
    planeHotspotElements,
    planeThermalRows,
    frameHotspotElements,
    frameForceRows,
    frameMaxAxialForce,
    frameMaxShearForce,
  } = buildWorkbenchHotspotData({
    isHeatPlane,
    isThermalFrame,
    isSpring,
    isThermalBar,
    isThermalTruss2d,
    isThermalTruss3d,
    isTorsion,
    isBeam,
    planeElements,
    planeResultField,
    displayTrussElements,
    activeLineResultField,
    selectedElement,
    planeHotspotLimit,
  });

  const selectionData = buildWorkbenchSelectionData({
    selectedNode,
    selectedElement,
    displayTrussNodes,
    displayTrussElements,
    displayTruss3dNodes,
    displayTruss3dElements,
    planeNodes,
    planeElements,
    isThermalFrame,
    thermalFrameModel,
    thermalFrameResult,
    frameModel,
    frameResult,
    isThermalBeam,
    thermalBeamModel,
    thermalBeamResult,
    beamModel,
    beamResult,
    torsionModel,
    torsionResult,
    isHeatBar,
    heatBarModel,
    heatBarResult,
    thermalBarModel,
    thermalBarResult,
    activeSpringModel,
  });

  const selectedNodeIssues =
    selectedNode !== null && trussDiagnostics ? trussDiagnostics.nodeIssues[selectedNode] ?? [] : [];

  const securityUi = buildWorkbenchSecurityUi(language);

  return {
    planeResultFieldMax,
    frameResultFieldMax,
    planeResultFieldLabel,
    planeLegendText,
    frameResultFieldLabel,
    frameLegendText,
    frameTreeValueLabel,
    planeHotspotElements,
    planeThermalRows,
    frameHotspotElements,
    frameForceRows,
    frameMaxAxialForce,
    frameMaxShearForce,
    ...selectionData,
    selectedNodeIssues,
    securityUi,
  };
}
