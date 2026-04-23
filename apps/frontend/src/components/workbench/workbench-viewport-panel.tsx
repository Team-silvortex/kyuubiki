"use client";

import type { ReactNode, RefObject, UIEvent as ReactUIEvent } from "react";

type WorkbenchViewportPanelProps = {
  viewportPanelRef: RefObject<HTMLElement | null>;
  immersiveViewport: boolean;
  title: string;
  headActions?: ReactNode;
  hasViewportDock: boolean;
  dockContent?: ReactNode;
  resultWindowBar?: ReactNode;
  isTruss3d: boolean;
  shouldStretchSpaceViewport: boolean;
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
  isTruss3d,
  shouldStretchSpaceViewport,
  onCanvasStageScroll,
  canvasStageRef,
  viewportContent,
  immersiveDrawer,
}: WorkbenchViewportPanelProps) {
  return (
    <section ref={viewportPanelRef} className={`panel canvas-panel${immersiveViewport ? " canvas-panel--immersive" : ""}`}>
      <div className="panel-head">
        <h2>{title}</h2>
        <div className="panel-head__actions">{headActions}</div>
      </div>
      <div className={`canvas-layout${hasViewportDock ? " canvas-layout--split" : ""}`}>
        {hasViewportDock ? <div className="viewport-dock">{dockContent}</div> : null}
        {resultWindowBar}
        <div
          className={`canvas-stage${isTruss3d ? " canvas-stage--space" : ""}${shouldStretchSpaceViewport ? " canvas-stage--space-fluid" : ""}`}
          onScroll={onCanvasStageScroll}
          ref={canvasStageRef}
        >
          {viewportContent}
        </div>
      </div>
      {immersiveDrawer}
    </section>
  );
}
