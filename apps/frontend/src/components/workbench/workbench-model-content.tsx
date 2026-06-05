"use client";

import type { ReactNode } from "react";

import { WorkbenchObjectTree } from "@/components/workbench/workbench-object-tree";
import { lineResultFieldValue, localMaterialLabel } from "@/components/workbench/workbench-result-helpers";
import { WorkbenchMaterialLibraryCard } from "@/components/workbench/model/workbench-material-library-card";
import { WorkbenchModelToolsCard } from "@/components/workbench/model/workbench-model-tools-card";
import { WorkbenchParametricCard } from "@/components/workbench/model/workbench-parametric-card";
import { WorkbenchTruss3dTreeCard } from "@/components/workbench/model/workbench-truss3d-tree-card";
import { MATERIAL_PRESETS } from "@/lib/materials";
import { fixed, scientific } from "@/lib/workbench/helpers";
import { classifyStudyKindDomain } from "@/lib/workbench/view-models";

export function buildWorkbenchModelContent(props: Record<string, any>) {
  const {
    t,
    isAxial,
    axialForm,
    handleAxialFieldChange,
    handleMaterialChange,
    localMaterialLabel: localMaterialLabelOverride,
    language,
    loadedModelName,
    studyDomainOptions,
    studyKind,
    studyKindOptionGroups,
    selectStudyKind,
    studyControlsRows,
    isPending,
    runAnalysis,
    isTruss3d,
    isFrameLike,
    isPlane,
    isTorsion,
    isThermal,
    currentStudyFamilyHint,
    selectedNode,
    selectedElement,
    truss3dLinkMode,
    undoStack,
    redoStack,
    isTruss,
    addTruss3dNode,
    addNode,
    deleteSelectedTruss3dNode,
    deleteSelectedNode,
    toggleTruss3dMemberFromDraft,
    toggleMemberFromDraft,
    deleteSelectedTruss3dElement,
    deleteSelectedElement,
    toggleTruss3dLinkMode,
    handleUndo,
    handleRedo,
    downloadModel,
    setStudyKind,
    setSidebarSection,
    setMessage,
    isBeam,
    isSpring,
    isSpring1d,
    isSpring2d,
    isSpring3d,
    hiddenMaterialIds,
    activeMaterial,
    currentMaterials,
    localMaterialLabel,
    materialColorMap,
    setActiveMaterial,
    addMaterialToCurrentModel,
    addCustomMaterialToCurrentModel,
    importMaterials,
    updateCurrentMaterial,
    toggleMaterialVisibility,
    applyMaterialToCurrentModel,
    deleteCurrentMaterial,
    round,
    isHeatBar,
    isThermalBar,
    panelParametric,
    parametric,
    handlePanelParametricChange,
    handleParametricChange,
    generatePanelModel,
    generateModel,
    selectedTruss3dNodes,
    memberDraftNodes,
    truss3dTreeRows,
    truss3dModel,
    setSelectedElement,
    setSelectedNode,
    setSelectedTruss3dNodes,
    setMemberDraftNodes,
    currentStudyFamilyLabel,
    spring3dModel,
    spring2dModel,
    springModel,
    thermalBarModel,
    thermalTrussModel,
    activePlaneInputModel,
    activeFrameLikeModel,
    activeBeamLikeModel,
    torsionModel,
    heatBarModel,
    planeElements,
    displayTrussElements,
    activeLineResultField,
    frameTreeValueLabel,
    trussDiagnostics,
    toggleDraftNode,
    setFocusedFrameElement,
  } = props;

  const materialLabelFn = localMaterialLabelOverride ?? localMaterialLabel;

  const studyControlsContent: ReactNode = isAxial ? (
    <div className="form-grid compact">
      <label>
        <span>{t.length}</span>
        <input
          type="number"
          value={axialForm.length}
          min={0.1}
          step={0.1}
          onChange={(event) => handleAxialFieldChange("length", Number(event.target.value))}
        />
      </label>
      <label>
        <span>{t.area}</span>
        <input
          type="number"
          value={axialForm.area}
          min={0.0001}
          step={0.0001}
          onChange={(event) => handleAxialFieldChange("area", Number(event.target.value))}
        />
      </label>
      <label>
        <span>{t.material}</span>
        <select value={axialForm.material} onChange={(event) => handleMaterialChange(event.target.value)}>
          {MATERIAL_PRESETS.map((preset) => (
            <option key={preset.value} value={preset.value}>
              {materialLabelFn(preset.value, language)}
            </option>
          ))}
        </select>
      </label>
      <label>
        <span>{t.modulus}</span>
        <input
          type="number"
          value={axialForm.youngsModulusGpa}
          min={0.1}
          step={0.1}
          onChange={(event) => handleAxialFieldChange("youngsModulusGpa", Number(event.target.value))}
        />
      </label>
      <label>
        <span>{t.elements}</span>
        <input
          type="number"
          value={axialForm.elements}
          min={1}
          max={120}
          step={1}
          onChange={(event) => handleAxialFieldChange("elements", Number(event.target.value))}
        />
      </label>
      <label>
        <span>{t.tipForce}</span>
        <input
          type="number"
          value={axialForm.tipForce}
          step={100}
          onChange={(event) => handleAxialFieldChange("tipForce", Number(event.target.value))}
        />
      </label>
    </div>
  ) : null;

  const modelStudyContent: ReactNode = (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{t.sections.study}</h2>
        <span>{loadedModelName}</span>
      </div>
      <div className="form-grid compact">
        <label>
          <span>{t.studyDomain}</span>
          <div className="button-row">
            {studyDomainOptions.map((option: { key: string; label: string }) => (
              <button
                key={option.key}
                className={`ghost-button ghost-button--compact${classifyStudyKindDomain(studyKind) === option.key ? " ghost-button--active" : ""}`}
                onClick={() => {
                  const fallback = studyKindOptionGroups.find((group: { domainKey: string; options: Array<{ value: string }> }) => group.domainKey === option.key)?.options[0]?.value;
                  if (fallback && fallback !== studyKind) {
                    selectStudyKind(fallback);
                  }
                }}
                type="button"
              >
                {option.label}
              </button>
            ))}
          </div>
        </label>
        <label>
          <span>{t.studyTypeLabel}</span>
          <select
            value={studyKind}
            onChange={(event) => {
              selectStudyKind(event.target.value);
            }}
          >
            {studyKindOptionGroups
              .filter((group: { options: unknown[] }) => group.options.length > 0)
              .map((group: { label: string; options: Array<{ value: string; label: string }> }) => (
                <optgroup key={group.label} label={group.label}>
                  {group.options.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </optgroup>
              ))}
          </select>
        </label>
      </div>
      {studyControlsContent}
      <div className="sidebar-list">
        {studyControlsRows.slice(0, 4).map((row: { label: string; value: string | number }) => (
          <div key={row.label}>
            <span>{row.label}</span>
            <strong>{row.value}</strong>
          </div>
        ))}
      </div>
      <button className="solve-button" disabled={isPending} onClick={runAnalysis} type="button">
        {isPending ? t.running : t.run}
      </button>
    </section>
  );

  const modelStudioContent: ReactNode = (
    <WorkbenchModelToolsCard
      title={isTruss3d ? t.spaceStudio : t.sections.model}
      status={isTruss3d ? t.orbitHint : isFrameLike ? t.ready : t.dragToEdit}
      hint={isTruss3d ? t.spaceStudioHint : isPlane ? t.planeHint : isFrameLike ? t.frameEditorHint : isTorsion ? t.torsionHint : isThermal ? currentStudyFamilyHint : t.modelStudioHint}
      selectionHint={t.selectionHint}
      addNodeLabel={t.addNode}
      addBranchNodeLabel={t.addBranchNode}
      deleteNodeLabel={t.deleteNode}
      toggleMemberLabel={t.toggleMember}
      deleteMemberLabel={t.deleteMember}
      linkModeLabel={t.linkMode}
      linkModeActiveLabel={t.linkModeActive}
      linkModeHint={t.linkModeIdle}
      undoLabel={t.undo}
      redoLabel={t.redo}
      downloadLabel={t.download}
      saveForSolverLabel={t.saveForSolver}
      canAddBranchNode={selectedNode !== null}
      canDeleteNode={selectedNode !== null}
      canDeleteMember={selectedElement !== null}
      canUndo={undoStack.length > 0}
      canRedo={redoStack.length > 0}
      isTruss={isTruss}
      isFrame={isFrameLike}
      isTruss3d={isTruss3d}
      truss3dLinkMode={truss3dLinkMode}
      onAddNode={() => {
        if (isTruss3d) return addTruss3dNode(false);
        addNode(false);
      }}
      onAddBranchNode={() => {
        if (isTruss3d) return addTruss3dNode(true);
        addNode(true);
      }}
      onDeleteNode={() => {
        if (isTruss3d) return deleteSelectedTruss3dNode();
        deleteSelectedNode();
      }}
      onToggleMember={() => {
        if (isTruss3d) return toggleTruss3dMemberFromDraft();
        toggleMemberFromDraft();
      }}
      onDeleteMember={() => {
        if (isTruss3d) return deleteSelectedTruss3dElement();
        deleteSelectedElement();
      }}
      onToggleLinkMode={toggleTruss3dLinkMode}
      onUndo={handleUndo}
      onRedo={handleRedo}
      onDownload={props.downloadModel}
      onSaveForSolver={() => {
        setStudyKind(isPlane || isBeam || isTorsion || isThermal ? studyKind : isTruss3d ? "truss_3d" : isFrameLike ? studyKind : "truss_2d");
        setSidebarSection("model");
        setMessage(isPlane ? t.planeHint : isBeam ? t.modelStudioHint : isTorsion ? t.torsionHint : isThermal ? currentStudyFamilyHint : isTruss3d ? t.switchedTo3dStudio : isFrameLike ? t.frameEditorHint : t.switchedTo2dStudio);
      }}
    />
  );

  const modelMaterialsContent: ReactNode =
    !isAxial && !isBeam && !isTorsion && !isHeatBar && !isThermalBar ? (
      <WorkbenchMaterialLibraryCard
        language={language}
        materialLabel={t.material}
        modulusLabel={t.modulus}
        poissonRatioLabel={t.poissonRatio}
        activeMaterial={activeMaterial}
        currentMaterials={currentMaterials}
        hiddenMaterialIds={hiddenMaterialIds}
        isPlane={isPlane}
        selectedElement={selectedElement}
        localMaterialLabel={materialLabelFn}
        getMaterialColor={(materialId) => materialColorMap.get(materialId) ?? "#1677a3"}
        onActiveMaterialChange={setActiveMaterial}
        onAddMaterial={addMaterialToCurrentModel}
        onAddCustomMaterial={addCustomMaterialToCurrentModel}
        onImportMaterials={(file) => void importMaterials(file)}
        onUpdateMaterial={updateCurrentMaterial}
        onToggleMaterialVisibility={toggleMaterialVisibility}
        onApplyMaterial={applyMaterialToCurrentModel}
        onDeleteMaterial={deleteCurrentMaterial}
        round={round}
      />
    ) : null;

  const modelGenerateContent: ReactNode =
    !isTruss3d && !isFrameLike && !isBeam && !isTorsion && !isThermal ? (
      <WorkbenchParametricCard
        isPlane={isPlane}
        title={isPlane ? t.panelGenerator : t.parametric}
        subtitle={t.modelTools}
        lengthLabel={t.length}
        heightLabel={t.height}
        divisionsXLabel={t.divisionsX}
        divisionsYLabel={t.divisionsY}
        thicknessLabel={t.planeThickness}
        modulusLabel={t.modulus}
        poissonRatioLabel={t.poissonRatio}
        loadCaseLabel={t.loadCase}
        baysLabel={t.bays}
        areaLabel={t.area}
        generateLabel={isPlane ? t.generatePanel : t.generate}
        panelParametric={panelParametric}
        parametric={parametric}
        onPanelParametricChange={handlePanelParametricChange}
        onParametricChange={handleParametricChange}
        onGenerate={isPlane ? generatePanelModel : generateModel}
      />
    ) : null;

  const modelTreeContent: ReactNode = isTruss3d ? (
    <WorkbenchTruss3dTreeCard
      title={t.objectTree}
      countLabel={
        selectedTruss3dNodes.length > 1
          ? `${selectedTruss3dNodes.length} ${t.nodes}`
          : truss3dLinkMode
            ? t.linkModeActive
            : `${memberDraftNodes.length}/2`
      }
      hint={truss3dLinkMode ? t.linkModeIdle : t.orbitHint}
      nodeILabel={t.nodeI}
      nodeJLabel={t.nodeJ}
      areaLabel={t.area}
      nodes={truss3dTreeRows.nodes}
      elements={truss3dTreeRows.elements.map((element: { index: number }) => ({
        ...element,
        area: fixed(truss3dModel.elements[element.index]?.area, 4),
        active: selectedElement === element.index,
      }))}
      onSelectNode={props.handleTruss3dNodePick}
      onSelectElement={(index) => {
        setSelectedElement(index);
        setSelectedNode(null);
        setSelectedTruss3dNodes([]);
        setMemberDraftNodes([]);
      }}
    />
  ) : (
    <WorkbenchObjectTree
      title={t.objectTree}
      scopeLabel={currentStudyFamilyLabel}
      countLabel={
        isTruss || isFrameLike
          ? `${memberDraftNodes.length}/2`
          : isBeam
            ? String(props.beamModel.elements.length)
            : isTorsion
              ? String(torsionModel.elements.length)
              : isHeatBar
                ? String(heatBarModel.elements.length)
                : isThermal
                  ? String(thermalBarModel.elements.length)
                  : isSpring1d
                    ? String(springModel.elements.length)
                    : isSpring2d
                      ? String(spring2dModel.elements.length)
                      : isSpring3d
                        ? String(spring3dModel.elements.length)
                        : String(activePlaneInputModel.elements.length)
      }
      hint={isTorsion ? t.torsionHint : currentStudyFamilyHint}
      geometryLabel={t.geometry}
      resultsLabel={t.results}
      sortByLabel={t.sortBy}
      diagnosticsLabel={t.diagnostics}
      loadCaseLabel={t.loadCase}
      nodeJLabel={t.nodeJ}
      nodeKLabel={t.nodeK}
      elementValueLabel={isFrameLike || isBeam || isTorsion || isSpring || isHeatBar || isThermal ? frameTreeValueLabel : undefined}
      nodeRows={(isPlane ? activePlaneInputModel.nodes : isFrameLike ? activeFrameLikeModel.nodes : isBeam ? activeBeamLikeModel.nodes : isTorsion ? torsionModel.nodes : isHeatBar ? heatBarModel.nodes : isThermalBar ? thermalBarModel.nodes : props.isThermalTruss2d ? thermalTrussModel.nodes : isSpring1d ? springModel.nodes : isSpring2d ? spring2dModel.nodes : isSpring3d ? spring3dModel.nodes : props.trussModel.nodes).map((node: any) => ({
        id: node.id,
        x: node.x,
        y: Number("y" in node ? node.y : 0),
        load_y: "load_y" in node ? node.load_y : "torque_z" in node ? node.torque_z : "load_x" in node ? node.load_x : 0,
      }))}
      elementRows={
        isPlane
          ? planeElements.map((element: any) => ({
              id: element.id,
              node_i: element.node_i,
              node_j: "node_j" in element ? element.node_j : undefined,
              node_k: "node_k" in element ? element.node_k : undefined,
            }))
          : displayTrussElements.map((element: any) => ({
              id: element.id,
              node_i: element.node_i,
              node_j: element.node_j,
              resultMagnitude: isFrameLike || isBeam || isSpring || isHeatBar || isThermal ? lineResultFieldValue(element, props.activeLineResultField) : undefined,
              resultValue: isFrameLike || isBeam || isSpring || isHeatBar || isThermal ? scientific(lineResultFieldValue(element, props.activeLineResultField)) : undefined,
            }))
      }
      isPlane={isPlane}
      isTruss={isTruss}
      enableElementResultsMode={isFrameLike || isBeam || isTorsion || isSpring || isHeatBar || isThermal}
      selectedNode={selectedNode}
      selectedElement={selectedElement}
      nodeIssueCounts={Object.fromEntries(
        Object.entries(trussDiagnostics?.nodeIssues ?? {}).map(([key, issues]: [string, any]) => [Number(key), issues.length]),
      )}
      onSelectNode={(index) => {
        if (isPlane) {
          setSelectedNode(index);
          setSelectedElement(null);
        } else if (isBeam || isTorsion || isSpring || isThermal) {
          setSelectedNode(index);
          setSelectedElement(null);
          setMemberDraftNodes([]);
        } else {
          toggleDraftNode(index);
        }
      }}
      onSelectElement={(index) => {
        setSelectedElement(index);
        setSelectedNode(null);
        if (isTruss || isFrameLike || isBeam || isTorsion || isSpring || isThermal) setMemberDraftNodes([]);
        if (isFrameLike || isBeam || isTorsion || isSpring || isThermal) setFocusedFrameElement(index);
      }}
    />
  );

  return {
    studyControlsContent,
    modelStudyContent,
    modelStudioContent,
    modelMaterialsContent,
    modelGenerateContent,
    modelTreeContent,
  };
}
