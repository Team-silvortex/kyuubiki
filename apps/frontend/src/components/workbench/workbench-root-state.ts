"use client";

import { useRef, useState, useTransition } from "react";
import { copyByLanguage } from "@/components/workbench/workbench-copy";
import { useWorkbenchRuntimeGuards } from "@/components/workbench/workbench-runtime-guards";
import {
  bindWorkbenchShellState,
  bindWorkbenchWorkspaceState,
} from "@/components/workbench/workbench-shell-bindings";
import { useWorkbenchShellState } from "@/components/workbench/workbench-shell-state";
import { useWorkbenchWorkspaceState } from "@/components/workbench/workbench-workspace-state";
import type { HealthPayload, ProtocolAgentDescriptor } from "@/lib/api";

export function useWorkbenchRootState() {
  const workspaceState = useWorkbenchWorkspaceState({
    defaultLoadedModelName: copyByLanguage.en.defaultModel,
    defaultMessage: copyByLanguage.en.initialLoaded,
    defaultProjectLabel: copyByLanguage.en.defaultProject,
  });
  const workspaceBindings = bindWorkbenchWorkspaceState(workspaceState);

  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [protocolAgents, setProtocolAgents] = useState<ProtocolAgentDescriptor[]>([]);

  const shellState = useWorkbenchShellState({
    setLoadedModelName: workspaceBindings.setLoadedModelName,
    setMessage: workspaceBindings.setMessage,
  });
  const shellBindings = bindWorkbenchShellState(shellState);

  const [isPending, startTransition] = useTransition();
  const viewportPanelRef = useRef<HTMLElement | null>(null);
  const runtimeGuards = useWorkbenchRuntimeGuards({
    beamResultField: workspaceBindings.beamResultField,
    focusedFrameElement: workspaceBindings.focusedFrameElement,
    focusedPlaneElement: workspaceBindings.focusedPlaneElement,
    immersiveGuardrails: shellBindings.immersiveGuardrails,
    immersiveViewport: shellBindings.immersiveViewport,
    job: workspaceBindings.job,
    language: shellBindings.language,
    frameResultField: workspaceBindings.frameResultField,
    frontendRuntimeMode: shellBindings.frontendRuntimeMode,
    directMeshEndpointsText: shellBindings.directMeshEndpointsText,
    controlPlaneApiToken: shellBindings.controlPlaneApiToken,
    clusterApiToken: shellBindings.clusterApiToken,
    directMeshApiToken: shellBindings.directMeshApiToken,
    protocolAgents,
    projects: workspaceBindings.projects,
    selectedProjectId: workspaceBindings.selectedProjectId,
    setAssistantWindowOpen: shellBindings.setAssistantWindowOpen,
    setBeamResultField: workspaceBindings.setBeamResultField,
    setFocusedFrameElement: workspaceBindings.setFocusedFrameElement,
    setFocusedPlaneElement: workspaceBindings.setFocusedPlaneElement,
    setFrameResultField: workspaceBindings.setFrameResultField,
    setFrontendRuntimeMode: shellBindings.setFrontendRuntimeMode,
    setImmersiveHelpDrawerOpen: shellBindings.setImmersiveHelpDrawerOpen,
    setImmersiveToolDrawerOpen: shellBindings.setImmersiveToolDrawerOpen,
    setImmersiveViewport: shellBindings.setImmersiveViewport,
    setMessage: workspaceBindings.setMessage,
    setProjectDescriptionDraft: workspaceBindings.setProjectDescriptionDraft,
    setProjectNameDraft: workspaceBindings.setProjectNameDraft,
    setSystemPanelTab: workspaceBindings.setSystemPanelTab,
    studyKind: workspaceBindings.studyKind,
    systemPanelTab: workspaceBindings.systemPanelTab,
    t: shellBindings.t,
    viewportPanelRef,
  });

  const jobIsActive =
    workspaceBindings.job?.status === "queued" ||
    workspaceBindings.job?.status === "preprocessing" ||
    workspaceBindings.job?.status === "partitioning" ||
    workspaceBindings.job?.status === "solving" ||
    workspaceBindings.job?.status === "postprocessing";

  return {
    workspaceState,
    workspaceBindings,
    shellState,
    shellBindings,
    health,
    setHealth,
    protocolAgents,
    setProtocolAgents,
    isPending,
    startTransition,
    viewportPanelRef,
    jobIsActive,
    runtimeGuards,
    ...workspaceBindings,
    ...shellBindings,
    ...runtimeGuards,
  };
}
