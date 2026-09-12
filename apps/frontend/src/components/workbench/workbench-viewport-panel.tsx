"use client";

import { useEffect, useLayoutEffect, useRef, type ReactNode, type RefObject, type UIEvent as ReactUIEvent } from "react";
import { observeWorkbenchViewportFit } from "./workbench-viewport-fit";
import { installImmersiveDockResize } from "./workbench-immersive-dock-resize";
import type { ImmersiveDockPreferences } from "./workbench-immersive-dock-layout";
import { getWorkbenchPanelLayoutCopy } from "./workbench-panel-layout-copy";

type WorkbenchViewportPanelProps = {
  viewportPanelRef: RefObject<HTMLElement | null>;
  immersiveViewport: boolean;
  title: string;
  language?: string;
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
  language,
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
  const layoutRef = useRef<HTMLDivElement>(null), resizeRef = useRef<HTMLDivElement>(null);
  const dockPreferences = useRef<ImmersiveDockPreferences>({});
  const resizable = immersiveViewport && hasViewportDock;
  useLayoutEffect(() => {
    if (resizable && layoutRef.current && resizeRef.current) {
      return installImmersiveDockResize(layoutRef.current, resizeRef.current, dockPreferences);
    }
  }, [resizable]);
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
      <div ref={layoutRef} className={`canvas-layout${hasViewportDock ? " canvas-layout--split" : ""}`} data-immersive-resizable={resizable || undefined}>
        {hasViewportDock ? (
          <div className="canvas-layout__dock">
            <div className="viewport-dock">{dockContent}</div>
          </div>
        ) : null}
        {resizable ? <div ref={resizeRef} className="immersive-dock-resize" data-workbench-immersive-resize="true"
          role="separator" tabIndex={0} aria-label="workbench-immersive-resize" aria-orientation="vertical"
          title={getWorkbenchPanelLayoutCopy(language).resize} /> : null}
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
