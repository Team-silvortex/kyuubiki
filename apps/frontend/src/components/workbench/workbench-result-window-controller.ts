"use client";

import { useEffect, useRef, type Dispatch, type RefObject, type SetStateAction, type UIEvent as ReactUIEvent } from "react";
import type { WorkbenchAlertItem } from "@/components/workbench/workbench-alert-strip";
import { dismissWorkbenchAlert, upsertWorkbenchAlert } from "@/components/workbench/workbench-alert-state";
import { strategyResultWindowLimit, type ViewportRenderStrategy } from "@/components/workbench/workbench-render-diagnostics";
import { fetchDirectMeshResultChunk, fetchResultChunk, type FrontendRuntimeMode, type ResultChunkPayload } from "@/lib/api";
import {
  clampChunkOffset,
  chunkCacheKey,
  computeResultWindowSize,
  computeVisibleResultWindowOffset,
  readChunkCache,
  RESULT_WINDOW_BASE_SIZE,
  RESULT_WINDOW_THRESHOLD,
  writeChunkCache,
} from "@/lib/workbench/result-window";

type StudyKind =
  | "axial_bar_1d"
  | "heat_bar_1d"
  | "electrostatic_plane_triangle_2d"
  | "electrostatic_plane_quad_2d"
  | "heat_plane_triangle_2d"
  | "heat_plane_quad_2d"
  | "thermal_bar_1d"
  | "thermal_beam_1d"
  | "thermal_frame_2d"
  | "thermal_truss_2d"
  | "thermal_truss_3d"
  | "thermal_plane_triangle_2d"
  | "thermal_plane_quad_2d"
  | "spring_1d"
  | "spring_2d"
  | "spring_3d"
  | "beam_1d"
  | "torsion_1d"
  | "truss_2d"
  | "truss_3d"
  | "plane_triangle_2d"
  | "plane_quad_2d"
  | "frame_2d";

export type ResultWindowState = {
  jobId: string;
  studyKind: Exclude<StudyKind, "axial_bar_1d">;
  nodes: Record<string, unknown>[];
  elements: Record<string, unknown>[];
  totalNodes: number;
  totalElements: number;
  limit: number;
};

type ResultWindowGuards = {
  isAxialResult: (value: unknown) => boolean;
  isTrussResult: (value: unknown) => boolean;
  isHeatBar1dResult: (value: unknown) => boolean;
  isElectrostaticPlaneQuad2dResult: (value: unknown) => boolean;
  isElectrostaticPlaneTriangle2dResult: (value: unknown) => boolean;
  isHeatPlaneQuad2dResult: (value: unknown) => boolean;
  isHeatPlaneTriangle2dResult: (value: unknown) => boolean;
  isThermalBar1dResult: (value: unknown) => boolean;
  isThermalBeam1dResult: (value: unknown) => boolean;
  isThermalTruss2dResult: (value: unknown) => boolean;
  isThermalTruss3dResult: (value: unknown) => boolean;
  isTruss3dResult: (value: unknown) => boolean;
  isSpring1dResult: (value: unknown) => boolean;
  isSpring2dResult: (value: unknown) => boolean;
  isSpring3dResult: (value: unknown) => boolean;
  isBeam1dResult: (value: unknown) => boolean;
  isTorsion1dResult: (value: unknown) => boolean;
  isFrame2dResult: (value: unknown) => boolean;
};

type UseWorkbenchResultWindowControllerArgs = {
  canvasStageRef: RefObject<HTMLDivElement | null>;
  canvasViewportWidth: number;
  frontendRuntimeMode: FrontendRuntimeMode;
  guards: ResultWindowGuards;
  jobId: string | null;
  requestTimedOutLabel: string;
  result: unknown;
  resultWindow: ResultWindowState | null;
  resultWindowLimit: number;
  renderStrategy: ViewportRenderStrategy;
  resultWindowMaxTotal: number;
  resultWindowOffset: number;
  setCanvasViewportWidth: Dispatch<SetStateAction<number>>;
  setMessage: Dispatch<SetStateAction<string>>;
  setSystemAlerts: Dispatch<SetStateAction<WorkbenchAlertItem[]>>;
  setResultWindow: Dispatch<SetStateAction<ResultWindowState | null>>;
  setResultWindowLimit: Dispatch<SetStateAction<number>>;
  setResultWindowOffset: Dispatch<SetStateAction<number>>;
  studyKind: StudyKind;
};

function resolveResultWindowStudyKind(
  result: unknown,
  studyKind: StudyKind,
  guards: ResultWindowGuards,
): Exclude<StudyKind, "axial_bar_1d"> {
  return guards.isTrussResult(result)
    ? "truss_2d"
    : guards.isHeatBar1dResult(result)
      ? "heat_bar_1d"
      : guards.isElectrostaticPlaneQuad2dResult(result)
        ? "electrostatic_plane_quad_2d"
        : guards.isElectrostaticPlaneTriangle2dResult(result)
          ? "electrostatic_plane_triangle_2d"
      : guards.isHeatPlaneQuad2dResult(result)
        ? "heat_plane_quad_2d"
        : guards.isHeatPlaneTriangle2dResult(result)
          ? "heat_plane_triangle_2d"
          : guards.isThermalBar1dResult(result)
            ? "thermal_bar_1d"
            : guards.isThermalBeam1dResult(result)
              ? "thermal_beam_1d"
              : guards.isThermalTruss2dResult(result)
                ? "thermal_truss_2d"
                : guards.isThermalTruss3dResult(result)
                  ? "thermal_truss_3d"
                  : guards.isTruss3dResult(result)
                    ? "truss_3d"
                    : guards.isSpring1dResult(result)
                      ? "spring_1d"
                      : guards.isSpring2dResult(result)
                        ? "spring_2d"
                        : guards.isSpring3dResult(result)
                          ? "spring_3d"
                          : guards.isBeam1dResult(result)
                            ? "beam_1d"
                            : guards.isTorsion1dResult(result)
                              ? "torsion_1d"
                              : guards.isFrame2dResult(result)
                                ? "frame_2d"
                                : studyKind === "plane_quad_2d"
                                  ? "plane_quad_2d"
                                  : "plane_triangle_2d";
}

export function useWorkbenchResultWindowController({
  canvasStageRef,
  canvasViewportWidth,
  frontendRuntimeMode,
  guards,
  jobId,
  requestTimedOutLabel,
  result,
  resultWindow,
  resultWindowLimit,
  renderStrategy,
  resultWindowMaxTotal,
  resultWindowOffset,
  setCanvasViewportWidth,
  setMessage,
  setSystemAlerts,
  setResultWindow,
  setResultWindowLimit,
  setResultWindowOffset,
  studyKind,
}: UseWorkbenchResultWindowControllerArgs) {
  const {
    isAxialResult,
    isTrussResult,
    isHeatBar1dResult,
    isElectrostaticPlaneQuad2dResult,
    isElectrostaticPlaneTriangle2dResult,
    isHeatPlaneQuad2dResult,
    isHeatPlaneTriangle2dResult,
    isThermalBar1dResult,
    isThermalBeam1dResult,
    isThermalTruss2dResult,
    isThermalTruss3dResult,
    isTruss3dResult,
    isSpring1dResult,
    isSpring2dResult,
    isSpring3dResult,
    isBeam1dResult,
    isTorsion1dResult,
    isFrame2dResult,
  } = guards;
  const chunkScrollFrameRef = useRef<number | null>(null);
  const chunkScrollLeftRef = useRef(0);
  const chunkScrollDirectionRef = useRef<-1 | 0 | 1>(0);
  const chunkCacheRef = useRef<Map<string, ResultChunkPayload<Record<string, unknown>>>>(new Map());

  useEffect(() => {
    setResultWindowOffset(0);
    setResultWindowLimit(strategyResultWindowLimit(RESULT_WINDOW_BASE_SIZE, renderStrategy));
    chunkCacheRef.current.clear();
  }, [jobId, renderStrategy, setResultWindowLimit, setResultWindowOffset, studyKind]);

  useEffect(() => {
    return () => {
      if (chunkScrollFrameRef.current !== null) {
        window.cancelAnimationFrame(chunkScrollFrameRef.current);
      }
    };
  }, []);

  useEffect(() => {
    const syncCanvasViewportWidth = () => {
      const nextWidth = canvasStageRef.current?.clientWidth ?? 980;
      setCanvasViewportWidth((current) => (current === nextWidth ? current : nextWidth));
    };

    syncCanvasViewportWidth();
    window.addEventListener("resize", syncCanvasViewportWidth);

    return () => {
      window.removeEventListener("resize", syncCanvasViewportWidth);
    };
  }, [canvasStageRef, setCanvasViewportWidth]);

  useEffect(() => {
    if (!jobId || !result || isAxialResult(result)) {
      dismissWorkbenchAlert(setSystemAlerts, "result-window-timeout");
      setResultWindow(null);
      return;
    }

    const totalNodes = Array.isArray((result as { nodes?: unknown[] }).nodes) ? (result as { nodes: unknown[] }).nodes.length : 0;
    const totalElements = Array.isArray((result as { elements?: unknown[] }).elements)
      ? (result as { elements: unknown[] }).elements.length
      : 0;
    const totalItems = Math.max(totalNodes, totalElements);

    if (totalNodes <= RESULT_WINDOW_THRESHOLD && totalElements <= RESULT_WINDOW_THRESHOLD) {
      dismissWorkbenchAlert(setSystemAlerts, "result-window-timeout");
      setResultWindow(null);
      return;
    }

    const limit = strategyResultWindowLimit(
      computeResultWindowSize(totalItems, canvasViewportWidth),
      renderStrategy,
      totalItems,
    );
    if (resultWindowLimit !== limit) {
      setResultWindowLimit(limit);
    }

    const nextStudyKind = resolveResultWindowStudyKind(result, studyKind, {
      isAxialResult,
      isTrussResult,
      isHeatBar1dResult,
      isElectrostaticPlaneQuad2dResult,
      isElectrostaticPlaneTriangle2dResult,
      isHeatPlaneQuad2dResult,
      isHeatPlaneTriangle2dResult,
      isThermalBar1dResult,
      isThermalBeam1dResult,
      isThermalTruss2dResult,
      isThermalTruss3dResult,
      isTruss3dResult,
      isSpring1dResult,
      isSpring2dResult,
      isSpring3dResult,
      isBeam1dResult,
      isTorsion1dResult,
      isFrame2dResult,
    });
    let cancelled = false;

    (async () => {
      try {
        const safeOffset = clampChunkOffset(resultWindowOffset, totalItems, limit);
        const chunkFetcher =
          frontendRuntimeMode === "direct_mesh_gui" ? fetchDirectMeshResultChunk : fetchResultChunk;
        const fetchChunk = async (kind: "nodes" | "elements", offset: number) => {
          const key = chunkCacheKey(frontendRuntimeMode, jobId, kind, offset, limit);
          const cached = readChunkCache(chunkCacheRef.current, key);
          if (cached) return cached;

          const chunk = await chunkFetcher(jobId, kind, { offset, limit });
          writeChunkCache(chunkCacheRef.current, key, chunk as ResultChunkPayload<Record<string, unknown>>);
          return chunk;
        };

        const nodesKey = chunkCacheKey(frontendRuntimeMode, jobId, "nodes", safeOffset, limit);
        const elementsKey = chunkCacheKey(frontendRuntimeMode, jobId, "elements", safeOffset, limit);
        const cachedNodes = readChunkCache(chunkCacheRef.current, nodesKey);
        const cachedElements = readChunkCache(chunkCacheRef.current, elementsKey);

        if (cachedNodes && cachedElements) {
          setResultWindow({
            jobId,
            studyKind: nextStudyKind,
            nodes: cachedNodes.items,
            elements: cachedElements.items,
            totalNodes: cachedNodes.total,
            totalElements: cachedElements.total,
            limit,
          });
        }

        const [nodesChunk, elementsChunk] = await Promise.all([
          fetchChunk("nodes", safeOffset),
          fetchChunk("elements", safeOffset),
        ]);

        if (cancelled) return;

        dismissWorkbenchAlert(setSystemAlerts, "result-window-timeout");
        setResultWindow({
          jobId,
          studyKind: nextStudyKind,
          nodes: nodesChunk.items,
          elements: elementsChunk.items,
          totalNodes: nodesChunk.total,
          totalElements: elementsChunk.total,
          limit,
        });

        const directionalOffsets =
          chunkScrollDirectionRef.current > 0
            ? [safeOffset + limit, safeOffset + limit * 2, safeOffset - limit]
            : chunkScrollDirectionRef.current < 0
              ? [safeOffset - limit, safeOffset - limit * 2, safeOffset + limit]
              : [safeOffset - limit, safeOffset + limit];

        const prefetchOffsets = directionalOffsets
          .map((offset) => clampChunkOffset(offset, totalItems, limit))
          .filter((offset, index, values) => offset !== safeOffset && values.indexOf(offset) === index);

        void Promise.all(
          prefetchOffsets.flatMap((offset) => [
            fetchChunk("nodes", offset),
            fetchChunk("elements", offset),
          ]),
        ).catch(() => undefined);
      } catch (error) {
        if (!cancelled && error instanceof Error && error.message.startsWith("request timed out:")) {
          upsertWorkbenchAlert(setSystemAlerts, {
            id: "result-window-timeout",
            message: requestTimedOutLabel,
            tone: "warning",
          });
          setMessage(requestTimedOutLabel);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [
    canvasViewportWidth,
    frontendRuntimeMode,
    jobId,
    isAxialResult,
    isBeam1dResult,
    isFrame2dResult,
    isHeatBar1dResult,
    isElectrostaticPlaneQuad2dResult,
    isElectrostaticPlaneTriangle2dResult,
    isHeatPlaneQuad2dResult,
    isHeatPlaneTriangle2dResult,
    isSpring1dResult,
    isSpring2dResult,
    isSpring3dResult,
    isThermalBar1dResult,
    isThermalBeam1dResult,
    isThermalTruss2dResult,
    isThermalTruss3dResult,
    isTorsion1dResult,
    isTruss3dResult,
    isTrussResult,
    requestTimedOutLabel,
    result,
    resultWindowLimit,
    renderStrategy,
    resultWindowOffset,
    setMessage,
    setSystemAlerts,
    setResultWindow,
    setResultWindowLimit,
    studyKind,
  ]);

  useEffect(() => {
    if (!resultWindow) return;

    const totalItems = Math.max(resultWindow.totalNodes, resultWindow.totalElements);
    const nextOffset = clampChunkOffset(resultWindowOffset, totalItems, resultWindowLimit);

    if (nextOffset !== resultWindowOffset) {
      setResultWindowOffset(nextOffset);
    }
  }, [resultWindow, resultWindowLimit, resultWindowOffset, setResultWindowOffset]);

  const handleCanvasStageScroll = (event: ReactUIEvent<HTMLDivElement>) => {
    if (!resultWindow) return;
    if (chunkScrollFrameRef.current !== null) return;

    const target = event.currentTarget;
    if (target.clientWidth > 0) {
      setCanvasViewportWidth((current) => (current === target.clientWidth ? current : target.clientWidth));
    }
    const previousLeft = chunkScrollLeftRef.current;
    chunkScrollDirectionRef.current =
      target.scrollLeft > previousLeft ? 1 : target.scrollLeft < previousLeft ? -1 : 0;
    chunkScrollLeftRef.current = target.scrollLeft;

    chunkScrollFrameRef.current = window.requestAnimationFrame(() => {
      chunkScrollFrameRef.current = null;

      const maxScrollLeft = Math.max(0, target.scrollWidth - target.clientWidth);
      if (maxScrollLeft <= 0) return;

      const nextOffset = computeVisibleResultWindowOffset(
        resultWindowMaxTotal,
        resultWindowLimit,
        target.clientWidth,
        target.scrollLeft,
        target.scrollWidth,
      );

      setResultWindowOffset((current) => (current === nextOffset ? current : nextOffset));
    });
  };

  return { handleCanvasStageScroll };
}
