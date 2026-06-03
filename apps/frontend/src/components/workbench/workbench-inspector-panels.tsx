"use client";

import { VirtualList } from "@/components/ui/virtual-list";
import type {
  ActionsPage,
  FrameElementSelection,
  FrameForceSort,
  InspectorLabels,
  JobLike,
  PlaneHeatSort,
  ResultPage,
  StudyKind,
} from "./workbench-inspector-types";

type ResultPanelsProps = {
  t: InspectorLabels;
  resultPage: ResultPage;
  studyKind: StudyKind;
  job: JobLike | null;
  nodeCount: number;
  tipDisplacement: string;
  maxStressValue: string;
  frameMaxAxialForceValue?: string;
  frameMaxShearForceValue?: string;
  reactionValue: string;
  frameMaxRotationValue?: string;
  thermalFrameMaxTemperatureDeltaValue?: string;
  thermalFrameMaxTemperatureGradientValue?: string;
  thermalBeamMaxTemperatureGradientValue?: string;
  thermalPlaneMaxTemperatureDeltaValue?: string;
  reportScopeLabel?: string;
  reportScopeHint?: string;
};

type ActionsExportPanelProps = {
  t: InspectorLabels;
  actionsPage: ActionsPage;
  studyKind: StudyKind;
  job: JobLike | null;
  tipDisplacement: string;
  maxStressValue: string;
  frameMaxAxialForceValue?: string;
  frameMaxShearForceValue?: string;
  reactionValue: string;
  frameMaxRotationValue?: string;
  thermalFrameMaxTemperatureDeltaValue?: string;
  thermalFrameMaxTemperatureGradientValue?: string;
  thermalBeamMaxTemperatureGradientValue?: string;
  thermalPlaneMaxTemperatureDeltaValue?: string;
  reportScopeLabel?: string;
  reportScopeHint?: string;
  createdAtValue: string;
  updatedAtValue: string;
  heartbeatStatusValue: string;
  heartbeatTone: "healthy" | "quiet" | "stale";
  failureReasonValue: string;
  canCancelJob: boolean;
  onCancelJob: () => void;
  onDownloadJson: () => void;
  onDownloadCsv: () => void;
  canProjectHeatToThermo?: boolean;
  projectHeatToThermoLabel?: string;
  onProjectHeatToThermo?: () => void;
  planeHotspotFieldLabel?: string;
  planeHotspotElements: Array<{ id: string; value: string; index: number; active?: boolean; summary?: string }>;
  planeThermalRows: Array<{
    id: string;
    index: number;
    active?: boolean;
    sortTemperature: number;
    sortGradient: number;
    sortFlux: number;
    averageTemperature: string;
    temperatureGradientX: string;
    temperatureGradientY: string;
    heatFluxX: string;
    heatFluxY: string;
    heatFluxMagnitude: string;
  }>;
  frameHotspotFieldLabel?: string;
  frameHotspotElements: Array<{ id: string; value: string; index: number; active?: boolean; summary?: string }>;
  frameForceRows: Array<{
    id: string;
    index: number;
    active?: boolean;
    sortAxial: number;
    sortShear: number;
    sortMoment: number;
    axialForceI: string;
    shearForceI: string;
    momentI: string;
    axialForceJ: string;
    shearForceJ: string;
    momentJ: string;
  }>;
  planeHotspotLimit: number;
  onDownloadPlaneHotspots: () => void;
  onDownloadFrameHotspots: () => void;
  onDownloadFrameForces: () => void;
  onSelectPlaneHotspot: (index: number) => void;
  onSelectFrameHotspot: (index: number) => void;
  onPlaneHotspotLimitChange: (limit: number) => void;
  selectedFrameElementData: FrameElementSelection | null;
  frameForceSort: FrameForceSort;
  onFrameForceSortChange: (sort: FrameForceSort) => void;
  planeHeatSort: PlaneHeatSort;
  onPlaneHeatSortChange: (sort: PlaneHeatSort) => void;
};

function isHeatPlaneStudy(kind: StudyKind) {
  return kind === "heat_plane_triangle_2d" || kind === "heat_plane_quad_2d";
}

function isThermalPlaneStudy(kind: StudyKind) {
  return kind === "thermal_plane_triangle_2d" || kind === "thermal_plane_quad_2d";
}

function isFrameStudy(kind: StudyKind) {
  return kind === "frame_2d" || kind === "thermal_frame_2d";
}

function isSpringStudy(kind: StudyKind) {
  return kind === "spring_1d" || kind === "spring_2d" || kind === "spring_3d";
}

function isBeamStudy(kind: StudyKind) {
  return kind === "beam_1d" || kind === "thermal_beam_1d";
}

function isTorsionStudy(kind: StudyKind) {
  return kind === "torsion_1d";
}

function isThermalAxialStudy(kind: StudyKind) {
  return kind === "thermal_bar_1d" || kind === "thermal_truss_2d" || kind === "thermal_truss_3d";
}

function isPlaneLikeStudy(kind: StudyKind) {
  return (
    isHeatPlaneStudy(kind) ||
    kind === "plane_triangle_2d" ||
    kind === "plane_quad_2d" ||
    isThermalPlaneStudy(kind)
  );
}


export function WorkbenchInspectorResultPanels({
  t,
  resultPage,
  studyKind,
  job,
  nodeCount,
  tipDisplacement,
  maxStressValue,
  frameMaxAxialForceValue,
  frameMaxShearForceValue,
  reactionValue,
  frameMaxRotationValue,
  thermalFrameMaxTemperatureDeltaValue,
  thermalFrameMaxTemperatureGradientValue,
  thermalBeamMaxTemperatureGradientValue,
  thermalPlaneMaxTemperatureDeltaValue,
  reportScopeLabel,
  reportScopeHint,
}: ResultPanelsProps) {
  const isHeatPlane = isHeatPlaneStudy(studyKind);
  const isHeatBar = studyKind === "heat_bar_1d";
  const isFrame = isFrameStudy(studyKind);
  const isSpring = isSpringStudy(studyKind);
  const isBeam = isBeamStudy(studyKind);
  const isTorsion = isTorsionStudy(studyKind);
  const isThermal = isThermalAxialStudy(studyKind);

  return (
    <>
      {resultPage === "summary" ? (
        <section className="info-card">
          <h3>{t.status}</h3>
          <div className="metric-grid">
            <div><span>{t.status}</span><strong>{job?.status ?? "--"}</strong></div>
            <div><span>{t.worker}</span><strong>{job?.worker_id ?? "--"}</strong></div>
            <div><span>{t.progress}</span><strong>{typeof job?.progress === "number" ? `${Math.round(job.progress * 100)}%` : "--"}</strong></div>
            <div><span>{t.iteration}</span><strong>{job?.iteration ?? "--"}</strong></div>
            <div><span>{t.residual}</span><strong>{typeof job?.residual === "number" ? job.residual.toExponential(3) : "--"}</strong></div>
            <div><span>{t.nodes}</span><strong>{nodeCount}</strong></div>
          </div>
        </section>
      ) : null}
      {resultPage === "details" ? (
        <section className="info-card">
          <h3>{t.result}</h3>
          {reportScopeLabel || reportScopeHint ? (
            <p className="card-copy">
              {[reportScopeLabel, reportScopeHint].filter(Boolean).join(" · ")}
            </p>
          ) : null}
          <div className="metric-grid">
            <div><span>{isHeatPlane ? t.maxTemperature : t.tipDisp}</span><strong>{tipDisplacement}</strong></div>
            <div><span>{isHeatPlane ? t.maxHeatFlux : t.maxStress}</span><strong>{maxStressValue}</strong></div>
            {isThermalPlaneStudy(studyKind) ? <div><span>{t.temperatureDelta}</span><strong>{thermalPlaneMaxTemperatureDeltaValue ?? "--"}</strong></div> : null}
            {isHeatBar ? <div><span>{t.maxTemperature}</span><strong>{tipDisplacement}</strong></div> : null}
            {isFrame || isSpring || studyKind === "thermal_bar_1d" || studyKind === "thermal_truss_2d" || studyKind === "thermal_truss_3d" ? <div><span>{t.maxAxialForce}</span><strong>{frameMaxAxialForceValue ?? "--"}</strong></div> : null}
            {isFrame || isBeam ? <div><span>{t.maxShearForce}</span><strong>{frameMaxShearForceValue ?? "--"}</strong></div> : null}
            {studyKind === "thermal_frame_2d" ? <div><span>{t.temperatureDelta}</span><strong>{thermalFrameMaxTemperatureDeltaValue ?? "--"}</strong></div> : null}
            {studyKind === "thermal_frame_2d" ? <div><span>{t.temperatureGradientY}</span><strong>{thermalFrameMaxTemperatureGradientValue ?? "--"}</strong></div> : null}
            {studyKind === "thermal_beam_1d" ? <div><span>{t.temperatureGradientY}</span><strong>{thermalBeamMaxTemperatureGradientValue ?? "--"}</strong></div> : null}
            <div><span>{isHeatBar ? t.maxHeatFlux : t.reaction}</span><strong>{reactionValue}</strong></div>
            {isFrame || isBeam || isTorsion ? <div><span>{t.maxRotation}</span><strong>{frameMaxRotationValue ?? "--"}</strong></div> : null}
          </div>
        </section>
      ) : null}
    </>
  );
}

export function WorkbenchInspectorActionsExportPanel({
  t,
  actionsPage,
  studyKind,
  job,
  tipDisplacement,
  maxStressValue,
  frameMaxAxialForceValue,
  frameMaxShearForceValue,
  reactionValue,
  frameMaxRotationValue,
  thermalFrameMaxTemperatureDeltaValue,
  thermalFrameMaxTemperatureGradientValue,
  thermalBeamMaxTemperatureGradientValue,
  thermalPlaneMaxTemperatureDeltaValue,
  reportScopeLabel,
  reportScopeHint,
  createdAtValue,
  updatedAtValue,
  heartbeatStatusValue,
  heartbeatTone,
  failureReasonValue,
  canCancelJob,
  onCancelJob,
  onDownloadJson,
  onDownloadCsv,
  canProjectHeatToThermo,
  projectHeatToThermoLabel,
  onProjectHeatToThermo,
  planeHotspotFieldLabel,
  planeHotspotElements,
  planeThermalRows,
  frameHotspotFieldLabel,
  frameHotspotElements,
  frameForceRows,
  planeHotspotLimit,
  onDownloadPlaneHotspots,
  onDownloadFrameHotspots,
  onDownloadFrameForces,
  onSelectPlaneHotspot,
  onSelectFrameHotspot,
  onPlaneHotspotLimitChange,
  selectedFrameElementData,
  frameForceSort,
  onFrameForceSortChange,
  planeHeatSort,
  onPlaneHeatSortChange,
}: ActionsExportPanelProps) {
  if (actionsPage !== "exports") {
    return null;
  }

  const isHeatPlane = isHeatPlaneStudy(studyKind);
  const isThermalPlane = isThermalPlaneStudy(studyKind);
  const isPlane = isPlaneLikeStudy(studyKind);
  const isHeatBar = studyKind === "heat_bar_1d";
  const isFrame = isFrameStudy(studyKind);
  const isSpring = isSpringStudy(studyKind);
  const isBeam = isBeamStudy(studyKind);
  const isTorsion = isTorsionStudy(studyKind);
  const isThermal = isThermalAxialStudy(studyKind);
  const sortedFrameForceRows =
    frameForceSort === "axial"
      ? [...frameForceRows].sort((left, right) => right.sortAxial - left.sortAxial)
      : frameForceSort === "shear"
        ? [...frameForceRows].sort((left, right) => right.sortShear - left.sortShear)
        : frameForceSort === "moment"
          ? [...frameForceRows].sort((left, right) => right.sortMoment - left.sortMoment)
          : frameForceRows;
  const sortedPlaneThermalRows =
    planeHeatSort === "temperature"
      ? [...planeThermalRows].sort((left, right) => right.sortTemperature - left.sortTemperature)
      : planeHeatSort === "gradient"
        ? [...planeThermalRows].sort((left, right) => right.sortGradient - left.sortGradient)
        : planeHeatSort === "flux"
          ? [...planeThermalRows].sort((left, right) => right.sortFlux - left.sortFlux)
          : planeThermalRows;

  return (
    <section className="info-card">
      <h3>{t.actions}</h3>
      {reportScopeLabel || reportScopeHint ? (
        <p className="card-copy">
          {[reportScopeLabel, reportScopeHint].filter(Boolean).join(" · ")}
        </p>
      ) : null}
      <div className="button-row">
        <button className="ghost-button" disabled={!canCancelJob} onClick={onCancelJob} type="button">{t.cancelJob}</button>
        <button className="ghost-button" onClick={onDownloadJson} type="button">{t.exportData} {t.exportJson}</button>
        <button className="ghost-button" onClick={onDownloadCsv} type="button">{t.exportData} {t.exportCsv}</button>
        {canProjectHeatToThermo && onProjectHeatToThermo && projectHeatToThermoLabel ? (
          <button className="ghost-button" onClick={onProjectHeatToThermo} type="button">{projectHeatToThermoLabel}</button>
        ) : null}
      </div>
      <div className="metric-grid">
        <div><span>{t.status}</span><strong>{job?.status ?? "--"}</strong></div>
        <div><span>{t.worker}</span><strong>{job?.worker_id ?? "--"}</strong></div>
        <div><span>{isHeatPlane ? t.maxTemperature : t.tipDisp}</span><strong>{tipDisplacement}</strong></div>
        <div><span>{isHeatPlane ? t.maxHeatFlux : t.maxStress}</span><strong>{maxStressValue}</strong></div>
        {isThermalPlane ? <div><span>{t.temperatureDelta}</span><strong>{thermalPlaneMaxTemperatureDeltaValue ?? "--"}</strong></div> : null}
        {isHeatBar ? <div><span>{t.maxTemperature}</span><strong>{tipDisplacement}</strong></div> : null}
        {isFrame || isSpring || studyKind === "thermal_bar_1d" || studyKind === "thermal_truss_2d" || studyKind === "thermal_truss_3d" ? <div><span>{t.maxAxialForce}</span><strong>{frameMaxAxialForceValue ?? "--"}</strong></div> : null}
        {isFrame || isBeam ? <div><span>{t.maxShearForce}</span><strong>{frameMaxShearForceValue ?? "--"}</strong></div> : null}
        {studyKind === "thermal_frame_2d" ? <div><span>{t.temperatureDelta}</span><strong>{thermalFrameMaxTemperatureDeltaValue ?? "--"}</strong></div> : null}
        {studyKind === "thermal_frame_2d" ? <div><span>{t.temperatureGradientY}</span><strong>{thermalFrameMaxTemperatureGradientValue ?? "--"}</strong></div> : null}
        {studyKind === "thermal_beam_1d" ? <div><span>{t.temperatureGradientY}</span><strong>{thermalBeamMaxTemperatureGradientValue ?? "--"}</strong></div> : null}
        <div><span>{isHeatBar ? t.maxHeatFlux : t.reaction}</span><strong>{reactionValue}</strong></div>
        {isFrame || isBeam || isTorsion ? <div><span>{t.maxRotation}</span><strong>{frameMaxRotationValue ?? "--"}</strong></div> : null}
        <div><span>{t.createdAt}</span><strong>{createdAtValue}</strong></div>
        <div><span>{t.updatedAt}</span><strong>{updatedAtValue}</strong></div>
        <div><span>{t.lastHeartbeat}</span><strong>{updatedAtValue}</strong></div>
        <div><span>{t.heartbeatStatus}</span><strong><span className={`heartbeat-badge heartbeat-badge--${heartbeatTone}`}>{heartbeatStatusValue}</span></strong></div>
        <div><span>{t.hasResult}</span><strong>{job?.has_result ? t.yes : t.no}</strong></div>
        <div><span>{t.failureReason}</span><strong>{failureReasonValue}</strong></div>
      </div>
      {isPlane ? (
        <div className="diagnostic-list">
          <div className="diagnostic-item">
            <strong>{t.currentField}: {planeHotspotFieldLabel ?? "--"}</strong>
          </div>
          <div className="button-row">
            <span className="card-copy">{t.topN}</span>
            {[3, 5, 10].map((limit) => (
              <button
                key={limit}
                className={`ghost-button ghost-button--compact${planeHotspotLimit === limit ? " ghost-button--active" : ""}`}
                onClick={() => onPlaneHotspotLimitChange(limit)}
                type="button"
              >
                {limit}
              </button>
            ))}
            <button className="ghost-button ghost-button--compact" onClick={onDownloadPlaneHotspots} type="button">
              {t.exportHotspots}
            </button>
          </div>
          {planeHotspotElements.length > 0 ? (
            <>
              <p className="card-copy">{t.planeHotspots}</p>
              {planeHotspotElements.map((entry) => (
                <button
                  key={entry.id}
                  className={`history-item${entry.active ? " history-item--active" : ""}`}
                  onClick={() => onSelectPlaneHotspot(entry.index)}
                  type="button"
                >
                  <strong>{entry.id}</strong>
                  <small>{entry.value}</small>
                  {entry.summary ? <small>{entry.summary}</small> : null}
                </button>
              ))}
            </>
          ) : (
            <p className="card-copy">--</p>
          )}
          {isHeatPlane && planeThermalRows.length > 0 ? (
            <div className="table-like table-like--console">
              <h4>{t.elementHeatTable}</h4>
              <div className="button-row">
                <span className="card-copy">{t.sortBy}</span>
                <button className={`ghost-button ghost-button--compact${planeHeatSort === "index" ? " ghost-button--active" : ""}`} onClick={() => onPlaneHeatSortChange("index")} type="button">#</button>
                <button className={`ghost-button ghost-button--compact${planeHeatSort === "temperature" ? " ghost-button--active" : ""}`} onClick={() => onPlaneHeatSortChange("temperature")} type="button">{t.averageTemperature}</button>
                <button className={`ghost-button ghost-button--compact${planeHeatSort === "gradient" ? " ghost-button--active" : ""}`} onClick={() => onPlaneHeatSortChange("gradient")} type="button">{t.temperatureGradientY}</button>
                <button className={`ghost-button ghost-button--compact${planeHeatSort === "flux" ? " ghost-button--active" : ""}`} onClick={() => onPlaneHeatSortChange("flux")} type="button">{t.maxHeatFlux}</button>
              </div>
              <div className="table-like__head table-like__head--frame-forces">
                <span>#</span>
                <span>{t.averageTemperature}</span>
                <span>{t.temperatureGradientX}</span>
                <span>{t.temperatureGradientY}</span>
                <span>{t.heatFluxX}</span>
                <span>{t.heatFluxY}</span>
                <span>{t.maxHeatFlux}</span>
              </div>
              <VirtualList
                className="table-like__body"
                items={sortedPlaneThermalRows}
                itemHeight={46}
                maxHeight={240}
                itemKey={(entry) => `${entry.id}-${entry.index}`}
                renderItem={(entry) => (
                  <button
                    className={`table-like__row table-like__row--frame-forces${entry.active ? " history-item--active" : ""}`}
                    onClick={() => onSelectPlaneHotspot(entry.index)}
                    type="button"
                  >
                    <strong>{entry.id}</strong>
                    <span>{entry.averageTemperature}</span>
                    <span>{entry.temperatureGradientX}</span>
                    <span>{entry.temperatureGradientY}</span>
                    <span>{entry.heatFluxX}</span>
                    <span>{entry.heatFluxY}</span>
                    <span>{entry.heatFluxMagnitude}</span>
                  </button>
                )}
              />
            </div>
          ) : null}
        </div>
      ) : isFrame || isBeam || isTorsion || isSpring ? (
        <div className="diagnostic-list">
          <div className="diagnostic-item">
            <strong>{t.currentField}: {frameHotspotFieldLabel ?? "--"}</strong>
          </div>
          <div className="button-row">
            <span className="card-copy">{t.topN}</span>
            {[3, 5, 10].map((limit) => (
              <button
                key={limit}
                className={`ghost-button ghost-button--compact${planeHotspotLimit === limit ? " ghost-button--active" : ""}`}
                onClick={() => onPlaneHotspotLimitChange(limit)}
                type="button"
              >
                {limit}
              </button>
            ))}
            <button className="ghost-button ghost-button--compact" onClick={onDownloadFrameHotspots} type="button">
              {t.exportHotspots}
            </button>
          </div>
          {frameHotspotElements.length > 0 ? (
            <>
              <p className="card-copy">{t.frameElements}</p>
              {frameHotspotElements.map((entry) => (
                <button
                  key={entry.id}
                  className={`history-item${entry.active ? " history-item--active" : ""}`}
                  onClick={() => onSelectFrameHotspot(entry.index)}
                  type="button"
                >
                  <strong>{entry.id}</strong>
                  <small>{entry.value}</small>
                  {entry.summary ? <small>{entry.summary}</small> : null}
                </button>
              ))}
            </>
          ) : (
            <p className="card-copy">--</p>
          )}
          {selectedFrameElementData ? (
            <>
              <p className="card-copy">{t.memberEndForces}</p>
              <div className="metric-grid">
                {!isBeam && !isTorsion ? <div><span>{t.forceI}</span><strong>{typeof selectedFrameElementData.axial_force_i === "number" ? selectedFrameElementData.axial_force_i.toExponential(3) : "--"}</strong></div> : null}
                {!isSpring && !isThermal && !isTorsion ? <div><span>{t.shearI}</span><strong>{typeof selectedFrameElementData.shear_force_i === "number" ? selectedFrameElementData.shear_force_i.toExponential(3) : "--"}</strong></div> : null}
                {!isSpring && !isThermal ? <div><span>{t.momentI}</span><strong>{typeof selectedFrameElementData.moment_i === "number" ? selectedFrameElementData.moment_i.toExponential(3) : "--"}</strong></div> : null}
                {!isBeam && !isTorsion ? <div><span>{t.forceJ}</span><strong>{typeof selectedFrameElementData.axial_force_j === "number" ? selectedFrameElementData.axial_force_j.toExponential(3) : "--"}</strong></div> : null}
                {!isSpring && !isThermal && !isTorsion ? <div><span>{t.shearJ}</span><strong>{typeof selectedFrameElementData.shear_force_j === "number" ? selectedFrameElementData.shear_force_j.toExponential(3) : "--"}</strong></div> : null}
                {!isSpring && !isThermal ? <div><span>{t.momentJ}</span><strong>{typeof selectedFrameElementData.moment_j === "number" ? selectedFrameElementData.moment_j.toExponential(3) : "--"}</strong></div> : null}
              </div>
            </>
          ) : null}
          {frameForceRows.length > 0 ? (
            <div className="table-like table-like--console">
              <h4>{t.memberForceTable}</h4>
              <div className="button-row">
                <span className="card-copy">{t.sortBy}</span>
                <button className={`ghost-button ghost-button--compact${frameForceSort === "index" ? " ghost-button--active" : ""}`} onClick={() => onFrameForceSortChange("index")} type="button">#</button>
                {!isBeam && !isTorsion ? <button className={`ghost-button ghost-button--compact${frameForceSort === "axial" ? " ghost-button--active" : ""}`} onClick={() => onFrameForceSortChange("axial")} type="button">{t.axialForce}</button> : null}
                {!isSpring && !isTorsion ? <button className={`ghost-button ghost-button--compact${frameForceSort === "shear" ? " ghost-button--active" : ""}`} onClick={() => onFrameForceSortChange("shear")} type="button">{t.shearForce}</button> : null}
                {!isSpring ? <button className={`ghost-button ghost-button--compact${frameForceSort === "moment" ? " ghost-button--active" : ""}`} onClick={() => onFrameForceSortChange("moment")} type="button">{t.maxMoment}</button> : null}
                <button className="ghost-button ghost-button--compact" onClick={onDownloadFrameForces} type="button">{t.exportMemberForces}</button>
              </div>
              <div className="table-like__head table-like__head--frame-forces">
                <span>#</span>
                {!isBeam && !isTorsion ? <span>{t.forceI}</span> : null}
                {!isSpring && !isTorsion ? <span>{t.shearI}</span> : null}
                {!isSpring ? <span>{t.momentI}</span> : null}
                {!isBeam && !isTorsion ? <span>{t.forceJ}</span> : null}
                {!isSpring && !isTorsion ? <span>{t.shearJ}</span> : null}
                {!isSpring ? <span>{t.momentJ}</span> : null}
              </div>
              <VirtualList
                className="table-like__body"
                items={sortedFrameForceRows}
                itemHeight={46}
                maxHeight={240}
                itemKey={(entry) => `${entry.id}-${entry.index}`}
                renderItem={(entry) => (
                  <button
                    className={`table-like__row table-like__row--frame-forces${entry.active ? " history-item--active" : ""}`}
                    onClick={() => onSelectFrameHotspot(entry.index)}
                    type="button"
                  >
                    <strong>{entry.id}</strong>
                    {!isBeam && !isTorsion ? <span>{entry.axialForceI}</span> : null}
                    {!isSpring && !isTorsion ? <span>{entry.shearForceI}</span> : null}
                    {!isSpring ? <span>{entry.momentI}</span> : null}
                    {!isBeam && !isTorsion ? <span>{entry.axialForceJ}</span> : null}
                    {!isSpring && !isTorsion ? <span>{entry.shearForceJ}</span> : null}
                    {!isSpring ? <span>{entry.momentJ}</span> : null}
                  </button>
                )}
              />
            </div>
          ) : null}
        </div>
      ) : null}
    </section>
  );
}
