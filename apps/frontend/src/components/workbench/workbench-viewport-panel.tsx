"use client";

import { useEffect, type ReactNode, type RefObject, type UIEvent as ReactUIEvent } from "react";
import { observeWorkbenchViewportFit } from "./workbench-viewport-fit";

type WorkbenchViewportPanelProps = {
  viewportPanelRef: RefObject<HTMLElement | null>;
  immersiveViewport: boolean;
  title: string;
  headActions?: ReactNode;
  hasViewportDock: boolean;
  dockContent?: ReactNode;
  resultWindowBar?: ReactNode;
  diagnosticsBar?: ReactNode;
  isTruss3d: boolean;
  shouldStretchSpaceViewport: boolean;
  windowedViewport?: boolean;
  onCanvasStageScroll: (event: ReactUIEvent<HTMLDivElement>) => void;
  canvasStageRef: RefObject<HTMLDivElement | null>;
  viewportContent: ReactNode;
  immersiveDrawer?: ReactNode;
};

export function WorkbenchViewportPanel({
  viewportPanelRef,
  immersiveViewport,
  title,
  headActions,
  hasViewportDock,
  dockContent,
  resultWindowBar,
  diagnosticsBar,
  isTruss3d,
  shouldStretchSpaceViewport,
  windowedViewport = false,
  onCanvasStageScroll,
  canvasStageRef,
  viewportContent,
  immersiveDrawer,
}: WorkbenchViewportPanelProps) {
  const hasChrome = Boolean(resultWindowBar || diagnosticsBar);
  useEffect(() => {
    const stage = canvasStageRef.current;
    if (stage) return observeWorkbenchViewportFit(stage);
  }, [canvasStageRef]);

  return (
    <section
      ref={viewportPanelRef}
      className={`panel canvas-panel${immersiveViewport ? " canvas-panel--immersive" : ""}`}
      data-workbench-panel="viewport"
      data-workbench-surface="built-in"
    >
      <div className="panel-head">
        <h2>{title}</h2>
        <div className="panel-head__actions">{headActions}</div>
      </div>
      <div className={`canvas-layout${hasViewportDock ? " canvas-layout--split" : ""}`}>
        {hasViewportDock ? (
          <div className="canvas-layout__dock">
            <div className="viewport-dock">{dockContent}</div>
          </div>
        ) : null}
        <div className="canvas-layout__main">
          {hasChrome ? (
            <div className="canvas-layout__chrome">
              {resultWindowBar}
              {diagnosticsBar}
            </div>
          ) : null}
          <div className="canvas-layout__viewport">
            <div
              className={`canvas-stage${isTruss3d ? " canvas-stage--space" : ""}${shouldStretchSpaceViewport ? " canvas-stage--space-fluid" : ""}`}
              onScroll={onCanvasStageScroll}
              ref={canvasStageRef}
              data-workbench-viewport="stage"
              data-workbench-viewport-sizing={windowedViewport ? "windowed" : "fit"}
            >
              {viewportContent}
            </div>
          </div>
        </div>
      </div>
      {immersiveDrawer}
    </section>
  );
}
