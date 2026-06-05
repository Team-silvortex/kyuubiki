"use client";

import { useEffect, useRef } from "react";
import { heartbeatTone } from "@/components/workbench/workbench-result-helpers";
import type {
  BeamResultField,
  FrameResultField,
  StudyKind,
  SystemPanelTab,
} from "@/components/workbench/workbench-types";
import type { JobEnvelope, ProjectRecord } from "@/lib/api";

export function useWorkbenchRuntimeGuards(params: {
  beamResultField: BeamResultField;
  focusedFrameElement: number | null;
  focusedPlaneElement: number | null;
  immersiveGuardrails: boolean;
  immersiveViewport: boolean;
  job: JobEnvelope["job"] | null;
  language: string;
  frameResultField: FrameResultField;
  projects: ProjectRecord[];
  selectedProjectId: string | null;
  setAssistantWindowOpen: (value: boolean) => void;
  setBeamResultField: (value: BeamResultField) => void;
  setFocusedFrameElement: (value: number | null) => void;
  setFocusedPlaneElement: (value: number | null) => void;
  setFrameResultField: (value: FrameResultField) => void;
  setImmersiveHelpDrawerOpen: (value: boolean) => void;
  setImmersiveToolDrawerOpen: (value: boolean) => void;
  setImmersiveViewport: (value: boolean) => void;
  setMessage: (value: string) => void;
  setProjectDescriptionDraft: (value: string) => void;
  setProjectNameDraft: (value: string) => void;
  setSystemPanelTab: (value: SystemPanelTab) => void;
  studyKind: StudyKind;
  systemPanelTab: SystemPanelTab;
  t: {
    defaultProject: string;
    initialFailed: string;
  };
  viewportPanelRef: React.RefObject<HTMLElement | null>;
}) {
  const {
    beamResultField,
    focusedFrameElement,
    focusedPlaneElement,
    immersiveGuardrails,
    immersiveViewport,
    job,
    language,
    frameResultField,
    projects,
    selectedProjectId,
    setAssistantWindowOpen,
    setBeamResultField,
    setFocusedFrameElement,
    setFocusedPlaneElement,
    setFrameResultField,
    setImmersiveHelpDrawerOpen,
    setImmersiveToolDrawerOpen,
    setImmersiveViewport,
    setMessage,
    setProjectDescriptionDraft,
    setProjectNameDraft,
    setSystemPanelTab,
    studyKind,
    systemPanelTab,
    t,
    viewportPanelRef,
  } = params;

  const staleHeartbeatAlertedRef = useRef<string | null>(null);
  const dragHistoryCapturedRef = useRef(false);
  const drag3dHistoryCapturedRef = useRef(false);
  const dragFrameRef = useRef<number | null>(null);
  const pendingDragPointRef = useRef<{ x: number; y: number } | null>(null);
  const canvasStageRef = useRef<HTMLDivElement | null>(null);
  const resultRefreshSeqRef = useRef(0);
  const jobPollTokenRef = useRef(0);

  useEffect(() => {
    if (systemPanelTab === "assistant") {
      setAssistantWindowOpen(true);
      setSystemPanelTab("config");
    }
  }, [setAssistantWindowOpen, setSystemPanelTab, systemPanelTab]);

  useEffect(() => {
    if (!job?.job_id) {
      staleHeartbeatAlertedRef.current = null;
      return;
    }

    const tone = heartbeatTone(job);
    if (tone === "stale") {
      if (staleHeartbeatAlertedRef.current !== job.job_id) {
        staleHeartbeatAlertedRef.current = job.job_id;
        setMessage(
          language === "zh"
            ? `任务 ${job.job_id.slice(0, 8)} 心跳已过期，请检查 agent 或考虑取消任务。`
            : `Job ${job.job_id.slice(0, 8)} heartbeat is stale. Check the agent or consider cancelling the run.`,
        );
      }
      return;
    }

    if (staleHeartbeatAlertedRef.current === job.job_id) {
      staleHeartbeatAlertedRef.current = null;
    }
  }, [job, language, setMessage]);

  useEffect(() => {
    if (studyKind === "torsion_1d" && frameResultField === "max_combined_stress") {
      setFrameResultField("max_bending_stress");
    }
    if (
      studyKind !== "thermal_frame_2d" &&
      (frameResultField === "average_temperature_delta" ||
        frameResultField === "temperature_gradient_y" ||
        frameResultField === "thermal_curvature")
    ) {
      setFrameResultField("max_combined_stress");
    }
  }, [frameResultField, setFrameResultField, studyKind]);

  useEffect(() => {
    if (
      studyKind !== "thermal_beam_1d" &&
      (beamResultField === "temperature_gradient_y" || beamResultField === "thermal_curvature")
    ) {
      setBeamResultField("max_bending_stress");
    }
  }, [beamResultField, setBeamResultField, studyKind]);

  useEffect(() => {
    return () => {
      if (dragFrameRef.current !== null) {
        window.cancelAnimationFrame(dragFrameRef.current);
      }
    };
  }, []);

  useEffect(() => {
    return () => {
      jobPollTokenRef.current += 1;
    };
  }, []);

  useEffect(() => {
    const handleFullscreenChange = () => {
      setImmersiveViewport(document.fullscreenElement === viewportPanelRef.current);
    };

    document.addEventListener("fullscreenchange", handleFullscreenChange);
    return () => document.removeEventListener("fullscreenchange", handleFullscreenChange);
  }, [setImmersiveViewport, viewportPanelRef]);

  useEffect(() => {
    if (studyKind === "truss_3d") return;

    setImmersiveToolDrawerOpen(false);
    setImmersiveHelpDrawerOpen(false);

    if (immersiveViewport) {
      setImmersiveViewport(false);
    }

    if (document.fullscreenElement === viewportPanelRef.current) {
      void document.exitFullscreen().catch(() => undefined);
    }
  }, [
    immersiveViewport,
    setImmersiveHelpDrawerOpen,
    setImmersiveToolDrawerOpen,
    setImmersiveViewport,
    studyKind,
    viewportPanelRef,
  ]);

  useEffect(() => {
    if (!immersiveViewport || !immersiveGuardrails) {
      document.documentElement.classList.remove("immersive-guardrails");
      return;
    }

    const stopEvent = (event: Event) => {
      event.preventDefault();
      event.stopPropagation();
    };

    const handleKeydown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && ["a", "c", "p", "s", "u", "v", "x"].includes(event.key.toLowerCase())) {
        stopEvent(event);
      }
    };

    document.documentElement.classList.add("immersive-guardrails");
    document.addEventListener("contextmenu", stopEvent, true);
    document.addEventListener("copy", stopEvent, true);
    document.addEventListener("cut", stopEvent, true);
    document.addEventListener("paste", stopEvent, true);
    document.addEventListener("selectstart", stopEvent, true);
    document.addEventListener("dragstart", stopEvent, true);
    document.addEventListener("keydown", handleKeydown, true);

    return () => {
      document.documentElement.classList.remove("immersive-guardrails");
      document.removeEventListener("contextmenu", stopEvent, true);
      document.removeEventListener("copy", stopEvent, true);
      document.removeEventListener("cut", stopEvent, true);
      document.removeEventListener("paste", stopEvent, true);
      document.removeEventListener("selectstart", stopEvent, true);
      document.removeEventListener("dragstart", stopEvent, true);
      document.removeEventListener("keydown", handleKeydown, true);
    };
  }, [immersiveViewport, immersiveGuardrails]);

  useEffect(() => {
    const selectedProject = projects.find((project) => project.project_id === selectedProjectId) ?? null;

    if (selectedProject) {
      setProjectNameDraft(selectedProject.name);
      setProjectDescriptionDraft(selectedProject.description ?? "");
    } else if (projects.length === 0) {
      setProjectNameDraft(t.defaultProject);
      setProjectDescriptionDraft("");
    }
  }, [
    projects,
    selectedProjectId,
    setProjectDescriptionDraft,
    setProjectNameDraft,
    t.defaultProject,
  ]);

  useEffect(() => {
    if (focusedPlaneElement === null) return;

    const timeout = window.setTimeout(() => {
      setFocusedPlaneElement(null);
    }, 1400);

    return () => window.clearTimeout(timeout);
  }, [focusedPlaneElement, setFocusedPlaneElement]);

  useEffect(() => {
    if (focusedFrameElement === null) return;

    const timeout = window.setTimeout(() => {
      setFocusedFrameElement(null);
    }, 1400);

    return () => window.clearTimeout(timeout);
  }, [focusedFrameElement, setFocusedFrameElement]);

  return {
    canvasStageRef,
    drag3dHistoryCapturedRef,
    dragFrameRef,
    dragHistoryCapturedRef,
    jobPollTokenRef,
    pendingDragPointRef,
    resultRefreshSeqRef,
    staleHeartbeatAlertedRef,
  };
}
