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

export type JobHistoryPayload = {
  jobs: JobState[];
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
