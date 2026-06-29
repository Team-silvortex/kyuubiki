import type { JobState } from "@/lib/api/fem-shared";

export type HealthPayload = {
  service: string;
  status: string;
  protocol?: {
    program: string;
    role: string;
    authority?: {
      control_mode: string;
      authority_mode: string;
      orchestrator_id?: string | null;
      orchestrator_session_id?: string | null;
      accepts_multi_orchestrator_binding: boolean;
      agent_library_replication: string;
    };
    protocol?: {
      name: string;
      version: number;
      transport?: {
        kind: string;
        encoding: string;
      };
      resources?: Record<string, string>;
    };
    compatible_solver_rpc?: {
      name: string;
      rpc_version: number;
      transport?: {
        kind: string;
        framing?: string;
        encoding: string;
      };
      methods?: string[];
    };
  };
  deployment?: {
    mode: string;
    discovery: string;
    manifest_path?: string | null;
    endpoint_count: number;
  };
  remote_solver_registry?: {
    active_agents: number;
    control_modes?: Record<string, number>;
    session_states?: Record<string, number>;
    mesh_topology?: {
      managed_orchestrators?: Array<{
        orch_id?: string | null;
        agent_count: number;
        agent_ids: string[];
        session_ids: string[];
      }>;
      offline_mesh?: {
        agent_count: number;
        agent_ids: string[];
        clustered_meshes?: Array<{
          cluster_id: string;
          agent_count: number;
          agent_ids: string[];
          relay_candidate_ids: string[];
        }>;
        unclustered_agent_ids?: string[];
      };
    };
  };
  security?: {
    api_token_configured: boolean;
    cluster_token_configured: boolean;
    cluster_agent_allowlist_enabled: boolean;
    cluster_agent_allowlist_count: number;
    cluster_cluster_allowlist_enabled: boolean;
    cluster_cluster_allowlist_count: number;
    cluster_fingerprint_required: boolean;
    cluster_timestamp_window_ms: number;
    protect_reads: boolean;
    mutating_routes_protected: boolean;
    cluster_routes_protected: boolean;
  };
  watchdog?: {
    scan_interval_ms: number;
    stale_job_ms: number;
    job_timeout_ms: number;
    active_jobs: number;
    stalled_jobs: number;
    timed_out_jobs: number;
    watchdog_state?: "unknown" | "healthy" | "watch" | "critical";
    orchestra_load?: {
      state: "healthy" | "watch" | "critical";
      active_jobs: number;
      queued_jobs: number;
      stale_active_jobs: number;
      timed_out_active_jobs: number;
      active_by_status: Record<string, number>;
      thresholds: {
        warn_active_jobs: number;
        critical_active_jobs: number;
      };
    };
    agent_load?: {
      state: "unknown" | "healthy" | "watch" | "critical";
      available: boolean;
      reason?: string;
      total_agents: number;
      active_agents: number;
      stale_agents: number;
      stale_after_ms?: number;
      capacity_slots: number;
      leased_slots: number;
      utilization: number;
      stale_execution_lease_count?: number;
      control_modes?: Record<string, number>;
      thresholds?: {
        warn_utilization: number;
        critical_utilization: number;
      };
    };
    operator_load?: {
      state: "healthy" | "watch" | "critical";
      active_jobs: number;
      active_operator_count: number;
      active_by_operator: Record<string, number>;
      hotspots: Array<{
        operator: string;
        active_jobs: number;
        state: "healthy" | "watch" | "critical";
      }>;
      thresholds: {
        warn_active_jobs: number;
        critical_active_jobs: number;
      };
    };
  };
  transport?: {
    http: number;
    solver_agent_tcp: number;
  };
  solver_agents?: Array<{
    id: string;
    host: string;
    port: number;
  }>;
};

export type AssetStoreEntryKind = "operator" | "workflow_template" | "frontend_dsl_template";
export type AssetStoreSourceType = "builtin" | "catalog_file" | "remote_http" | string;

export type AssetStoreSource = {
  id: string;
  type: AssetStoreSourceType;
  label: string;
  enabled: boolean;
  editable: boolean;
  status: "ready" | "configured" | "unavailable" | string;
  url?: string | null;
  path?: string | null;
  supports: AssetStoreEntryKind[];
};

export type AssetStoreEntry = {
  id: string;
  kind: AssetStoreEntryKind;
  title: string;
  summary?: string | null;
  version?: string | null;
  source_id: string;
  source_kind: AssetStoreSourceType;
  package_ref?: string | null;
  domain?: string | null;
  category?: string | null;
  tags: string[];
  install: {
    mode: string;
    requires_download: boolean;
    target?: string | null;
  };
  payload?: Record<string, unknown>;
};

export type AssetStorePayload = {
  entries: AssetStoreEntry[];
  sources: AssetStoreSource[];
  summary: {
    entry_count: number;
    kinds: Partial<Record<AssetStoreEntryKind, number>>;
    sources: Record<string, number>;
  };
};

export type AssetStoreSourceListPayload = {
  sources: AssetStoreSource[];
};

export type AssetStoreEntryEnvelope = {
  entry: AssetStoreEntry;
};

export type ProtocolAgentDescriptor = {
  id: string;
  host: string;
  port: number;
  control_mode?: string | null;
  orch_id?: string | null;
  orch_session_id?: string | null;
  cluster_id?: string | null;
  tags?: string[];
  capacity?: number | null;
  region?: string | null;
  zone?: string | null;
  role?: string | null;
  execution_state?: "idle" | "leased" | "lease_stale";
  active_lease?: {
    lease_id: string;
    agent_id: string;
    control_mode?: string | null;
    orch_id?: string | null;
    orch_session_id?: string | null;
    job_id?: string | null;
    method?: string | null;
    claimed_at?: string | null;
    age_ms?: number | null;
    is_stale?: boolean | null;
  } | null;
  watchdog?: AgentWatchdogSnapshot | null;
  mesh?: {
    cluster_id?: string | null;
    peer_group_id?: string | null;
    peer_count?: number;
    topology_role?: string | null;
    relay_candidate?: boolean;
    peers?: Array<{
      id: string;
      address: string;
      cluster_id?: string | null;
      health_score?: number | null;
      status?: string;
      topology_role?: string | null;
    }>;
  } | null;
  descriptor?: {
    program: string;
    role: string;
    authority?: {
      control_mode: string;
      authority_mode: string;
      orchestrator_id?: string | null;
      orchestrator_session_id?: string | null;
      accepts_multi_orchestrator_binding: boolean;
      agent_library_replication: string;
    };
    runtime: {
      cluster_id?: string | null;
      runtime_mode: string;
      headless: boolean;
      cluster_size?: number;
      health_score?: number;
      peers: Array<{
        address: string;
        status?: string;
        failure_count?: number;
        last_seen_unix_s?: number | null;
      }>;
    };
    protocol: {
      name: string;
      rpc_version: number;
      transport: {
        kind: string;
        framing?: string;
        encoding: string;
      };
      methods: string[];
    };
    capabilities: Array<{
      id: string;
      role: string;
      methods: string[];
      tags: string[];
    }>;
    deployment_modes: string[];
    watchdog?: AgentWatchdogSnapshot | null;
  };
  descriptor_error?: string;
};

export type AgentWatchdogSnapshot = {
  state: "unknown" | "healthy" | "watch" | "critical";
  active_execution_count: number;
  recent_failure_count: number;
  active_executions?: Array<{
    request_id: string;
    job_id?: string | null;
    method: string;
    elapsed_ms: number;
    idle_ms: number;
  }>;
  recent_failures?: Array<{
    request_id?: string;
    job_id?: string | null;
    method?: string;
    reason_code: string;
    message: string;
    elapsed_ms?: number;
    occurred_unix_ms?: number;
  }>;
};

export type ProtocolAgentListPayload = {
  agents: ProtocolAgentDescriptor[];
};

export type RegisteredAgentSnapshot = {
  id: string;
  control_mode?: string | null;
  orch_id?: string | null;
  orch_session_id?: string | null;
  cluster_id?: string | null;
  execution_state?: "idle" | "leased" | "lease_stale";
  active_lease?: ProtocolAgentDescriptor["active_lease"];
  mesh?: ProtocolAgentDescriptor["mesh"];
};

export type RegisteredAgentRegistryPayload = {
  agents: RegisteredAgentSnapshot[];
  summary: {
    active_execution_lease_count: number;
    stale_execution_lease_count: number;
    active_execution_leases?: Array<ProtocolAgentDescriptor["active_lease"]>;
  };
};

export type FrontendRuntimeMode = "orchestrated_gui" | "direct_mesh_gui";
export type DirectMeshSelectionMode = "first_reachable" | "healthiest";

export type DirectMeshAgentListPayload = {
  mode: FrontendRuntimeMode;
  discovery: string;
  endpoint_count: number;
  agents: ProtocolAgentDescriptor[];
};

export type DirectMeshSolveEnvelope<TResult = unknown> = {
  job: JobState;
  result?: TResult;
  direct_mesh: {
    endpoint: string;
    strategy: DirectMeshSelectionMode;
    progress_frames: Array<Record<string, unknown>>;
  };
};
