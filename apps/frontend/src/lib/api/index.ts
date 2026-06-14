export * from "./workflow-types";
export * from "./runtime-client";
export * from "./security-results-client";
export * from "./headless-results-client";
export * from "./headless-handoff-client";
export * from "./project-client";
export * from "./fem-shared";
export * from "./fem-1d";
export * from "./fem-2d-line";
export * from "./fem-2d-surface";
export * from "./fem-3d";

import type { JobState } from "./fem-shared";

export type ModelVersionRecord = {
  version_id: string;
  project_id: string;
  model_id: string;
  name: string;
  version_number: number;
  kind: string;
  material?: string | null;
  model_schema_version: string;
  payload: Record<string, unknown>;
  inserted_at: string;
  updated_at: string;
};

export type ModelRecord = {
  model_id: string;
  project_id: string;
  name: string;
  kind: string;
  material?: string | null;
  model_schema_version: string;
  payload: Record<string, unknown>;
  latest_version_id?: string | null;
  latest_version_number?: number | null;
  inserted_at: string;
  updated_at: string;
  versions?: ModelVersionRecord[];
};

export type ProjectRecord = {
  project_id: string;
  name: string;
  description?: string | null;
  inserted_at: string;
  updated_at: string;
  models?: ModelRecord[];
};

export type ProjectListPayload = {
  projects: ProjectRecord[];
};

export type ProjectEnvelope = {
  project: ProjectRecord;
};

export type ModelEnvelope = {
  model: ModelRecord;
};

export type ModelListPayload = {
  models: ModelRecord[];
};

export type ModelVersionEnvelope = {
  version: ModelVersionRecord;
};

export type ModelVersionListPayload = {
  versions: ModelVersionRecord[];
};

export type HealthPayload = {
  service: string;
  status: string;
  protocol?: {
    program: string;
    role: string;
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

export type ProtocolAgentDescriptor = {
  id: string;
  host: string;
  port: number;
  tags?: string[];
  capacity?: number | null;
  region?: string | null;
  zone?: string | null;
  role?: string | null;
  descriptor?: {
    program: string;
    role: string;
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
  };
  descriptor_error?: string;
};

export type ProtocolAgentListPayload = {
  agents: ProtocolAgentDescriptor[];
};

export type DatabaseExportPayload = {
  exported_at: string;
  projects: ProjectRecord[];
  models: ModelRecord[];
  model_versions: ModelVersionRecord[];
  jobs: JobState[];
  results: Array<{
    job_id: string;
    result: Record<string, unknown>;
    inserted_at?: string;
    updated_at?: string;
  }>;
  security_events?: SecurityEventRecord[];
};

export type SecurityEventRecord = {
  event_id: string;
  event_type: string;
  source: string;
  action: string;
  risk: string;
  status: string;
  note?: string | null;
  context: Record<string, unknown>;
  occurred_at: string;
  inserted_at?: string;
  updated_at?: string;
};

export type SecurityEventListPayload = {
  events: SecurityEventRecord[];
};

export type SecurityEventEnvelope = {
  event: SecurityEventRecord;
};

export type SecurityEventExportPayload = {
  exported_at: string;
  schema: {
    name: string;
    version: number;
    fields: Array<{ name: string; type: string }>;
  };
  filters: Record<string, string>;
  summary: {
    total: number;
    by_source: Record<string, number>;
    by_risk: Record<string, number>;
    by_status: Record<string, number>;
  };
  events: SecurityEventRecord[];
};

export type ResultRecord = {
  job_id: string;
  result: Record<string, unknown>;
  inserted_at?: string;
  updated_at?: string;
};

export type ResultListPayload = {
  results: ResultRecord[];
};

export type ResultChunkKind = "nodes" | "elements";

export type ResultChunkPayload<TItem = Record<string, unknown>> = {
  job_id: string;
  kind: ResultChunkKind;
  offset: number;
  limit: number;
  returned: number;
  total: number;
  items: TItem[];
};

export type FrontendRuntimeMode = "orchestrated_gui" | "direct_mesh_gui";
export type DirectMeshSelectionMode = "first_reachable" | "healthiest";

export type DirectMeshAgentListPayload = {
  mode: FrontendRuntimeMode;
  discovery: string;
  endpoint_count: number;
  agents: ProtocolAgentDescriptor[];
};
