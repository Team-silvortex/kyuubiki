"use client";

import { fixed } from "@/lib/workbench/helpers";
import type { WorkbenchCopy } from "./workbench-copy";

type Truss3dNodeLike = {
  id?: string;
  index: number;
  x?: number;
  y?: number;
  z?: number;
};

type WorkbenchViewportDockProps = {
  t: WorkbenchCopy;
  immersiveViewport: boolean;
  immersiveToolDrawerOpen: boolean;
  immersiveHelpDrawerOpen: boolean;
  showShortcutHints: boolean;
  showViewportToolStrip: boolean;
  immersiveToolTab: "node" | "props";
  truss3dViewPreset: "iso" | "front" | "right" | "top";
  selectedNode: number | null;
  selectedElement: number | null;
  selectedTruss3dNodes: number[];
  selectedTruss3dNodeData: Truss3dNodeLike | null;
  truss3dLinkMode: boolean;
  truss3dProjectionMode: "ortho" | "persp";
  truss3dShowGrid: boolean;
  truss3dShowLabels: boolean;
  truss3dShowNodes: boolean;
  truss3dBoxSelectMode: boolean;
  truss3dNudgeStep: number;
  truss3dBatchLoadX: number;
  truss3dBatchLoadY: number;
  truss3dBatchLoadZ: number;
  undoStack: unknown[];
  redoStack: unknown[];
  truss3dModel: {
    nodes: Array<{
      x?: number;
      y?: number;
      z?: number;
      load_x?: number;
      load_y?: number;
      load_z?: number;
      fix_x?: boolean;
      fix_y?: boolean;
      fix_z?: boolean;
    }>;
  };
  setImmersiveToolTab: (tab: "node" | "props") => void;
  handleTruss3dViewPresetChange: (preset: "iso" | "front" | "right" | "top") => void;
  handleTruss3dFocusViewport: () => void;
  setTruss3dFocusRequestVersion: (updater: (current: number) => number) => void;
  handleTruss3dProjectionModeChange: (mode: "ortho" | "persp") => void;
  setTruss3dShowGrid: (updater: (current: boolean) => boolean) => void;
  setTruss3dShowLabels: (updater: (current: boolean) => boolean) => void;
  setTruss3dShowNodes: (updater: (current: boolean) => boolean) => void;
  handleTruss3dBoxSelectModeChange: (next: boolean) => void;
  handleTruss3dResetViewport: () => void;
  addTruss3dNode: (branch: boolean) => void;
  deleteSelectedTruss3dNode: () => void;
  handleToggleTruss3dLinkMode: () => void;
  toggleTruss3dMemberFromDraft: () => void;
  deleteSelectedTruss3dElement: () => void;
  cloneSelectedTruss3dNodes: (mirrorAxis: "x" | "y" | "z" | null) => void;
  handleUndo: () => void;
  handleRedo: () => void;
  updateSelectedTruss3dNode: (field: "x" | "y" | "z" | "load_x" | "load_y" | "load_z" | "fix_x" | "fix_y" | "fix_z", value: number | boolean) => void;
  nudgeSelectedTruss3dNodes: (axis: "x" | "y" | "z", delta: number) => void;
  setTruss3dNudgeStep: (updater: number) => void;
  updateSelectedTruss3dNodes: (field: "fix_x" | "fix_y" | "fix_z", value: boolean) => void;
  setTruss3dBatchLoadX: (value: number) => void;
  setTruss3dBatchLoadY: (value: number) => void;
  setTruss3dBatchLoadZ: (value: number) => void;
  applySelectedTruss3dLoads: (mode: "apply" | "clear") => void;
};

export function WorkbenchViewportDock({
  t,
  immersiveViewport,
  immersiveToolDrawerOpen,
  immersiveHelpDrawerOpen,
  showShortcutHints,
  showViewportToolStrip,
  immersiveToolTab,
  truss3dViewPreset,
  selectedNode,
  selectedElement,
  selectedTruss3dNodes,
  selectedTruss3dNodeData,
  truss3dLinkMode,
  truss3dProjectionMode,
  truss3dShowGrid,
  truss3dShowLabels,
  truss3dShowNodes,
  truss3dBoxSelectMode,
  truss3dNudgeStep,
  truss3dBatchLoadX,
  truss3dBatchLoadY,
  truss3dBatchLoadZ,
  undoStack,
  redoStack,
  truss3dModel,
  setImmersiveToolTab,
  handleTruss3dViewPresetChange,
  handleTruss3dFocusViewport,
  setTruss3dFocusRequestVersion,
  handleTruss3dProjectionModeChange,
  setTruss3dShowGrid,
  setTruss3dShowLabels,
  setTruss3dShowNodes,
  handleTruss3dBoxSelectModeChange,
  handleTruss3dResetViewport,
  addTruss3dNode,
  deleteSelectedTruss3dNode,
  handleToggleTruss3dLinkMode,
  toggleTruss3dMemberFromDraft,
  deleteSelectedTruss3dElement,
  cloneSelectedTruss3dNodes,
  handleUndo,
  handleRedo,
  updateSelectedTruss3dNode,
  nudgeSelectedTruss3dNodes,
  setTruss3dNudgeStep,
  updateSelectedTruss3dNodes,
  setTruss3dBatchLoadX,
  setTruss3dBatchLoadY,
  setTruss3dBatchLoadZ,
  applySelectedTruss3dLoads,
}: WorkbenchViewportDockProps) {
  return (
    <>
      {immersiveViewport && immersiveToolDrawerOpen ? (
        <section className="viewport-dock__card">
          <div className="card-head">
            <h2>{t.immersiveTools}</h2>
            <span>{t.kinds.truss_3d}</span>
          </div>
          {showViewportToolStrip ? (
            <div className="viewport-toolbar-strip viewport-toolbar-strip--dock" role="toolbar" aria-label={t.immersiveViewTools}>
              {(["iso", "front", "right", "top"] as const).map((preset) => (
                <button
                  key={preset}
                  className={`ghost-button ghost-button--compact${truss3dViewPreset === preset ? " ghost-button--active" : ""}`}
                  onClick={() => handleTruss3dViewPresetChange(preset)}
                  type="button"
                >
                  {preset === "iso" ? "ISO" : preset === "front" ? "FR" : preset === "right" ? "RT" : "TP"}
                </button>
              ))}
              <button
                className={`ghost-button ghost-button--compact${selectedNode !== null || selectedTruss3dNodes.length > 0 ? " ghost-button--active" : ""}`}
                onClick={handleTruss3dFocusViewport}
                type="button"
              >
                FOCUS
              </button>
              <button
                className={`ghost-button ghost-button--compact${selectedTruss3dNodes.length > 0 ? " ghost-button--active" : ""}`}
                onClick={() => setTruss3dFocusRequestVersion((current) => current + 1)}
                type="button"
              >
                {t.frameSelection}
              </button>
              <button
                className={`ghost-button ghost-button--compact${truss3dProjectionMode === "persp" ? " ghost-button--active" : ""}`}
                onClick={() => handleTruss3dProjectionModeChange(truss3dProjectionMode === "ortho" ? "persp" : "ortho")}
                type="button"
              >
                {truss3dProjectionMode === "ortho" ? "TO PERSP" : "TO ORTHO"}
              </button>
              <button className={`ghost-button ghost-button--compact${truss3dShowGrid ? " ghost-button--active" : ""}`} onClick={() => setTruss3dShowGrid((current) => !current)} type="button">
                GRID
              </button>
              <button className={`ghost-button ghost-button--compact${truss3dShowLabels ? " ghost-button--active" : ""}`} onClick={() => setTruss3dShowLabels((current) => !current)} type="button">
                LABEL
              </button>
              <button className={`ghost-button ghost-button--compact${truss3dShowNodes ? " ghost-button--active" : ""}`} onClick={() => setTruss3dShowNodes((current) => !current)} type="button">
                NODE
              </button>
              <button className={`ghost-button ghost-button--compact${truss3dBoxSelectMode ? " ghost-button--active" : ""}`} onClick={() => handleTruss3dBoxSelectModeChange(!truss3dBoxSelectMode)} type="button">
                BOX
              </button>
              <button className="ghost-button ghost-button--compact" onClick={handleTruss3dResetViewport} type="button">
                RESET
              </button>
            </div>
          ) : null}
          <div className="panel-tabs viewport-dock__tabs">
            <button className={`panel-tab${immersiveToolTab === "node" ? " panel-tab--active" : ""}`} onClick={() => setImmersiveToolTab("node")} type="button">
              {t.immersiveNodeOps}
            </button>
            <button className={`panel-tab${immersiveToolTab === "props" ? " panel-tab--active" : ""}`} onClick={() => setImmersiveToolTab("props")} type="button">
              {t.immersiveQuickProps}
            </button>
          </div>
          <div className="viewport-dock__stack">
            {immersiveViewport && immersiveToolTab === "node" ? (
              <div>
                <div className="card-subhead">
                  <strong>{t.immersiveNodeOps}</strong>
                  <span>{selectedTruss3dNodes.length > 1 ? `${selectedTruss3dNodes.length} ${t.nodes}` : selectedTruss3dNodeData?.id ?? t.none}</span>
                </div>
                <div className="button-row">
                  <button className="ghost-button ghost-button--compact" onClick={() => addTruss3dNode(false)} type="button">{t.addNode}</button>
                  <button className="ghost-button ghost-button--compact" disabled={selectedNode === null} onClick={() => addTruss3dNode(true)} type="button">{t.addBranchNode}</button>
                  <button className="ghost-button ghost-button--compact" disabled={selectedNode === null} onClick={deleteSelectedTruss3dNode} type="button">{t.deleteNode}</button>
                </div>
                <div className="button-row">
                  <button className={`ghost-button ghost-button--compact${truss3dLinkMode ? " ghost-button--active" : ""}`} onClick={handleToggleTruss3dLinkMode} type="button">
                    {truss3dLinkMode ? t.linkModeActive : t.linkMode}
                  </button>
                  <button className="ghost-button ghost-button--compact" onClick={toggleTruss3dMemberFromDraft} type="button">{t.toggleMember}</button>
                  <button className="ghost-button ghost-button--compact" disabled={selectedElement === null} onClick={deleteSelectedTruss3dElement} type="button">{t.deleteMember}</button>
                </div>
                <div className="button-row">
                  <button className="ghost-button ghost-button--compact" disabled={selectedNode === null && selectedTruss3dNodes.length === 0} onClick={() => cloneSelectedTruss3dNodes(null)} type="button">{t.duplicateNodes}</button>
                  <button className="ghost-button ghost-button--compact" disabled={selectedNode === null && selectedTruss3dNodes.length === 0} onClick={() => cloneSelectedTruss3dNodes("x")} type="button">{t.mirrorX}</button>
                  <button className="ghost-button ghost-button--compact" disabled={selectedNode === null && selectedTruss3dNodes.length === 0} onClick={() => cloneSelectedTruss3dNodes("y")} type="button">{t.mirrorY}</button>
                  <button className="ghost-button ghost-button--compact" disabled={selectedNode === null && selectedTruss3dNodes.length === 0} onClick={() => cloneSelectedTruss3dNodes("z")} type="button">{t.mirrorZ}</button>
                </div>
                <div className="button-row">
                  <button className="ghost-button ghost-button--compact" disabled={undoStack.length === 0} onClick={handleUndo} type="button">{t.undo}</button>
                  <button className="ghost-button ghost-button--compact" disabled={redoStack.length === 0} onClick={handleRedo} type="button">{t.redo}</button>
                </div>
                <p className="card-copy">{truss3dLinkMode ? t.linkModeIdle : t.selectionHint}</p>
              </div>
            ) : null}
            {immersiveViewport && immersiveToolTab === "props" ? (
              <div>
                <div className="card-subhead">
                  <strong>{t.immersiveQuickProps}</strong>
                  <span>{selectedTruss3dNodes.length > 1 ? `${selectedTruss3dNodes.length} ${t.immersiveNodeSelection}` : selectedTruss3dNodeData?.id ?? t.none}</span>
                </div>
                {selectedTruss3dNodeData ? (
                  <>
                    <div className="form-grid compact">
                      <label><span>{t.nodeX}</span><input type="number" step={0.1} value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.x ?? selectedTruss3dNodeData.x} onChange={(event) => updateSelectedTruss3dNode("x", Number(event.target.value))} /></label>
                      <label><span>{t.nodeY}</span><input type="number" step={0.1} value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.y ?? selectedTruss3dNodeData.y} onChange={(event) => updateSelectedTruss3dNode("y", Number(event.target.value))} /></label>
                      <label><span>{t.nodeZ}</span><input type="number" step={0.1} value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.z ?? selectedTruss3dNodeData.z} onChange={(event) => updateSelectedTruss3dNode("z", Number(event.target.value))} /></label>
                      <label><span>{t.loadX}</span><input type="number" step={100} value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.load_x ?? 0} onChange={(event) => updateSelectedTruss3dNode("load_x", Number(event.target.value))} /></label>
                      <label><span>{t.loadY}</span><input type="number" step={100} value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.load_y ?? 0} onChange={(event) => updateSelectedTruss3dNode("load_y", Number(event.target.value))} /></label>
                      <label><span>{t.loadZ}</span><input type="number" step={100} value={truss3dModel.nodes[selectedTruss3dNodeData.index]?.load_z ?? 0} onChange={(event) => updateSelectedTruss3dNode("load_z", Number(event.target.value))} /></label>
                    </div>
                    <div className="button-row">
                      <button className={`ghost-button ghost-button--compact${truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_x ? " ghost-button--active" : ""}`} onClick={() => updateSelectedTruss3dNode("fix_x", !(truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_x ?? false))} type="button">{t.fixX}</button>
                      <button className={`ghost-button ghost-button--compact${truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_y ? " ghost-button--active" : ""}`} onClick={() => updateSelectedTruss3dNode("fix_y", !(truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_y ?? false))} type="button">{t.fixY}</button>
                      <button className={`ghost-button ghost-button--compact${truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_z ? " ghost-button--active" : ""}`} onClick={() => updateSelectedTruss3dNode("fix_z", !(truss3dModel.nodes[selectedTruss3dNodeData.index]?.fix_z ?? false))} type="button">{t.fixZ}</button>
                    </div>
                    <div className="card-subhead">
                      <strong>{t.immersiveTransform}</strong>
                      <span>{t.nudgeStep}: {fixed(truss3dNudgeStep, 2)}</span>
                    </div>
                    <div className="button-row">
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("x", -truss3dNudgeStep)} type="button">X-</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("x", truss3dNudgeStep)} type="button">X+</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("y", -truss3dNudgeStep)} type="button">Y-</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("y", truss3dNudgeStep)} type="button">Y+</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("z", -truss3dNudgeStep)} type="button">Z-</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("z", truss3dNudgeStep)} type="button">Z+</button>
                    </div>
                    <label className="inline-field">
                      <span>{t.nudgeStep}</span>
                      <input type="number" min={0.01} step={0.05} value={truss3dNudgeStep} onChange={(event) => setTruss3dNudgeStep(Math.max(0.01, Number(event.target.value) || 0.01))} />
                    </label>
                  </>
                ) : selectedTruss3dNodes.length > 1 ? (
                  <>
                    <div className="button-row">
                      <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_x", true)} type="button">{t.fixX}</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_y", true)} type="button">{t.fixY}</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_z", true)} type="button">{t.fixZ}</button>
                    </div>
                    <div className="button-row">
                      <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_x", false)} type="button">{t.releaseX}</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_y", false)} type="button">{t.releaseY}</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => updateSelectedTruss3dNodes("fix_z", false)} type="button">{t.releaseZ}</button>
                    </div>
                    <div className="card-subhead">
                      <strong>{t.immersiveTransform}</strong>
                      <span>{t.nudgeStep}: {fixed(truss3dNudgeStep, 2)}</span>
                    </div>
                    <div className="button-row">
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("x", -truss3dNudgeStep)} type="button">X-</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("x", truss3dNudgeStep)} type="button">X+</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("y", -truss3dNudgeStep)} type="button">Y-</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("y", truss3dNudgeStep)} type="button">Y+</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("z", -truss3dNudgeStep)} type="button">Z-</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => nudgeSelectedTruss3dNodes("z", truss3dNudgeStep)} type="button">Z+</button>
                    </div>
                    <label className="inline-field">
                      <span>{t.nudgeStep}</span>
                      <input type="number" min={0.01} step={0.05} value={truss3dNudgeStep} onChange={(event) => setTruss3dNudgeStep(Math.max(0.01, Number(event.target.value) || 0.01))} />
                    </label>
                    <div className="card-subhead">
                      <strong>{t.immersiveLoads}</strong>
                      <span>{selectedTruss3dNodes.length} {t.nodes}</span>
                    </div>
                    <div className="form-grid compact">
                      <label><span>{t.loadX}</span><input type="number" step={100} value={truss3dBatchLoadX} onChange={(event) => setTruss3dBatchLoadX(Number(event.target.value))} /></label>
                      <label><span>{t.loadY}</span><input type="number" step={100} value={truss3dBatchLoadY} onChange={(event) => setTruss3dBatchLoadY(Number(event.target.value))} /></label>
                      <label><span>{t.loadZ}</span><input type="number" step={100} value={truss3dBatchLoadZ} onChange={(event) => setTruss3dBatchLoadZ(Number(event.target.value))} /></label>
                    </div>
                    <div className="button-row">
                      <button className="ghost-button ghost-button--compact" onClick={() => applySelectedTruss3dLoads("apply")} type="button">{t.applyLoads}</button>
                      <button className="ghost-button ghost-button--compact" onClick={() => applySelectedTruss3dLoads("clear")} type="button">{t.clearLoads}</button>
                    </div>
                    <p className="card-copy">{t.selectionHint}</p>
                  </>
                ) : (
                  <p className="card-copy">{t.immersiveNoNodeSelection}</p>
                )}
              </div>
            ) : null}
          </div>
        </section>
      ) : null}
      {showShortcutHints && immersiveHelpDrawerOpen ? (
        <section className="viewport-dock__card viewport-dock__card--help">
          <div className="card-head">
            <h2>{t.immersiveHelp}</h2>
            <span>{t.kinds.truss_3d}</span>
          </div>
          <div className="viewport-help-list">
            {t.shortcutLegendRows.map((row) => (
              <p key={row} className="card-copy">
                {row}
              </p>
            ))}
          </div>
        </section>
      ) : null}
    </>
  );
}
