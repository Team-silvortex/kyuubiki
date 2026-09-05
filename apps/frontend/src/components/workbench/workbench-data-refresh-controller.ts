import { useEffect, useLayoutEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import { createWorkbenchProjectContext, type WorkbenchProjectRefresh } from "@/lib/workbench/project-context";
import type {
  DirectMeshSelectionMode,
  FrontendRuntimeMode,
  HealthPayload,
  ProtocolAgentDescriptor,
} from "@/lib/api";
import type { ModelVersionRecord, ProjectRecord } from "@/lib/api/project-types";
import type { SecurityEventRecord } from "@/lib/api/security-results-types";
import { copyByLanguage } from "@/components/workbench/workbench-copy";
import type { SecurityEventWindow } from "@/components/workbench/workbench-types";
import type { WorkbenchSecurityAuditRisk, WorkbenchSecurityAuditSource } from "@/lib/workbench/security-audit";
import {
  clearWorkbenchRuntimeRecoveryIssue,
  upsertWorkbenchRuntimeRecoveryIssue,
  type WorkbenchRuntimeRecoveryState,
} from "@/components/workbench/workbench-runtime-recovery";
import { normalizeWorkbenchRequestError } from "@/lib/api/request-errors";
import {
  workbenchRuntimeStatusBackendService,
  type WorkbenchRuntimeStatusBackendService,
} from "@/lib/workbench/runtime-status-backend-service";
import {
  workbenchSecurityEventBackendService,
  type WorkbenchSecurityEventBackendService,
} from "@/lib/workbench/security-event-backend-service";
import {
  workbenchProjectLibraryBackendService,
  type WorkbenchProjectLibraryBackendService,
} from "@/lib/workbench/project-library-backend-service";

type UseWorkbenchDataRefreshControllerArgs = {
  directMeshEndpointsText: string;
  directMeshSelectionMode: DirectMeshSelectionMode;
  frontendRuntimeMode: FrontendRuntimeMode;
  securityEventActionFilter: string;
  securityEventRiskFilter: WorkbenchSecurityAuditRisk | "";
  securityEventSourceFilter: WorkbenchSecurityAuditSource | "hub-assistant" | "";
  securityEventStatusFilter: "" | "allowed" | "blocked";
  securityEventWindowFilter: SecurityEventWindow;
  selectedModelId: string | null;
  selectedProjectId: string | null;
  selectedVersionId: string | null;
  setHealth: (value: HealthPayload | null) => void;
  setModelVersions: (value: ModelVersionRecord[]) => void;
  setProjects: (value: ProjectRecord[]) => void;
  setProtocolAgents: (value: ProtocolAgentDescriptor[]) => void;
  setRuntimeRecovery: Dispatch<SetStateAction<WorkbenchRuntimeRecoveryState>>;
  setSecurityEventRecords: (value: SecurityEventRecord[]) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedProjectId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  refreshJobHistory: () => Promise<void>;
  refreshResults: () => Promise<void>;
  projectLibraryBackendService?: WorkbenchProjectLibraryBackendService;
  runtimeStatusBackendService?: WorkbenchRuntimeStatusBackendService;
  securityEventBackendService?: WorkbenchSecurityEventBackendService;
  securityEventWindowMs: Record<Exclude<SecurityEventWindow, "">, number>;
};

export function useWorkbenchDataRefreshController({
  directMeshEndpointsText,
  directMeshSelectionMode,
  frontendRuntimeMode,
  securityEventActionFilter,
  securityEventRiskFilter,
  securityEventSourceFilter,
  securityEventStatusFilter,
  securityEventWindowFilter,
  selectedModelId,
  selectedProjectId,
  selectedVersionId,
  setHealth,
  setModelVersions,
  setProjects,
  setProtocolAgents,
  setRuntimeRecovery,
  setSecurityEventRecords,
  setSelectedModelId,
  setSelectedProjectId,
  setSelectedVersionId,
  refreshJobHistory,
  refreshResults,
  projectLibraryBackendService = workbenchProjectLibraryBackendService,
  runtimeStatusBackendService = workbenchRuntimeStatusBackendService,
  securityEventBackendService = workbenchSecurityEventBackendService,
  securityEventWindowMs,
}: UseWorkbenchDataRefreshControllerArgs) {
  const healthRefreshSeqRef = useRef(0);
  const projectRefreshSeqRef = useRef(0);
  const securityEventsRefreshSeqRef = useRef(0);
  const versionsRefreshSeqRef = useRef(0);
  const [projectContext] = useState(() => createWorkbenchProjectContext({
    projectId: selectedProjectId, modelId: selectedModelId, versionId: selectedVersionId,
  }));
  useLayoutEffect(() => {
    projectContext.update({ projectId: selectedProjectId, modelId: selectedModelId, versionId: selectedVersionId });
  }, [projectContext, selectedProjectId, selectedModelId, selectedVersionId]);
  useLayoutEffect(() => {
    projectContext.mount();
    return () => projectContext.dispose();
  }, [projectContext]);

  function clearRecovery(channel: "health" | "projects" | "security_events") {
    setRuntimeRecovery((current) => clearWorkbenchRuntimeRecoveryIssue(current, channel));
  }

  function pushRecovery(
    channel: "health" | "projects" | "security_events",
    error: unknown,
    scopeLabel: string,
  ) {
    const requestError = normalizeWorkbenchRequestError(error, scopeLabel);
    setRuntimeRecovery((current) =>
      upsertWorkbenchRuntimeRecoveryIssue({
        channel,
        current,
        error: requestError,
        scopeLabel,
      }),
    );
  }

  async function refreshHealth() {
    const refreshSeq = ++healthRefreshSeqRef.current;

    try {
      const snapshot = await runtimeStatusBackendService.fetchStatus({
        directMeshEndpointsText,
        directMeshSelectionMode,
        frontendRuntimeMode,
      });

      if (refreshSeq !== healthRefreshSeqRef.current) return;

      setHealth(snapshot.health);
      setProtocolAgents(snapshot.protocolAgents);
      clearRecovery("health");
    } catch (error) {
      if (refreshSeq !== healthRefreshSeqRef.current) return;
      setHealth(null);
      setProtocolAgents([]);
      pushRecovery(
        "health",
        error,
        frontendRuntimeMode === "direct_mesh_gui" ? "Direct mesh runtime" : "Hub health",
      );
    }
  }

  const refreshProjects: WorkbenchProjectRefresh = async (bootstrap = false, preferredProjectId, options) => {
    const refreshSeq = ++projectRefreshSeqRef.current;
    const isCurrent = projectContext.capture();

    try {
      const payload = await projectLibraryBackendService.fetchProjects();
      let nextProjects = payload.projects;

      if (bootstrap && nextProjects.length === 0) {
        const created = await projectLibraryBackendService.createProject({
          name: copyByLanguage.en.defaultProject,
          description: "Local workspace",
        });
        nextProjects = [created.project];
      }

      if (refreshSeq !== projectRefreshSeqRef.current) return;

      setProjects(nextProjects);
      clearRecovery("projects");
      if (options?.preserveSelection || !isCurrent()) return;

      const selection = projectContext.current();
      const requestedProjectId = preferredProjectId === undefined ? selection.projectId : preferredProjectId;
      const nextProjectId =
        requestedProjectId && nextProjects.some((project) => project.project_id === requestedProjectId)
          ? requestedProjectId
          : nextProjects[0]?.project_id ?? null;

      setSelectedProjectId(nextProjectId);

      const nextModelId =
        selection.modelId &&
        (nextProjects.find((project) => project.project_id === nextProjectId)?.models ?? [])
          .some((model) => model.model_id === selection.modelId)
          ? selection.modelId
          : null;

      // A catalog refresh never loads a payload, so it cannot attach an unrelated saved model.
      setSelectedModelId(nextModelId);
      if (!nextModelId || nextModelId !== selection.modelId) {
        setSelectedVersionId(null);
      }
    } catch (error) {
      if (refreshSeq !== projectRefreshSeqRef.current) return;
      // A refresh failure must not discard the last usable project catalog.
      pushRecovery("projects", error, "Project library");
    }
  };

  async function refreshSecurityEvents() {
    const refreshSeq = ++securityEventsRefreshSeqRef.current;

    try {
      const occurredAfter =
        securityEventWindowFilter && securityEventWindowMs[securityEventWindowFilter]
          ? new Date(Date.now() - securityEventWindowMs[securityEventWindowFilter]).toISOString()
          : undefined;
      const payload = await securityEventBackendService.fetchEvents({
        occurred_after: occurredAfter,
        source: securityEventSourceFilter || undefined,
        risk: securityEventRiskFilter || undefined,
        status: securityEventStatusFilter || undefined,
        action: securityEventActionFilter || undefined,
        limit: 120,
      });
      if (refreshSeq !== securityEventsRefreshSeqRef.current) return;
      setSecurityEventRecords(payload.events);
      clearRecovery("security_events");
    } catch (error) {
      if (refreshSeq !== securityEventsRefreshSeqRef.current) return;
      setSecurityEventRecords([]);
      pushRecovery("security_events", error, "Security audit");
    }
  }

  async function refreshVersions(modelId: string) {
    const refreshSeq = ++versionsRefreshSeqRef.current;

    try {
      const payload = await projectLibraryBackendService.fetchModelVersions(modelId);
      if (refreshSeq !== versionsRefreshSeqRef.current || projectContext.current().modelId !== modelId) return;
      setModelVersions(payload.versions);
    } catch {
      if (refreshSeq !== versionsRefreshSeqRef.current || projectContext.current().modelId !== modelId) return;
      setModelVersions([]);
    }
  }

  useEffect(() => {
    void refreshHealth();
    void refreshJobHistory();
    void refreshResults();
    void refreshProjects(true);
    void refreshSecurityEvents();
  }, []);

  useEffect(() => {
    void refreshHealth();
  }, [frontendRuntimeMode, directMeshEndpointsText, directMeshSelectionMode]);

  useEffect(() => {
    void refreshSecurityEvents();
  }, [
    securityEventWindowFilter,
    securityEventSourceFilter,
    securityEventRiskFilter,
    securityEventStatusFilter,
    securityEventActionFilter,
    securityEventBackendService,
  ]);

  useEffect(() => {
    if (!selectedModelId) {
      versionsRefreshSeqRef.current += 1;
      setModelVersions([]);
      setSelectedVersionId(null);
      return;
    }

    void refreshVersions(selectedModelId);
  }, [selectedModelId]);

  return {
    projectContext,
    refreshHealth,
    refreshProjects,
    refreshSecurityEvents,
    refreshVersions,
  };
}
