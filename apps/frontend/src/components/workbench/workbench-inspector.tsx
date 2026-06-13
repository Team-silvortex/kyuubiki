"use client";

import { memo, useState } from "react";
import {
  WorkbenchInspectorActionsExportPanel,
  WorkbenchInspectorResultPanels,
} from "./workbench-inspector-panels";
import {
  WorkbenchInspectorDiagnosticsPanel,
  WorkbenchInspectorHistoryPanel,
  WorkbenchInspectorTabChrome,
} from "./workbench-inspector-chrome";
import type {
  ActionsPage,
  FrameForceSort,
  InspectorTab,
  PlaneHeatSort,
  ResultPage,
  StatusPage,
  WorkbenchInspectorProps,
} from "./workbench-inspector-types";

function formatInspectorMetric(value: number | undefined) {
  return typeof value === "number" ? value.toExponential(3) : "--";
}

function WorkbenchInspectorInner({
  t,
  reportScopeLabel,
  reportScopeHint,
  sidebarSection,
  studyKind,
  isPending,
  selectedNodeData,
  selectedElementData,
  selectedTruss3dNodeData,
  selectedTruss3dElementData,
  selectedPlaneNodeData,
  selectedPlaneElementData,
  selectedFrameNodeData,
  selectedFrameElementData,
  trussElementArea,
  trussElementModulusGpa,
  trussElementMaterialId,
  truss3dElementArea,
  truss3dElementModulusGpa,
  truss3dElementMaterialId,
  planeElementThickness,
  planeElementModulusGpa,
  planeElementPoissonRatio,
  planeElementMaterialId,
  frameElementMaterialId,
  materialOptions,
  materialLabel,
  onUpdateSelectedNode,
  onUpdateSelectedElement,
  onAssignSelectedElementMaterial,
  onUpdateSelectedTruss3dNode,
  onUpdateSelectedTruss3dElement,
  onAssignSelectedTruss3dElementMaterial,
  onUpdateSelectedPlaneNode,
  onUpdateSelectedPlaneElement,
  onAssignSelectedPlaneElementMaterial,
  onUpdateSelectedFrameNode,
  onUpdateSelectedFrameElement,
  onAssignSelectedFrameElementMaterial,
  trussDiagnostics,
  trussStability,
  hotspotNodeLabels,
  onApplyTrussSuggestion,
  undoStack,
  redoStack,
  onUndo,
  onRedo,
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
  planeHotspotFieldLabel,
  planeHotspotElements,
  planeThermalRows,
  frameHotspotFieldLabel,
  frameHotspotElements,
  frameForceRows,
  planeHotspotLimit,
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
  onDownloadPlaneHotspots,
  onDownloadFrameHotspots,
  onDownloadFrameForces,
  onSelectPlaneHotspot,
  onSelectFrameHotspot,
  onPlaneHotspotLimitChange,
}: WorkbenchInspectorProps) {
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("status");
  const [statusPage, setStatusPage] = useState<StatusPage>("properties");
  const [actionsPage, setActionsPage] = useState<ActionsPage>("exports");
  const [resultPage, setResultPage] = useState<ResultPage>("summary");
  const [frameForceSort, setFrameForceSort] = useState<FrameForceSort>("index");
  const [planeHeatSort, setPlaneHeatSort] = useState<PlaneHeatSort>("index");
  const isTruss = studyKind === "truss_2d" || studyKind === "thermal_truss_2d";
  const isTruss3d = studyKind === "truss_3d" || studyKind === "thermal_truss_3d";
  const isSpring3d = studyKind === "spring_3d";
  const isElectrostaticPlane =
    studyKind === "electrostatic_plane_triangle_2d" || studyKind === "electrostatic_plane_quad_2d";
  const isHeatPlane = studyKind === "heat_plane_triangle_2d" || studyKind === "heat_plane_quad_2d";
  const isThermalPlane = studyKind === "thermal_plane_triangle_2d" || studyKind === "thermal_plane_quad_2d";
  const isPlane =
    isHeatPlane || isElectrostaticPlane || studyKind === "plane_triangle_2d" || studyKind === "plane_quad_2d" || isThermalPlane;
  const isHeatBar = studyKind === "heat_bar_1d";
  const isThermal = studyKind === "thermal_bar_1d" || studyKind === "thermal_truss_2d" || studyKind === "thermal_truss_3d";
  const isSpring = studyKind === "spring_1d" || studyKind === "spring_2d" || studyKind === "spring_3d";
  const isBeam = studyKind === "beam_1d" || studyKind === "thermal_beam_1d";
  const isTorsion = studyKind === "torsion_1d";
  const isFrame = studyKind === "frame_2d" || studyKind === "thermal_frame_2d";
  const hasPlaneElectrostaticNode = typeof selectedPlaneNodeData?.potential === "number" || typeof selectedPlaneNodeData?.charge_density === "number";
  const hasPlaneElectrostaticElement =
    typeof selectedPlaneElementData?.average_potential === "number" ||
    typeof selectedPlaneElementData?.electric_field_magnitude === "number" ||
    typeof selectedPlaneElementData?.electric_flux_density_magnitude === "number";

  return (
    <aside
      className="workspace-inspector panel"
      data-workbench-panel="inspector"
      data-workbench-surface="built-in"
    >
      <div className="panel-head">
        <h2>{t.overview}</h2>
        <span>{isPending ? t.busy : t.ready}</span>
      </div>
      <div className="inspector-stack panel-scroll-window">
        <div className="panel-tabs panel-tabs--wide">
          <button className={`panel-tab${inspectorTab === "status" ? " panel-tab--active" : ""}`} onClick={() => setInspectorTab("status")} type="button">{t.status}</button>
          <button className={`panel-tab${inspectorTab === "result" ? " panel-tab--active" : ""}`} onClick={() => setInspectorTab("result")} type="button">{t.result}</button>
          <button className={`panel-tab${inspectorTab === "actions" ? " panel-tab--active" : ""}`} onClick={() => setInspectorTab("actions")} type="button">{t.actions}</button>
        </div>
        <WorkbenchInspectorTabChrome
          t={t}
          inspectorTab={inspectorTab}
          statusPage={statusPage}
          actionsPage={actionsPage}
          resultPage={resultPage}
          onStatusPageChange={setStatusPage}
          onActionsPageChange={setActionsPage}
          onResultPageChange={setResultPage}
        />
        {sidebarSection === "model" && inspectorTab === "status" && statusPage === "properties" ? (
          <section className="info-card">
            <h3>{t.properties}</h3>
            {isTruss && selectedNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedNodeData.x} onChange={(event) => onUpdateSelectedNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedNodeData.y} onChange={(event) => onUpdateSelectedNode("y", Number(event.target.value))} /></label>
                <label><span>{t.loadX}</span><input type="number" step={100} value={selectedNodeData.load_x} onChange={(event) => onUpdateSelectedNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.loadY}</span><input type="number" step={100} value={selectedNodeData.load_y} onChange={(event) => onUpdateSelectedNode("load_y", Number(event.target.value))} /></label>
                <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedNodeData.fix_x} onChange={(event) => onUpdateSelectedNode("fix_x", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedNodeData.fix_y} onChange={(event) => onUpdateSelectedNode("fix_y", event.target.checked)} /></label>
              </div>
            ) : isTruss && selectedElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedElementData.node_j} readOnly /></label>
                <label>
                  <span>{materialLabel}</span>
                  <select value={trussElementMaterialId} onChange={(event) => onAssignSelectedElementMaterial(event.target.value)}>
                    {materialOptions.map((option) => (
                      <option key={option.id} value={option.id}>{option.label}</option>
                    ))}
                  </select>
                </label>
                <label><span>{t.area}</span><input type="number" step={0.0001} value={trussElementArea} onChange={(event) => onUpdateSelectedElement("area", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={trussElementModulusGpa} onChange={(event) => onUpdateSelectedElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
              </div>
            ) : (isTruss3d || isSpring3d) && selectedTruss3dNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedTruss3dNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedTruss3dNodeData.x} onChange={(event) => onUpdateSelectedTruss3dNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedTruss3dNodeData.y} onChange={(event) => onUpdateSelectedTruss3dNode("y", Number(event.target.value))} /></label>
                <label><span>{t.nodeZ}</span><input type="number" step={0.1} value={selectedTruss3dNodeData.z} onChange={(event) => onUpdateSelectedTruss3dNode("z", Number(event.target.value))} /></label>
                <label><span>{t.loadX}</span><input type="number" step={100} value={selectedTruss3dNodeData.load_x} onChange={(event) => onUpdateSelectedTruss3dNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.loadY}</span><input type="number" step={100} value={selectedTruss3dNodeData.load_y} onChange={(event) => onUpdateSelectedTruss3dNode("load_y", Number(event.target.value))} /></label>
                <label><span>{t.loadZ}</span><input type="number" step={100} value={selectedTruss3dNodeData.load_z} onChange={(event) => onUpdateSelectedTruss3dNode("load_z", Number(event.target.value))} /></label>
                <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedTruss3dNodeData.fix_x} onChange={(event) => onUpdateSelectedTruss3dNode("fix_x", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedTruss3dNodeData.fix_y} onChange={(event) => onUpdateSelectedTruss3dNode("fix_y", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixZ}</span><input type="checkbox" checked={selectedTruss3dNodeData.fix_z} onChange={(event) => onUpdateSelectedTruss3dNode("fix_z", event.target.checked)} /></label>
              </div>
            ) : isTruss3d && selectedTruss3dElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedTruss3dElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedTruss3dElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedTruss3dElementData.node_j} readOnly /></label>
                <label>
                  <span>{materialLabel}</span>
                  <select value={truss3dElementMaterialId} onChange={(event) => onAssignSelectedTruss3dElementMaterial(event.target.value)}>
                    {materialOptions.map((option) => (
                      <option key={option.id} value={option.id}>{option.label}</option>
                    ))}
                  </select>
                </label>
                <label><span>{t.area}</span><input type="number" step={0.0001} value={truss3dElementArea} onChange={(event) => onUpdateSelectedTruss3dElement("area", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={truss3dElementModulusGpa} onChange={(event) => onUpdateSelectedTruss3dElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
              </div>
            ) : isPlane && selectedPlaneNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedPlaneNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedPlaneNodeData.x} onChange={(event) => onUpdateSelectedPlaneNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedPlaneNodeData.y} onChange={(event) => onUpdateSelectedPlaneNode("y", Number(event.target.value))} /></label>
                {isHeatPlane ? (
                  <>
                    <label><span>{t.temperature}</span><input type="number" step={1} value={selectedPlaneNodeData.temperature ?? 0} onChange={(event) => onUpdateSelectedPlaneNode("temperature", Number(event.target.value))} /></label>
                    <label><span>{t.heatLoad}</span><input type="number" step={1} value={selectedPlaneNodeData.heat_load ?? 0} onChange={(event) => onUpdateSelectedPlaneNode("heat_load", Number(event.target.value))} /></label>
                    <label className="toggle-row"><span>{t.fixTemperature}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_temperature ?? false} onChange={(event) => onUpdateSelectedPlaneNode("fix_temperature", event.target.checked)} /></label>
                  </>
                ) : isElectrostaticPlane ? (
                  <>
                    <label><span>{t.potential}</span><input type="number" step={0.1} value={selectedPlaneNodeData.potential ?? 0} onChange={(event) => onUpdateSelectedPlaneNode("potential", Number(event.target.value))} /></label>
                    <label><span>{t.chargeDensity}</span><input type="number" step={0.1} value={selectedPlaneNodeData.charge_density ?? 0} onChange={(event) => onUpdateSelectedPlaneNode("charge_density", Number(event.target.value))} /></label>
                    <label className="toggle-row"><span>{t.fixPotential}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_potential ?? false} onChange={(event) => onUpdateSelectedPlaneNode("fix_potential", event.target.checked)} /></label>
                  </>
                ) : (
                  <>
                    <label><span>{t.loadX}</span><input type="number" step={100} value={selectedPlaneNodeData.load_x} onChange={(event) => onUpdateSelectedPlaneNode("load_x", Number(event.target.value))} /></label>
                    <label><span>{t.loadY}</span><input type="number" step={100} value={selectedPlaneNodeData.load_y} onChange={(event) => onUpdateSelectedPlaneNode("load_y", Number(event.target.value))} /></label>
                  </>
                )}
                {!isHeatPlane && "temperature_delta" in selectedPlaneNodeData ? (
                  <label><span>{t.temperatureDelta}</span><input type="number" step={1} value={selectedPlaneNodeData.temperature_delta ?? 0} onChange={(event) => onUpdateSelectedPlaneNode("temperature_delta", Number(event.target.value))} /></label>
                ) : null}
                {hasPlaneElectrostaticNode && !isElectrostaticPlane ? <label><span>{t.potential}</span><input value={formatInspectorMetric(selectedPlaneNodeData.potential)} readOnly /></label> : null}
                {hasPlaneElectrostaticNode && !isElectrostaticPlane ? <label><span>{t.chargeDensity}</span><input value={formatInspectorMetric(selectedPlaneNodeData.charge_density)} readOnly /></label> : null}
                {typeof selectedPlaneNodeData.fix_potential === "boolean" && !isElectrostaticPlane ? <label className="toggle-row"><span>{t.fixPotential}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_potential} readOnly /></label> : null}
                {!isHeatPlane ? <label><span>{t.displacementMagnitude}</span><input value={typeof selectedPlaneNodeData.displacement_magnitude === "number" ? selectedPlaneNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label> : null}
                {!isHeatPlane && !isElectrostaticPlane ? <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_x} onChange={(event) => onUpdateSelectedPlaneNode("fix_x", event.target.checked)} /></label> : null}
                {!isHeatPlane && !isElectrostaticPlane ? <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedPlaneNodeData.fix_y} onChange={(event) => onUpdateSelectedPlaneNode("fix_y", event.target.checked)} /></label> : null}
              </div>
            ) : isPlane && selectedPlaneElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedPlaneElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedPlaneElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedPlaneElementData.node_j} readOnly /></label>
                <label><span>{t.nodeK}</span><input value={selectedPlaneElementData.node_k} readOnly /></label>
                <label><span>{t.planeThickness}</span><input type="number" step={0.001} value={planeElementThickness} onChange={(event) => onUpdateSelectedPlaneElement("thickness", Number(event.target.value))} /></label>
                {isHeatPlane ? (
                  <>
                    <label><span>{t.conductivity}</span><input type="number" step={0.1} value={selectedPlaneElementData.conductivity ?? 0} onChange={(event) => onUpdateSelectedPlaneElement("conductivity", Number(event.target.value))} /></label>
                    <label><span>{t.averageTemperature}</span><input value={typeof selectedPlaneElementData.average_temperature === "number" ? selectedPlaneElementData.average_temperature.toExponential(3) : "--"} readOnly /></label>
                    <label><span>{t.temperatureGradientX}</span><input value={typeof selectedPlaneElementData.temperature_gradient_x === "number" ? selectedPlaneElementData.temperature_gradient_x.toExponential(3) : "--"} readOnly /></label>
                    <label><span>{t.temperatureGradientY}</span><input value={typeof selectedPlaneElementData.temperature_gradient_y === "number" ? selectedPlaneElementData.temperature_gradient_y.toExponential(3) : "--"} readOnly /></label>
                    <label><span>{t.heatFluxX}</span><input value={typeof selectedPlaneElementData.heat_flux_x === "number" ? selectedPlaneElementData.heat_flux_x.toExponential(3) : "--"} readOnly /></label>
                    <label><span>{t.heatFluxY}</span><input value={typeof selectedPlaneElementData.heat_flux_y === "number" ? selectedPlaneElementData.heat_flux_y.toExponential(3) : "--"} readOnly /></label>
                    <label><span>{t.maxHeatFlux}</span><input value={typeof selectedPlaneElementData.heat_flux_magnitude === "number" ? selectedPlaneElementData.heat_flux_magnitude.toExponential(3) : "--"} readOnly /></label>
                  </>
                ) : isElectrostaticPlane ? (
                  <>
                    <label>
                      <span>{materialLabel}</span>
                      <select value={planeElementMaterialId} onChange={(event) => onAssignSelectedPlaneElementMaterial(event.target.value)}>
                        {materialOptions.map((option) => (
                          <option key={option.id} value={option.id}>{option.label}</option>
                        ))}
                      </select>
                    </label>
                    <label><span>{t.electricFluxDensity}</span><input value={`${formatInspectorMetric(selectedPlaneElementData.electric_flux_density_x)} / ${formatInspectorMetric(selectedPlaneElementData.electric_flux_density_y)}`} readOnly /></label>
                    <label><span>{t.electricField}</span><input value={`${formatInspectorMetric(selectedPlaneElementData.electric_field_x)} / ${formatInspectorMetric(selectedPlaneElementData.electric_field_y)}`} readOnly /></label>
                    <label><span>{t.permittivity}</span><input type="number" step={0.1} value={selectedPlaneElementData.permittivity ?? 0} onChange={(event) => onUpdateSelectedPlaneElement("permittivity", Number(event.target.value))} /></label>
                  </>
                ) : (
                  <>
                    <label>
                      <span>{materialLabel}</span>
                      <select value={planeElementMaterialId} onChange={(event) => onAssignSelectedPlaneElementMaterial(event.target.value)}>
                        {materialOptions.map((option) => (
                          <option key={option.id} value={option.id}>{option.label}</option>
                        ))}
                      </select>
                    </label>
                    <label><span>{t.modulus}</span><input type="number" step={0.1} value={planeElementModulusGpa} onChange={(event) => onUpdateSelectedPlaneElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                    <label><span>{t.poissonRatio}</span><input type="number" step={0.01} min={0.01} max={0.49} value={planeElementPoissonRatio} onChange={(event) => onUpdateSelectedPlaneElement("poisson_ratio", Number(event.target.value))} /></label>
                  </>
                )}
                {!isHeatPlane && "thermal_expansion" in selectedPlaneElementData ? (
                  <>
                    <label><span>{t.thermalExpansion}</span><input type="number" step={0.000001} value={selectedPlaneElementData.thermal_expansion ?? 0} onChange={(event) => onUpdateSelectedPlaneElement("thermal_expansion", Number(event.target.value))} /></label>
                    <label><span>{t.temperatureDelta}</span><input value={typeof selectedPlaneElementData.average_temperature_delta === "number" ? selectedPlaneElementData.average_temperature_delta.toExponential(3) : "--"} readOnly /></label>
                    <label><span>{t.thermalStrain}</span><input value={typeof selectedPlaneElementData.thermal_strain === "number" ? selectedPlaneElementData.thermal_strain.toExponential(3) : "--"} readOnly /></label>
                    <label><span>{t.mechanicalStrain}</span><input value={typeof selectedPlaneElementData.mechanical_strain_x === "number" || typeof selectedPlaneElementData.mechanical_strain_y === "number" ? `${(selectedPlaneElementData.mechanical_strain_x ?? 0).toExponential(3)} / ${(selectedPlaneElementData.mechanical_strain_y ?? 0).toExponential(3)}` : "--"} readOnly /></label>
                    <label><span>{t.totalStrain}</span><input value={typeof selectedPlaneElementData.total_strain_x === "number" || typeof selectedPlaneElementData.total_strain_y === "number" ? `${(selectedPlaneElementData.total_strain_x ?? 0).toExponential(3)} / ${(selectedPlaneElementData.total_strain_y ?? 0).toExponential(3)}` : "--"} readOnly /></label>
                  </>
                ) : null}
                {hasPlaneElectrostaticElement ? <label><span>{t.averagePotential}</span><input value={formatInspectorMetric(selectedPlaneElementData.average_potential)} readOnly /></label> : null}
                {hasPlaneElectrostaticElement ? <label><span>{t.potentialGradient}</span><input value={`${formatInspectorMetric(selectedPlaneElementData.potential_gradient_x)} / ${formatInspectorMetric(selectedPlaneElementData.potential_gradient_y)}`} readOnly /></label> : null}
                {hasPlaneElectrostaticElement ? <label><span>{t.electricField}</span><input value={`${formatInspectorMetric(selectedPlaneElementData.electric_field_x)} / ${formatInspectorMetric(selectedPlaneElementData.electric_field_y)}`} readOnly /></label> : null}
                {hasPlaneElectrostaticElement ? <label><span>{t.electricFieldMagnitude}</span><input value={formatInspectorMetric(selectedPlaneElementData.electric_field_magnitude)} readOnly /></label> : null}
                {hasPlaneElectrostaticElement ? <label><span>{t.electricFluxDensity}</span><input value={`${formatInspectorMetric(selectedPlaneElementData.electric_flux_density_x)} / ${formatInspectorMetric(selectedPlaneElementData.electric_flux_density_y)}`} readOnly /></label> : null}
                {hasPlaneElectrostaticElement ? <label><span>{t.electricFluxDensityMagnitude}</span><input value={formatInspectorMetric(selectedPlaneElementData.electric_flux_density_magnitude)} readOnly /></label> : null}
                {!isHeatPlane ? <label><span>{t.principalStress1}</span><input value={typeof selectedPlaneElementData.principal_stress_1 === "number" ? selectedPlaneElementData.principal_stress_1.toExponential(3) : "--"} readOnly /></label> : null}
                {!isHeatPlane ? <label><span>{t.principalStress2}</span><input value={typeof selectedPlaneElementData.principal_stress_2 === "number" ? selectedPlaneElementData.principal_stress_2.toExponential(3) : "--"} readOnly /></label> : null}
                {!isHeatPlane ? <label><span>{t.maxInPlaneShear}</span><input value={typeof selectedPlaneElementData.max_in_plane_shear === "number" ? selectedPlaneElementData.max_in_plane_shear.toExponential(3) : "--"} readOnly /></label> : null}
              </div>
            ) : isBeam && selectedFrameNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedFrameNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input value={selectedFrameNodeData.x} readOnly /></label>
                <label><span>{t.nodeY}</span><input value={selectedFrameNodeData.y} readOnly /></label>
                <label><span>{t.loadY}</span><input value={selectedFrameNodeData.load_y} readOnly /></label>
                <label><span>{t.momentZ}</span><input value={selectedFrameNodeData.moment_z} readOnly /></label>
                <label><span>{t.displacementMagnitude}</span><input value={typeof selectedFrameNodeData.displacement_magnitude === "number" ? selectedFrameNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.rotationZ}</span><input value={typeof selectedFrameNodeData.rz === "number" ? selectedFrameNodeData.rz.toExponential(3) : "--"} readOnly /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedFrameNodeData.fix_y} readOnly /></label>
                <label className="toggle-row"><span>{t.fixRz}</span><input type="checkbox" checked={selectedFrameNodeData.fix_rz} readOnly /></label>
              </div>
            ) : isHeatBar && selectedFrameNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedFrameNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedFrameNodeData.x} onChange={(event) => onUpdateSelectedFrameNode("x", Number(event.target.value))} /></label>
                <label><span>{t.temperature}</span><input type="number" step={1} value={selectedFrameNodeData.temperature ?? 0} onChange={(event) => onUpdateSelectedFrameNode("temperature_delta", Number(event.target.value))} /></label>
                <label><span>{t.heatLoad}</span><input type="number" step={1} value={selectedFrameNodeData.heat_load ?? selectedFrameNodeData.load_x ?? 0} onChange={(event) => onUpdateSelectedFrameNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.maxTemperature}</span><input value={typeof selectedFrameNodeData.displacement_magnitude === "number" ? selectedFrameNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label>
                <label className="toggle-row"><span>{t.fixTemperature}</span><input type="checkbox" checked={selectedFrameNodeData.fix_temperature ?? selectedFrameNodeData.fix_x} onChange={(event) => onUpdateSelectedFrameNode("fix_x", event.target.checked)} /></label>
              </div>
            ) : isTorsion && selectedFrameNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedFrameNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedFrameNodeData.x} onChange={(event) => onUpdateSelectedFrameNode("x", Number(event.target.value))} /></label>
                <label><span>{t.torqueZ}</span><input type="number" step={100} value={selectedFrameNodeData.moment_z} onChange={(event) => onUpdateSelectedFrameNode("moment_z", Number(event.target.value))} /></label>
                <label><span>{t.rotationZ}</span><input value={typeof selectedFrameNodeData.rz === "number" ? selectedFrameNodeData.rz.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.displacementMagnitude}</span><input value={typeof selectedFrameNodeData.displacement_magnitude === "number" ? selectedFrameNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label>
                <label className="toggle-row"><span>{t.fixRz}</span><input type="checkbox" checked={selectedFrameNodeData.fix_rz} onChange={(event) => onUpdateSelectedFrameNode("fix_rz", event.target.checked)} /></label>
              </div>
            ) : isFrame && selectedFrameNodeData ? (
              <div className="form-grid compact">
                <label><span>{t.dragNode}</span><input value={selectedFrameNodeData.id} readOnly /></label>
                <label><span>{t.nodeX}</span><input type="number" step={0.1} value={selectedFrameNodeData.x} onChange={(event) => onUpdateSelectedFrameNode("x", Number(event.target.value))} /></label>
                <label><span>{t.nodeY}</span><input type="number" step={0.1} value={selectedFrameNodeData.y} onChange={(event) => onUpdateSelectedFrameNode("y", Number(event.target.value))} /></label>
                <label><span>{t.loadX}</span><input type="number" step={100} value={selectedFrameNodeData.load_x} onChange={(event) => onUpdateSelectedFrameNode("load_x", Number(event.target.value))} /></label>
                <label><span>{t.loadY}</span><input type="number" step={100} value={selectedFrameNodeData.load_y} onChange={(event) => onUpdateSelectedFrameNode("load_y", Number(event.target.value))} /></label>
                <label><span>{t.momentZ}</span><input type="number" step={100} value={selectedFrameNodeData.moment_z} onChange={(event) => onUpdateSelectedFrameNode("moment_z", Number(event.target.value))} /></label>
                {studyKind === "thermal_frame_2d" ? (
                  <label><span>{t.temperatureDelta}</span><input type="number" step={1} value={selectedFrameNodeData.temperature_delta ?? 0} onChange={(event) => onUpdateSelectedFrameNode("temperature_delta", Number(event.target.value))} /></label>
                ) : null}
                <label><span>{t.displacementMagnitude}</span><input value={typeof selectedFrameNodeData.displacement_magnitude === "number" ? selectedFrameNodeData.displacement_magnitude.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.rotationZ}</span><input value={typeof selectedFrameNodeData.rz === "number" ? selectedFrameNodeData.rz.toExponential(3) : "--"} readOnly /></label>
                <label className="toggle-row"><span>{t.fixX}</span><input type="checkbox" checked={selectedFrameNodeData.fix_x} onChange={(event) => onUpdateSelectedFrameNode("fix_x", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixY}</span><input type="checkbox" checked={selectedFrameNodeData.fix_y} onChange={(event) => onUpdateSelectedFrameNode("fix_y", event.target.checked)} /></label>
                <label className="toggle-row"><span>{t.fixRz}</span><input type="checkbox" checked={selectedFrameNodeData.fix_rz} onChange={(event) => onUpdateSelectedFrameNode("fix_rz", event.target.checked)} /></label>
              </div>
            ) : isBeam && selectedFrameElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedFrameElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedFrameElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedFrameElementData.node_j} readOnly /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={(selectedFrameElementData.youngs_modulus / 1.0e9).toFixed(3)} onChange={(event) => onUpdateSelectedFrameElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                <label><span>{t.momentOfInertia}</span><input type="number" step={0.000001} value={selectedFrameElementData.moment_of_inertia} onChange={(event) => onUpdateSelectedFrameElement("moment_of_inertia", Number(event.target.value))} /></label>
                <label><span>{t.sectionModulus}</span><input type="number" step={0.000001} value={selectedFrameElementData.section_modulus} onChange={(event) => onUpdateSelectedFrameElement("section_modulus", Number(event.target.value))} /></label>
                <label><span>{t.distributedLoadY}</span><input type="number" step={100} value={selectedFrameElementData.distributed_load_y ?? 0} onChange={(event) => onUpdateSelectedFrameElement("distributed_load_y", Number(event.target.value))} /></label>
                {studyKind === "thermal_beam_1d" ? (
                  <>
                    <label><span>{t.temperatureGradientY}</span><input type="number" step={1} value={selectedFrameElementData.temperature_gradient_y ?? 0} onChange={(event) => onUpdateSelectedFrameElement("temperature_gradient_y", Number(event.target.value))} /></label>
                    <label><span>{t.thermalCurvature}</span><input value={typeof selectedFrameElementData.thermal_curvature === "number" ? selectedFrameElementData.thermal_curvature.toExponential(3) : "--"} readOnly /></label>
                  </>
                ) : null}
                <label><span>{t.bendingStress}</span><input value={typeof selectedFrameElementData.max_bending_stress === "number" ? selectedFrameElementData.max_bending_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.shearI}</span><input value={typeof selectedFrameElementData.shear_force_i === "number" ? selectedFrameElementData.shear_force_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentI}</span><input value={typeof selectedFrameElementData.moment_i === "number" ? selectedFrameElementData.moment_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.shearJ}</span><input value={typeof selectedFrameElementData.shear_force_j === "number" ? selectedFrameElementData.shear_force_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentJ}</span><input value={typeof selectedFrameElementData.moment_j === "number" ? selectedFrameElementData.moment_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.maxMoment}</span><input value={Math.max(Math.abs(selectedFrameElementData.moment_i ?? 0), Math.abs(selectedFrameElementData.moment_j ?? 0)).toExponential(3)} readOnly /></label>
              </div>
            ) : isHeatBar && selectedFrameElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedFrameElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedFrameElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedFrameElementData.node_j} readOnly /></label>
                <label><span>{t.area}</span><input type="number" step={0.0001} value={selectedFrameElementData.area} onChange={(event) => onUpdateSelectedFrameElement("area", Number(event.target.value))} /></label>
                <label><span>{t.conductivity}</span><input type="number" step={0.1} value={selectedFrameElementData.youngs_modulus ?? 0} onChange={(event) => onUpdateSelectedFrameElement("youngs_modulus", Number(event.target.value))} /></label>
                <label><span>{t.averageTemperature}</span><input value={typeof selectedFrameElementData.average_temperature === "number" ? selectedFrameElementData.average_temperature.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.temperatureGradientX}</span><input value={typeof selectedFrameElementData.temperature_gradient_x === "number" ? selectedFrameElementData.temperature_gradient_x.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.heatFluxX}</span><input value={typeof selectedFrameElementData.heat_flux_x === "number" ? selectedFrameElementData.heat_flux_x.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.maxHeatFlux}</span><input value={typeof selectedFrameElementData.heat_flux_magnitude === "number" ? selectedFrameElementData.heat_flux_magnitude.toExponential(3) : "--"} readOnly /></label>
              </div>
            ) : isTorsion && selectedFrameElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedFrameElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedFrameElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedFrameElementData.node_j} readOnly /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={(selectedFrameElementData.youngs_modulus / 1.0e9).toFixed(3)} onChange={(event) => onUpdateSelectedFrameElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                <label><span>{t.momentOfInertia}</span><input type="number" step={0.000001} value={selectedFrameElementData.moment_of_inertia} onChange={(event) => onUpdateSelectedFrameElement("moment_of_inertia", Number(event.target.value))} /></label>
                <label><span>{t.sectionModulus}</span><input type="number" step={0.000001} value={selectedFrameElementData.section_modulus} onChange={(event) => onUpdateSelectedFrameElement("section_modulus", Number(event.target.value))} /></label>
                <label><span>{t.torsionStress}</span><input value={typeof selectedFrameElementData.max_bending_stress === "number" ? selectedFrameElementData.max_bending_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentI}</span><input value={typeof selectedFrameElementData.moment_i === "number" ? selectedFrameElementData.moment_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentJ}</span><input value={typeof selectedFrameElementData.moment_j === "number" ? selectedFrameElementData.moment_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.maxTorque}</span><input value={Math.max(Math.abs(selectedFrameElementData.moment_i ?? 0), Math.abs(selectedFrameElementData.moment_j ?? 0)).toExponential(3)} readOnly /></label>
              </div>
            ) : isFrame && selectedFrameElementData ? (
              <div className="form-grid compact">
                <label><span>{t.memberSelection}</span><input value={selectedFrameElementData.id} readOnly /></label>
                <label><span>{t.nodeI}</span><input value={selectedFrameElementData.node_i} readOnly /></label>
                <label><span>{t.nodeJ}</span><input value={selectedFrameElementData.node_j} readOnly /></label>
                <label>
                  <span>{materialLabel}</span>
                  <select value={frameElementMaterialId} onChange={(event) => onAssignSelectedFrameElementMaterial(event.target.value)}>
                    {materialOptions.map((option) => (
                      <option key={option.id} value={option.id}>{option.label}</option>
                    ))}
                  </select>
                </label>
                <label><span>{t.area}</span><input type="number" step={0.0001} value={selectedFrameElementData.area} onChange={(event) => onUpdateSelectedFrameElement("area", Number(event.target.value))} /></label>
                <label><span>{t.modulus}</span><input type="number" step={0.1} value={(selectedFrameElementData.youngs_modulus / 1.0e9).toFixed(3)} onChange={(event) => onUpdateSelectedFrameElement("youngs_modulus", Number(event.target.value) * 1.0e9)} /></label>
                <label><span>{t.momentOfInertia}</span><input type="number" step={0.000001} value={selectedFrameElementData.moment_of_inertia} onChange={(event) => onUpdateSelectedFrameElement("moment_of_inertia", Number(event.target.value))} /></label>
                <label><span>{t.sectionModulus}</span><input type="number" step={0.000001} value={selectedFrameElementData.section_modulus} onChange={(event) => onUpdateSelectedFrameElement("section_modulus", Number(event.target.value))} /></label>
                {studyKind === "thermal_frame_2d" ? (
                  <>
                    <label><span>{t.temperatureGradientY}</span><input type="number" step={1} value={selectedFrameElementData.temperature_gradient_y ?? 0} onChange={(event) => onUpdateSelectedFrameElement("temperature_gradient_y", Number(event.target.value))} /></label>
                    <label><span>{t.temperatureDelta}</span><input value={typeof selectedFrameElementData.average_temperature_delta === "number" ? selectedFrameElementData.average_temperature_delta.toExponential(3) : "--"} readOnly /></label>
                    <label><span>{t.thermalCurvature}</span><input value={typeof selectedFrameElementData.thermal_curvature === "number" ? selectedFrameElementData.thermal_curvature.toExponential(3) : "--"} readOnly /></label>
                  </>
                ) : null}
                <label><span>{t.principalStress1}</span><input value={typeof selectedFrameElementData.axial_stress === "number" ? selectedFrameElementData.axial_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.bendingStress}</span><input value={typeof selectedFrameElementData.max_bending_stress === "number" ? selectedFrameElementData.max_bending_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.combinedStress}</span><input value={typeof selectedFrameElementData.max_combined_stress === "number" ? selectedFrameElementData.max_combined_stress.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.forceI}</span><input value={typeof selectedFrameElementData.axial_force_i === "number" ? selectedFrameElementData.axial_force_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.shearI}</span><input value={typeof selectedFrameElementData.shear_force_i === "number" ? selectedFrameElementData.shear_force_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentI}</span><input value={typeof selectedFrameElementData.moment_i === "number" ? selectedFrameElementData.moment_i.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.forceJ}</span><input value={typeof selectedFrameElementData.axial_force_j === "number" ? selectedFrameElementData.axial_force_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.shearJ}</span><input value={typeof selectedFrameElementData.shear_force_j === "number" ? selectedFrameElementData.shear_force_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.momentJ}</span><input value={typeof selectedFrameElementData.moment_j === "number" ? selectedFrameElementData.moment_j.toExponential(3) : "--"} readOnly /></label>
                <label><span>{t.maxMoment}</span><input value={Math.max(Math.abs(selectedFrameElementData.moment_i ?? 0), Math.abs(selectedFrameElementData.moment_j ?? 0)).toExponential(3)} readOnly /></label>
              </div>
            ) : (
              <p className="card-copy">{t.selectionHint}</p>
            )}
          </section>
        ) : null}

        {sidebarSection === "model" && inspectorTab === "status" && statusPage === "diagnostics" ? (
          <WorkbenchInspectorDiagnosticsPanel
            t={t}
            isTruss={isTruss}
            trussDiagnostics={trussDiagnostics}
            trussStability={trussStability}
            hotspotNodeLabels={hotspotNodeLabels}
            onApplyTrussSuggestion={onApplyTrussSuggestion}
          />
        ) : null}

        {inspectorTab === "actions" && actionsPage === "history" ? (
          <WorkbenchInspectorHistoryPanel
            t={t}
            undoStack={undoStack}
            redoStack={redoStack}
            onUndo={onUndo}
            onRedo={onRedo}
          />
        ) : null}

        {inspectorTab === "result" ? (
          <WorkbenchInspectorResultPanels
            t={t}
            resultPage={resultPage}
            studyKind={studyKind}
            job={job}
            nodeCount={nodeCount}
            tipDisplacement={tipDisplacement}
            maxStressValue={maxStressValue}
            frameMaxAxialForceValue={frameMaxAxialForceValue}
            frameMaxShearForceValue={frameMaxShearForceValue}
            reactionValue={reactionValue}
            frameMaxRotationValue={frameMaxRotationValue}
            thermalFrameMaxTemperatureDeltaValue={thermalFrameMaxTemperatureDeltaValue}
            thermalFrameMaxTemperatureGradientValue={thermalFrameMaxTemperatureGradientValue}
            thermalBeamMaxTemperatureGradientValue={thermalBeamMaxTemperatureGradientValue}
            thermalPlaneMaxTemperatureDeltaValue={thermalPlaneMaxTemperatureDeltaValue}
            reportScopeLabel={reportScopeLabel}
            reportScopeHint={reportScopeHint}
          />
        ) : null}

        <WorkbenchInspectorActionsExportPanel
          t={t}
          actionsPage={actionsPage}
          studyKind={studyKind}
          job={job}
          tipDisplacement={tipDisplacement}
          maxStressValue={maxStressValue}
          frameMaxAxialForceValue={frameMaxAxialForceValue}
          frameMaxShearForceValue={frameMaxShearForceValue}
          reactionValue={reactionValue}
          frameMaxRotationValue={frameMaxRotationValue}
          thermalFrameMaxTemperatureDeltaValue={thermalFrameMaxTemperatureDeltaValue}
          thermalFrameMaxTemperatureGradientValue={thermalFrameMaxTemperatureGradientValue}
          thermalBeamMaxTemperatureGradientValue={thermalBeamMaxTemperatureGradientValue}
          thermalPlaneMaxTemperatureDeltaValue={thermalPlaneMaxTemperatureDeltaValue}
          reportScopeLabel={reportScopeLabel}
          reportScopeHint={reportScopeHint}
          createdAtValue={createdAtValue}
          updatedAtValue={updatedAtValue}
          heartbeatStatusValue={heartbeatStatusValue}
          heartbeatTone={heartbeatTone}
          failureReasonValue={failureReasonValue}
          canCancelJob={canCancelJob}
          onCancelJob={onCancelJob}
          onDownloadJson={onDownloadJson}
          onDownloadCsv={onDownloadCsv}
          canProjectHeatToThermo={canProjectHeatToThermo}
          projectHeatToThermoLabel={projectHeatToThermoLabel}
          onProjectHeatToThermo={onProjectHeatToThermo}
          planeHotspotFieldLabel={planeHotspotFieldLabel}
          planeHotspotElements={planeHotspotElements}
          planeThermalRows={planeThermalRows}
          frameHotspotFieldLabel={frameHotspotFieldLabel}
          frameHotspotElements={frameHotspotElements}
          frameForceRows={frameForceRows}
          planeHotspotLimit={planeHotspotLimit}
          onDownloadPlaneHotspots={onDownloadPlaneHotspots}
          onDownloadFrameHotspots={onDownloadFrameHotspots}
          onDownloadFrameForces={onDownloadFrameForces}
          onSelectPlaneHotspot={onSelectPlaneHotspot}
          onSelectFrameHotspot={onSelectFrameHotspot}
          onPlaneHotspotLimitChange={onPlaneHotspotLimitChange}
          selectedFrameElementData={selectedFrameElementData}
          frameForceSort={frameForceSort}
          onFrameForceSortChange={setFrameForceSort}
          planeHeatSort={planeHeatSort}
          onPlaneHeatSortChange={setPlaneHeatSort}
        />
      </div>
    </aside>
  );
}

export const WorkbenchInspector = memo(WorkbenchInspectorInner);
