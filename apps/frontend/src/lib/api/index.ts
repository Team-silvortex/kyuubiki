export type AxialBarJobInput = {
  length: number;
  area: number;
  elements: number;
  tip_force: number;
  youngs_modulus_gpa: number;
  project_id?: string;
  model_version_id?: string;
};

export type TrussNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

export type TrussElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  material_id?: string;
};

export type ModelMaterial = {
  id: string;
  name: string;
  youngs_modulus: number;
  poisson_ratio?: number | null;
};

export type Truss2dJobInput = {
  nodes: TrussNodeInput[];
  elements: TrussElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type Truss3dNodeInput = {
  id: string;
  x: number;
  y: number;
  z: number;
  fix_x: boolean;
  fix_y: boolean;
  fix_z: boolean;
  load_x: number;
  load_y: number;
  load_z: number;
};

export type Truss3dElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  area: number;
  youngs_modulus: number;
  material_id?: string;
};

export type Truss3dJobInput = {
  nodes: Truss3dNodeInput[];
  elements: Truss3dElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type PlaneNodeInput = {
  id: string;
  x: number;
  y: number;
  fix_x: boolean;
  fix_y: boolean;
  load_x: number;
  load_y: number;
};

export type PlaneTriangleElementInput = {
  id: string;
  node_i: number;
  node_j: number;
  node_k: number;
  thickness: number;
  youngs_modulus: number;
  poisson_ratio: number;
  material_id?: string;
};

export type PlaneTriangle2dJobInput = {
  nodes: PlaneNodeInput[];
  elements: PlaneTriangleElementInput[];
  materials?: ModelMaterial[];
  project_id?: string;
  model_version_id?: string;
};

export type JobState = {
  job_id: string;
  status: string;
  worker_id: string | null;
  model_version_id?: string | null;
  message?: string | null;
  progress: number;
  residual?: number | null;
  iteration?: number | null;
  has_result?: boolean;
  project_id?: string;
  simulation_case_id?: string;
  created_at?: string;
  updated_at?: string;
};

export type AxialBarResult = {
  tip_displacement: number;
  reaction_force: number;
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; x: number; displacement: number }>;
  elements: Array<{
    index: number;
    x1: number;
    x2: number;
    strain: number;
    stress: number;
    axial_force: number;
  }>;
  input: {
    length: number;
    area: number;
    elements: number;
    tip_force: number;
    youngs_modulus: number;
  };
};

export type Truss2dResult = {
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    strain: number;
    stress: number;
    axial_force: number;
  }>;
  input: Truss2dJobInput;
};

export type Truss3dResult = {
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; z: number; ux: number; uy: number; uz: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    length: number;
    strain: number;
    stress: number;
    axial_force: number;
  }>;
  input: Truss3dJobInput;
};

export type PlaneTriangle2dResult = {
  max_displacement: number;
  max_stress: number;
  nodes: Array<{ index: number; id: string; x: number; y: number; ux: number; uy: number }>;
  elements: Array<{
    index: number;
    id: string;
    node_i: number;
    node_j: number;
    node_k: number;
    area: number;
    strain_x: number;
    strain_y: number;
    gamma_xy: number;
    stress_x: number;
    stress_y: number;
    tau_xy: number;
    von_mises: number;
  }>;
  input: PlaneTriangle2dJobInput;
};

export type JobEnvelope<TResult = unknown> = {
  job: JobState;
  result?: TResult;
};

export type JobResultRecord<TResult = unknown> = {
  job_id: string;
  status?: string;
  worker_id?: string | null;
  result: TResult;
};

export type JobHistoryPayload = {
  jobs: JobState[];
};

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

const SETTINGS_KEY = "kyuubiki-workbench-settings";

function resolveMaterialLookup(materials: ModelMaterial[] | undefined) {
  return new Map((materials ?? []).map((material) => [material.id, material]));
}

export function resolveTruss2dJobInput(
  input: Truss2dJobInput,
): Omit<Truss2dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);

  return {
    nodes: input.nodes,
    elements: input.elements.map(({ material_id, ...element }) => {
      const material = material_id ? materials.get(material_id) : null;
      return {
        ...element,
        youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
      };
    }),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolveTruss3dJobInput(
  input: Truss3dJobInput,
): Omit<Truss3dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);

  return {
    nodes: input.nodes,
    elements: input.elements.map(({ material_id, ...element }) => {
      const material = material_id ? materials.get(material_id) : null;
      return {
        ...element,
        youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
      };
    }),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

export function resolvePlaneTriangle2dJobInput(
  input: PlaneTriangle2dJobInput,
): Omit<PlaneTriangle2dJobInput, "materials"> {
  const materials = resolveMaterialLookup(input.materials);

  return {
    nodes: input.nodes,
    elements: input.elements.map(({ material_id, ...element }) => {
      const material = material_id ? materials.get(material_id) : null;
      return {
        ...element,
        youngs_modulus: material?.youngs_modulus ?? element.youngs_modulus,
        poisson_ratio:
          material?.poisson_ratio === null || material?.poisson_ratio === undefined
            ? element.poisson_ratio
            : material.poisson_ratio,
      };
    }),
    ...(input.project_id ? { project_id: input.project_id } : {}),
    ...(input.model_version_id ? { model_version_id: input.model_version_id } : {}),
  };
}

function authHeadersFor(url: string) {
  if (typeof window === "undefined") return {};

  try {
    const raw = window.localStorage.getItem(SETTINGS_KEY);
    if (!raw) return {};

    const parsed = JSON.parse(raw) as {
      controlPlaneApiToken?: string;
      clusterApiToken?: string;
      directMeshApiToken?: string;
    };

    if (url.startsWith("/api/direct-mesh")) {
      return parsed.directMeshApiToken ? { "x-kyuubiki-token": parsed.directMeshApiToken } : {};
    }

    if (
      url === "/api/v1/agents/register" ||
      /^\/api\/v1\/agents\/[^/]+\/heartbeat$/.test(url) ||
      /^\/api\/v1\/agents\/[^/]+$/.test(url)
    ) {
      return parsed.clusterApiToken
        ? { "x-kyuubiki-token": parsed.clusterApiToken }
        : parsed.controlPlaneApiToken
          ? { "x-kyuubiki-token": parsed.controlPlaneApiToken }
          : {};
    }

    if (url.startsWith("/api/v1") || url.startsWith("/api/playground") || url === "/api/health") {
      return parsed.controlPlaneApiToken ? { "x-kyuubiki-token": parsed.controlPlaneApiToken } : {};
    }

    return {};
  } catch {
    return {};
  }
}

async function requestJson<T>(url: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  Object.entries(authHeadersFor(url)).forEach(([key, value]) => {
    if (value) headers.set(key, value);
  });

  const response = await fetch(url, {
    ...init,
    headers,
  });
  const payload = (await response.json()) as T & { error?: string };

  if (!response.ok) {
    throw new Error(payload.error ?? "request failed");
  }

  return payload;
}

export function createAxialBarJob(input: AxialBarJobInput): Promise<JobEnvelope<AxialBarResult>> {
  return requestJson<JobEnvelope<AxialBarResult>>("/api/v1/fem/axial-bar/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createTruss2dJob(input: Truss2dJobInput): Promise<JobEnvelope<Truss2dResult>> {
  return requestJson<JobEnvelope<Truss2dResult>>("/api/v1/fem/truss-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createTruss3dJob(input: Truss3dJobInput): Promise<JobEnvelope<Truss3dResult>> {
  return requestJson<JobEnvelope<Truss3dResult>>("/api/v1/fem/truss-3d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function createPlaneTriangle2dJob(
  input: PlaneTriangle2dJobInput,
): Promise<JobEnvelope<PlaneTriangle2dResult>> {
  return requestJson<JobEnvelope<PlaneTriangle2dResult>>("/api/v1/fem/plane-triangle-2d/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function fetchJobStatus<TResult>(jobId: string): Promise<JobEnvelope<TResult>> {
  return requestJson<JobEnvelope<TResult>>(`/api/v1/jobs/${jobId}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchJobHistory(): Promise<JobHistoryPayload> {
  return requestJson<JobHistoryPayload>("/api/v1/jobs", {
    method: "GET",
    cache: "no-store",
  });
}

export function updateJobRecord(
  jobId: string,
  input: Partial<{
    project_id: string;
    model_version_id: string;
    simulation_case_id: string;
    message: string;
  }>,
): Promise<JobEnvelope> {
  return requestJson<JobEnvelope>(`/api/v1/jobs/${jobId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function deleteJobRecord(jobId: string): Promise<{ job: JobState; deleted: boolean }> {
  return requestJson<{ job: JobState; deleted: boolean }>(`/api/v1/jobs/${jobId}`, {
    method: "DELETE",
  });
}

export function cancelJob(jobId: string): Promise<JobEnvelope> {
  return requestJson<JobEnvelope>(`/api/v1/jobs/${jobId}/cancel`, {
    method: "POST",
  });
}

export function fetchHealth(): Promise<HealthPayload> {
  return requestJson<HealthPayload>("/api/health", {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchProtocolAgents(): Promise<ProtocolAgentListPayload> {
  return requestJson<ProtocolAgentListPayload>("/api/v1/protocol/agents", {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchDirectMeshAgents(endpoints: string[]): Promise<DirectMeshAgentListPayload> {
  return requestJson<DirectMeshAgentListPayload>("/api/direct-mesh/agents", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ endpoints }),
    cache: "no-store",
  });
}

export function createDirectMeshSolve<TResult>(
  studyKind: "axial_bar_1d" | "truss_2d" | "truss_3d" | "plane_triangle_2d",
  input: Record<string, unknown>,
  endpoints: string[],
  selectionMode: DirectMeshSelectionMode,
): Promise<JobEnvelope<TResult> & { direct_mesh: { endpoint: string; strategy: DirectMeshSelectionMode; progress_frames: Array<Record<string, unknown>> } }> {
  return requestJson<JobEnvelope<TResult> & { direct_mesh: { endpoint: string; strategy: DirectMeshSelectionMode; progress_frames: Array<Record<string, unknown>> } }>(
    "/api/direct-mesh/solve",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ study_kind: studyKind, input, endpoints, selection_mode: selectionMode }),
    },
  );
}

export function fetchDatabaseExport(): Promise<DatabaseExportPayload> {
  return requestJson<DatabaseExportPayload>("/api/v1/export/database", {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchResults(): Promise<ResultListPayload> {
  return requestJson<ResultListPayload>("/api/v1/results", {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchResultChunk<TItem = Record<string, unknown>>(
  jobId: string,
  kind: ResultChunkKind,
  options: { offset?: number; limit?: number } = {},
): Promise<ResultChunkPayload<TItem>> {
  const params = new URLSearchParams();

  if (typeof options.offset === "number") params.set("offset", String(options.offset));
  if (typeof options.limit === "number") params.set("limit", String(options.limit));

  const suffix = params.size > 0 ? `?${params.toString()}` : "";

  return requestJson<ResultChunkPayload<TItem>>(`/api/v1/results/${jobId}/chunks/${kind}${suffix}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchDirectMeshResultChunk<TItem = Record<string, unknown>>(
  jobId: string,
  kind: ResultChunkKind,
  options: { offset?: number; limit?: number } = {},
): Promise<ResultChunkPayload<TItem>> {
  const params = new URLSearchParams();

  if (typeof options.offset === "number") params.set("offset", String(options.offset));
  if (typeof options.limit === "number") params.set("limit", String(options.limit));

  const suffix = params.size > 0 ? `?${params.toString()}` : "";

  return requestJson<ResultChunkPayload<TItem>>(`/api/direct-mesh/results/${jobId}/chunks/${kind}${suffix}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function updateResultRecord(
  jobId: string,
  result: Record<string, unknown>,
): Promise<{ job_id: string; result: Record<string, unknown> }> {
  return requestJson<{ job_id: string; result: Record<string, unknown> }>(`/api/v1/results/${jobId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ result }),
  });
}

export function deleteResultRecord(
  jobId: string,
): Promise<{ job_id: string; result: Record<string, unknown>; deleted: boolean }> {
  return requestJson<{ job_id: string; result: Record<string, unknown>; deleted: boolean }>(
    `/api/v1/results/${jobId}`,
    { method: "DELETE" },
  );
}

export function fetchProjects(): Promise<ProjectListPayload> {
  return requestJson<ProjectListPayload>("/api/v1/projects", { method: "GET", cache: "no-store" });
}

export function createProject(input: { name: string; description?: string }): Promise<ProjectEnvelope> {
  return requestJson<ProjectEnvelope>("/api/v1/projects", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function updateProject(projectId: string, input: { name?: string; description?: string }): Promise<ProjectEnvelope> {
  return requestJson<ProjectEnvelope>(`/api/v1/projects/${projectId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function deleteProject(projectId: string): Promise<ProjectEnvelope> {
  return requestJson<ProjectEnvelope>(`/api/v1/projects/${projectId}`, { method: "DELETE" });
}

export function fetchModel(modelId: string): Promise<ModelEnvelope> {
  return requestJson<ModelEnvelope>(`/api/v1/models/${modelId}`, { method: "GET", cache: "no-store" });
}

export function createModel(
  projectId: string,
  input: {
    name: string;
    kind: string;
    material?: string;
    model_schema_version?: string;
    payload: Record<string, unknown>;
  },
): Promise<ModelEnvelope> {
  return requestJson<ModelEnvelope>(`/api/v1/projects/${projectId}/models`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function updateModel(
  modelId: string,
  input: Partial<{
    name: string;
    kind: string;
    material: string;
    model_schema_version: string;
    payload: Record<string, unknown>;
  }>,
): Promise<ModelEnvelope> {
  return requestJson<ModelEnvelope>(`/api/v1/models/${modelId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function deleteModel(modelId: string): Promise<ModelEnvelope> {
  return requestJson<ModelEnvelope>(`/api/v1/models/${modelId}`, { method: "DELETE" });
}

export function fetchModelVersions(modelId: string): Promise<ModelVersionListPayload> {
  return requestJson<ModelVersionListPayload>(`/api/v1/models/${modelId}/versions`, {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchModelVersion(versionId: string): Promise<ModelVersionEnvelope> {
  return requestJson<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, {
    method: "GET",
    cache: "no-store",
  });
}

export function createModelVersion(
  modelId: string,
  input: Partial<{
    name: string;
    kind: string;
    material: string;
    model_schema_version: string;
  }> & { payload: Record<string, unknown> },
): Promise<ModelVersionEnvelope> {
  return requestJson<ModelVersionEnvelope>(`/api/v1/models/${modelId}/versions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function updateModelVersion(
  versionId: string,
  input: Partial<{
    name: string;
    kind: string;
    material: string;
    model_schema_version: string;
    payload: Record<string, unknown>;
  }>,
): Promise<ModelVersionEnvelope> {
  return requestJson<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
}

export function deleteModelVersion(versionId: string): Promise<ModelVersionEnvelope> {
  return requestJson<ModelVersionEnvelope>(`/api/v1/model-versions/${versionId}`, { method: "DELETE" });
}
