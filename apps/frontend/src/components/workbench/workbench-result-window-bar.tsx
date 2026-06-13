"use client";

import type { WorkbenchCopy } from "./workbench-copy";
import type { PlaneResultField } from "./workbench-defaults";
import type { ViewportRenderStrategy } from "./workbench-render-diagnostics";
import {
  prioritizedBeamFields,
  prioritizedFrameFields,
  prioritizedPlaneFields,
} from "./workbench-result-field-priority";
import type { ResultWindowState } from "./workbench-result-window-controller";
import type { BeamResultField, FrameResultField, StudyKind } from "./workbench-types";

type ResultWindowJump = {
  label: string;
  offset: number;
};

type WorkbenchResultWindowBarProps = {
  t: WorkbenchCopy;
  activeResultWindow: ResultWindowState | null;
  resultWindowStart: number;
  resultWindowEnd: number;
  activeResultWindowLimit: number;
  resultWindowOffset: number;
  resultWindowMaxTotal: number;
  resultWindowJumps: ResultWindowJump[];
  isPlane: boolean;
  isHeatPlane: boolean;
  isHeatPlaneTriangle: boolean;
  isThermalPlaneTriangle: boolean;
  isThermalPlaneQuad: boolean;
  isFrameLike: boolean;
  isThermalFrame: boolean;
  isBeam: boolean;
  isTorsion: boolean;
  studyKind: string;
  planeResult: unknown;
  activeFrameLikeResult: unknown;
  activeBeamLikeResult: unknown;
  torsionResult: unknown;
  planeResultField: PlaneResultField;
  frameResultField: FrameResultField;
  beamResultField: BeamResultField;
  renderStrategy: ViewportRenderStrategy;
  setPlaneResultField: (value: PlaneResultField) => void;
  setFrameResultField: (value: FrameResultField) => void;
  setBeamResultField: (value: BeamResultField) => void;
  setResultWindowOffset: (updater: number | ((current: number) => number)) => void;
  clampChunkOffset: (offset: number, total: number, limit: number) => number;
};

const planeFieldLabels: Record<PlaneResultField, (t: WorkbenchCopy) => string> = {
  von_mises: (t) => t.planeViewVonMises,
  principal_stress_1: (t) => t.planeViewPrincipal1,
  max_in_plane_shear: (t) => t.planeViewMaxShear,
  average_potential: (t) => t.averagePotential,
  potential_gradient_x: (t) => `${t.potentialGradient} X`,
  potential_gradient_y: (t) => `${t.potentialGradient} Y`,
  electric_field_x: (t) => `${t.electricField} X`,
  electric_field_y: (t) => `${t.electricField} Y`,
  electric_field_magnitude: (t) => t.electricFieldMagnitude,
  electric_flux_density_x: (t) => `${t.electricFluxDensity} X`,
  electric_flux_density_y: (t) => `${t.electricFluxDensity} Y`,
  electric_flux_density_magnitude: (t) => t.electricFluxDensityMagnitude,
  average_temperature: (t) => t.maxTemperature,
  average_temperature_delta: (t) => t.temperatureDelta,
  temperature_gradient_x: (t) => t.temperatureGradientX,
  temperature_gradient_y: (t) => t.temperatureGradientY,
  heat_flux_x: (t) => t.heatFluxX,
  heat_flux_y: (t) => t.heatFluxY,
  heat_flux_magnitude: (t) => t.maxHeatFlux,
  thermal_strain: (t) => t.thermalStrain,
  mechanical_strain: (t) => t.mechanicalStrain,
};

const frameFieldLabels: Record<FrameResultField, (t: WorkbenchCopy) => string> = {
  axial_stress: (t) => t.stress,
  max_bending_stress: (t) => t.bendingStress,
  max_combined_stress: (t) => t.combinedStress,
  moment: (t) => t.maxMoment,
  average_temperature_delta: (t) => t.temperatureDelta,
  temperature_gradient_y: (t) => t.temperatureGradientY,
  thermal_curvature: (t) => t.thermalCurvature,
};

const beamFieldLabels: Record<BeamResultField, (t: WorkbenchCopy) => string> = {
  max_bending_stress: (t) => t.bendingStress,
  shear_force: (t) => t.shearForce,
  moment: (t) => t.maxMoment,
  temperature_gradient_y: (t) => t.temperatureGradientY,
  thermal_curvature: (t) => t.thermalCurvature,
};

export function WorkbenchResultWindowBar({
  t,
  activeResultWindow,
  resultWindowStart,
  resultWindowEnd,
  activeResultWindowLimit,
  resultWindowOffset,
  resultWindowMaxTotal,
  resultWindowJumps,
  isPlane,
  isHeatPlane,
  isHeatPlaneTriangle,
  isThermalPlaneTriangle,
  isThermalPlaneQuad,
  isFrameLike,
  isThermalFrame,
  isBeam,
  isTorsion,
  studyKind,
  planeResult,
  activeFrameLikeResult,
  activeBeamLikeResult,
  torsionResult,
  planeResultField,
  frameResultField,
  beamResultField,
  renderStrategy,
  setPlaneResultField,
  setFrameResultField,
  setBeamResultField,
  setResultWindowOffset,
  clampChunkOffset,
}: WorkbenchResultWindowBarProps) {
  if (!activeResultWindow) return null;

  const focusOnly = renderStrategy === "focus";
  const planeFields = prioritizedPlaneFields(studyKind as StudyKind, renderStrategy);
  const frameFields = prioritizedFrameFields(studyKind as StudyKind, renderStrategy);
  const beamFields = prioritizedBeamFields(studyKind as StudyKind, renderStrategy);

  return (
    <div className="viewport-window-bar">
      <div className="viewport-window-bar__meta">
        <strong>{t.resultWindow}</strong>
        <span>
          {t.pageRange}: {resultWindowStart}-{resultWindowEnd}
        </span>
        <span>
          {t.chunkSize}: {activeResultWindowLimit}
        </span>
        <span>
          {t.nodes}: {activeResultWindow.totalNodes}
        </span>
        <span>
          {t.totalElements}: {activeResultWindow.totalElements}
        </span>
        {focusOnly ? <span>{t.renderFocusFieldHint}</span> : null}
      </div>
      <div className="button-row">
        {isPlane && planeResult ? (
          <>
            {planeFields.map((field) => (
              <button
                key={field}
                className={`ghost-button ghost-button--compact${planeResultField === field ? " ghost-button--active" : ""}`}
                onClick={() => setPlaneResultField(field)}
                type="button"
              >
                {planeFieldLabels[field](t)}
              </button>
            ))}
          </>
        ) : null}
        {isFrameLike && activeFrameLikeResult ? (
          <>
            {frameFields.map((field) => (
              <button
                key={field}
                className={`ghost-button ghost-button--compact${frameResultField === field ? " ghost-button--active" : ""}`}
                onClick={() => setFrameResultField(field)}
                type="button"
              >
                {studyKind === "torsion_1d" && field === "max_bending_stress" ? t.torsionStress : frameFieldLabels[field](t)}
              </button>
            ))}
          </>
        ) : null}
        {isBeam && activeBeamLikeResult ? (
          <>
            {beamFields.map((field) => (
              <button
                key={field}
                className={`ghost-button ghost-button--compact${beamResultField === field ? " ghost-button--active" : ""}`}
                onClick={() => setBeamResultField(field)}
                type="button"
              >
                {beamFieldLabels[field](t)}
              </button>
            ))}
          </>
        ) : null}
        {isTorsion && torsionResult ? (
          <>
            {frameFields.map((field) => (
              <button
                key={field}
                className={`ghost-button ghost-button--compact${frameResultField === field ? " ghost-button--active" : ""}`}
                onClick={() => setFrameResultField(field)}
                type="button"
              >
                {field === "max_bending_stress" ? t.torsionStress : t.maxTorque}
              </button>
            ))}
          </>
        ) : null}
        {resultWindowJumps.map((jump) => (
          <button
            key={jump.label}
            className="ghost-button ghost-button--compact"
            disabled={resultWindowOffset === jump.offset}
            onClick={() => setResultWindowOffset(jump.offset)}
            type="button"
          >
            {jump.label}
          </button>
        ))}
        <button
          className="ghost-button ghost-button--compact"
          disabled={resultWindowOffset <= 0}
          onClick={() =>
            setResultWindowOffset((current) =>
              clampChunkOffset(current - activeResultWindowLimit, resultWindowMaxTotal, activeResultWindowLimit),
            )
          }
          type="button"
        >
          {t.previousPage}
        </button>
        <button
          className="ghost-button ghost-button--compact"
          disabled={resultWindowOffset + activeResultWindowLimit >= resultWindowMaxTotal}
          onClick={() =>
            setResultWindowOffset((current) =>
              clampChunkOffset(current + activeResultWindowLimit, resultWindowMaxTotal, activeResultWindowLimit),
            )
          }
          type="button"
        >
          {t.nextPage}
        </button>
      </div>
    </div>
  );
}
