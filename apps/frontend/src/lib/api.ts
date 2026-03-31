export type AxialBarJobInput = {
  length: number;
  area: number;
  elements: number;
  tip_force: number;
  youngs_modulus_gpa: number;
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
};

export type Truss2dJobInput = {
  nodes: TrussNodeInput[];
  elements: TrussElementInput[];
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
};

export type Truss3dJobInput = {
  nodes: Truss3dNodeInput[];
  elements: Truss3dElementInput[];
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
};

export type PlaneTriangle2dJobInput = {
  nodes: PlaneNodeInput[];
  elements: PlaneTriangleElementInput[];
};

export type JobState = {
  job_id: string;
  status: string;
  worker_id: string | null;
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
  transport?: {
    http: number;
    solver_agent_tcp: number;
  };
};

async function requestJson<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init);
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

export function fetchHealth(): Promise<HealthPayload> {
  return requestJson<HealthPayload>("/api/health", {
    method: "GET",
    cache: "no-store",
  });
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
