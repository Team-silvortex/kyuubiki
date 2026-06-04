"use client";

import type { WorkbenchCopy } from "./workbench-copy";
import type { PlaneResultField } from "./workbench-defaults";
import type { ResultWindowState } from "./workbench-result-window-controller";
import type { BeamResultField, FrameResultField } from "./workbench-types";

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
  setPlaneResultField: (value: PlaneResultField) => void;
  setFrameResultField: (value: FrameResultField) => void;
  setBeamResultField: (value: BeamResultField) => void;
  setResultWindowOffset: (updater: number | ((current: number) => number)) => void;
  clampChunkOffset: (offset: number, total: number, limit: number) => number;
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
  setPlaneResultField,
  setFrameResultField,
  setBeamResultField,
  setResultWindowOffset,
  clampChunkOffset,
}: WorkbenchResultWindowBarProps) {
  if (!activeResultWindow) return null;

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
      </div>
      <div className="button-row">
        {isPlane && planeResult ? (
          <>
            {isHeatPlane ? (
              <>
                <button className={`ghost-button ghost-button--compact${planeResultField === "average_temperature" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("average_temperature")} type="button">
                  {t.maxTemperature}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "temperature_gradient_x" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("temperature_gradient_x")} type="button">
                  {t.temperatureGradientX}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "temperature_gradient_y" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("temperature_gradient_y")} type="button">
                  {t.temperatureGradientY}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "heat_flux_x" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("heat_flux_x")} type="button">
                  {t.heatFluxX}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "heat_flux_y" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("heat_flux_y")} type="button">
                  {t.heatFluxY}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "heat_flux_magnitude" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("heat_flux_magnitude")} type="button">
                  {t.maxHeatFlux}
                </button>
              </>
            ) : (
              <>
                <button className={`ghost-button ghost-button--compact${planeResultField === "von_mises" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("von_mises")} type="button">
                  {t.planeViewVonMises}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "principal_stress_1" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("principal_stress_1")} type="button">
                  {t.planeViewPrincipal1}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "max_in_plane_shear" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("max_in_plane_shear")} type="button">
                  {t.planeViewMaxShear}
                </button>
              </>
            )}
            {isThermalPlaneTriangle || isThermalPlaneQuad ? (
              <>
                <button className={`ghost-button ghost-button--compact${planeResultField === "average_temperature_delta" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("average_temperature_delta")} type="button">
                  {t.temperatureDelta}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "thermal_strain" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("thermal_strain")} type="button">
                  {t.thermalStrain}
                </button>
                <button className={`ghost-button ghost-button--compact${planeResultField === "mechanical_strain" ? " ghost-button--active" : ""}`} onClick={() => setPlaneResultField("mechanical_strain")} type="button">
                  {t.mechanicalStrain}
                </button>
              </>
            ) : null}
          </>
        ) : null}
        {isFrameLike && activeFrameLikeResult ? (
          <>
            <button className={`ghost-button ghost-button--compact${frameResultField === "axial_stress" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("axial_stress")} type="button">
              {t.stress}
            </button>
            <button className={`ghost-button ghost-button--compact${frameResultField === "max_bending_stress" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("max_bending_stress")} type="button">
              {t.bendingStress}
            </button>
            <button className={`ghost-button ghost-button--compact${frameResultField === "max_combined_stress" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("max_combined_stress")} type="button">
              {t.combinedStress}
            </button>
            <button className={`ghost-button ghost-button--compact${frameResultField === "moment" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("moment")} type="button">
              {t.maxMoment}
            </button>
            {isThermalFrame ? (
              <>
                <button className={`ghost-button ghost-button--compact${frameResultField === "average_temperature_delta" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("average_temperature_delta")} type="button">
                  {t.temperatureDelta}
                </button>
                <button className={`ghost-button ghost-button--compact${frameResultField === "temperature_gradient_y" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("temperature_gradient_y")} type="button">
                  {t.temperatureGradientY}
                </button>
                <button className={`ghost-button ghost-button--compact${frameResultField === "thermal_curvature" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("thermal_curvature")} type="button">
                  {t.thermalCurvature}
                </button>
              </>
            ) : null}
          </>
        ) : null}
        {isBeam && activeBeamLikeResult ? (
          <>
            <button className={`ghost-button ghost-button--compact${beamResultField === "max_bending_stress" ? " ghost-button--active" : ""}`} onClick={() => setBeamResultField("max_bending_stress")} type="button">
              {t.bendingStress}
            </button>
            <button className={`ghost-button ghost-button--compact${beamResultField === "shear_force" ? " ghost-button--active" : ""}`} onClick={() => setBeamResultField("shear_force")} type="button">
              {t.shearForce}
            </button>
            <button className={`ghost-button ghost-button--compact${beamResultField === "moment" ? " ghost-button--active" : ""}`} onClick={() => setBeamResultField("moment")} type="button">
              {t.maxMoment}
            </button>
            {studyKind === "thermal_beam_1d" ? (
              <>
                <button className={`ghost-button ghost-button--compact${beamResultField === "temperature_gradient_y" ? " ghost-button--active" : ""}`} onClick={() => setBeamResultField("temperature_gradient_y")} type="button">
                  {t.temperatureGradientY}
                </button>
                <button className={`ghost-button ghost-button--compact${beamResultField === "thermal_curvature" ? " ghost-button--active" : ""}`} onClick={() => setBeamResultField("thermal_curvature")} type="button">
                  {t.thermalCurvature}
                </button>
              </>
            ) : null}
          </>
        ) : null}
        {isTorsion && torsionResult ? (
          <>
            <button className={`ghost-button ghost-button--compact${frameResultField === "max_bending_stress" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("max_bending_stress")} type="button">
              {t.torsionStress}
            </button>
            <button className={`ghost-button ghost-button--compact${frameResultField === "moment" ? " ghost-button--active" : ""}`} onClick={() => setFrameResultField("moment")} type="button">
              {t.maxTorque}
            </button>
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
