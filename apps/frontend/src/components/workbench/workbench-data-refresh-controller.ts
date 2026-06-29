import { useEffect, useRef, type Dispatch, type SetStateAction } from "react";
import type {
  DirectMeshSelectionMode,
  FrontendRuntimeMode,
  HealthPayload,
  ModelVersionRecord,
  ProjectRecord,
  ProtocolAgentDescriptor,
  SecurityEventRecord,
} from "@/lib/api";
import {
  createProject,
  fetchModelVersions,
  fetchProjects,
  fetchSecurityEvents,
} from "@/lib/api";
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
  runtimeStatusBackendService?: WorkbenchRuntimeStatusBackendService;
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
  runtimeStatusBackendService = workbenchRuntimeStatusBackendService,
  securityEventWindowMs,
}: UseWorkbenchDataRefreshControllerArgs) {
  const healthRefreshSeqRef = useRef(0);
  const projectRefreshSeqRef = useRef(0);
  const securityEventsRefreshSeqRef = useRef(0);
  const versionsRefreshSeqRef = useRef(0);

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

  async function refreshProjects(bootstrap = false) {
    const refreshSeq = ++projectRefreshSeqRef.current;

    try {
      const payload = await fetchProjects();
      let nextProjects = payload.projects;

      if (bootstrap && nextProjects.length === 0) {
        const created = await createProject({
          name: copyByLanguage.en.defaultProject,
          description: "Local workspace",
        });
        nextProjects = [created.project];
      }

      if (refreshSeq !== projectRefreshSeqRef.current) return;

      setProjects(nextProjects);

      const nextProjectId =
        selectedProjectId && nextProjects.some((project) => project.project_id === selectedProjectId)
          ? selectedProjectId
          : nextProjects[0]?.project_id ?? null;

      setSelectedProjectId(nextProjectId);

      const nextModelId =
        selectedModelId &&
        nextProjects.some((project) =>
          (project.models ?? []).some((model) => model.model_id === selectedModelId),
        )
          ? selectedModelId
          : (nextProjects.find((project) => project.project_id === nextProjectId)?.models ?? [])[0]
              ?.model_id ?? null;

      setSelectedModelId(nextModelId);
      if (!nextModelId) {
        setSelectedVersionId(null);
      }
      clearRecovery("projects");
    } catch (error) {
      if (refreshSeq !== projectRefreshSeqRef.current) return;
      setProjects([]);
      pushRecovery("projects", error, "Project library");
    }
  }

  async function refreshSecurityEvents() {
    const refreshSeq = ++securityEventsRefreshSeqRef.current;

    try {
      const occurredAfter =
        securityEventWindowFilter && securityEventWindowMs[securityEventWindowFilter]
          ? new Date(Date.now() - securityEventWindowMs[securityEventWindowFilter]).toISOString()
          : undefined;
      const payload = await fetchSecurityEvents({
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
      const payload = await fetchModelVersions(modelId);
      if (refreshSeq !== versionsRefreshSeqRef.current) return;
      setModelVersions(payload.versions);
    } catch {
      if (refreshSeq !== versionsRefreshSeqRef.current) return;
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
  ]);

  useEffect(() => {
    if (!selectedModelId) {
      setModelVersions([]);
      return;
    }

    void refreshVersions(selectedModelId);
  }, [selectedModelId]);

  return {
    refreshHealth,
    refreshProjects,
    refreshSecurityEvents,
    refreshVersions,
  };
}
