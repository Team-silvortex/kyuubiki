import { useEffect, useRef } from "react";
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
  fetchDirectMeshAgents,
  fetchHealth,
  fetchModelVersions,
  fetchProjects,
  fetchProtocolAgents,
  fetchRegisteredAgents,
  fetchSecurityEvents,
} from "@/lib/api";
import { copyByLanguage } from "@/components/workbench/workbench-copy";
import { parseDirectMeshEndpoints } from "@/lib/workbench/helpers";
import { validateWorkbenchExecutionGovernance } from "@/lib/workbench/governance";
import type { SecurityEventWindow } from "@/components/workbench/workbench-types";
import type { WorkbenchSecurityAuditRisk, WorkbenchSecurityAuditSource } from "@/lib/workbench/security-audit";

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
  setSecurityEventRecords: (value: SecurityEventRecord[]) => void;
  setSelectedModelId: (value: string | null) => void;
  setSelectedProjectId: (value: string | null) => void;
  setSelectedVersionId: (value: string | null) => void;
  refreshJobHistory: () => Promise<void>;
  refreshResults: () => Promise<void>;
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
  setSecurityEventRecords,
  setSelectedModelId,
  setSelectedProjectId,
  setSelectedVersionId,
  refreshJobHistory,
  refreshResults,
  securityEventWindowMs,
}: UseWorkbenchDataRefreshControllerArgs) {
  const healthRefreshSeqRef = useRef(0);
  const projectRefreshSeqRef = useRef(0);
  const securityEventsRefreshSeqRef = useRef(0);
  const versionsRefreshSeqRef = useRef(0);

  async function refreshHealth() {
    const refreshSeq = ++healthRefreshSeqRef.current;

    if (frontendRuntimeMode === "direct_mesh_gui") {
      try {
        const governance = validateWorkbenchExecutionGovernance({ frontendRuntimeMode, directMeshEndpointsText });
        if (!governance.ok) {
          if (refreshSeq !== healthRefreshSeqRef.current) return;
          setHealth(null);
          setProtocolAgents([]);
          return;
        }
        const endpoints = parseDirectMeshEndpoints(governance.directMeshEndpointsText);
        const nextDirect = await fetchDirectMeshAgents(endpoints);
        const directMethods = [
          ...new Set(nextDirect.agents.flatMap((agent) => agent.descriptor?.protocol?.methods ?? [])),
        ];

        if (refreshSeq !== healthRefreshSeqRef.current) return;

        setProtocolAgents(nextDirect.agents);
        setHealth({
          service: "kyuubiki-frontend-direct-mesh",
          status: nextDirect.agents.length > 0 ? "ok" : "degraded",
          protocol: {
            program: "kyuubiki-frontend",
            role: "gui",
            protocol: {
              name: "kyuubiki.direct-mesh/http-v1",
              version: 1,
              transport: { kind: "http", encoding: "json" },
            },
            compatible_solver_rpc: {
              name: "kyuubiki.solver-rpc/v1",
              rpc_version: 1,
              transport: {
                kind: "tcp",
                framing: "length_prefixed_u32",
                encoding: "json",
              },
              methods: directMethods,
            },
          },
          deployment: {
            mode: "direct_mesh",
            discovery: nextDirect.discovery,
            endpoint_count: nextDirect.endpoint_count,
          },
          remote_solver_registry: {
            active_agents: nextDirect.agents.length,
          },
        });
      } catch {
        if (refreshSeq !== healthRefreshSeqRef.current) return;
        setHealth(null);
        setProtocolAgents([]);
      }
      return;
    }

    try {
      const [nextHealth, nextProtocolAgents, nextRegisteredAgents] = await Promise.all([
        fetchHealth(),
        fetchProtocolAgents().catch(() => ({ agents: [] })),
        fetchRegisteredAgents().catch(() => ({
          agents: [],
          summary: { active_execution_lease_count: 0, stale_execution_lease_count: 0 },
        })),
      ]);

      if (refreshSeq !== healthRefreshSeqRef.current) return;

      setHealth(nextHealth);
      const registryById = new Map(
        nextRegisteredAgents.agents.map((agent) => [agent.id, agent] as const),
      );

      setProtocolAgents(
        nextProtocolAgents.agents.map((agent) => {
          const registered = registryById.get(agent.id);

          return registered
            ? {
                ...agent,
                execution_state: registered.execution_state,
                active_lease: registered.active_lease,
              }
            : agent;
        }),
      );
    } catch {
      if (refreshSeq !== healthRefreshSeqRef.current) return;
      setHealth(null);
      setProtocolAgents([]);
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
    } catch {
      if (refreshSeq !== projectRefreshSeqRef.current) return;
      setProjects([]);
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
    } catch {
      if (refreshSeq !== securityEventsRefreshSeqRef.current) return;
      setSecurityEventRecords([]);
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
