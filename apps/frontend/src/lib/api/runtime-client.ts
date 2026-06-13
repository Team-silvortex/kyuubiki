import type {
  DirectMeshAgentListPayload,
  DirectMeshSelectionMode,
  HealthPayload,
  JobEnvelope,
  JobHistoryPayload,
  JobState,
  ProtocolAgentListPayload,
  WorkflowCatalogQuery,
  WorkflowCatalogPayload,
  WorkflowGraphDefinition,
  WorkflowGraphJobResult,
  WorkflowOperatorCatalogQuery,
  WorkflowOperatorCatalogPayload,
} from "./index";
import { requestJson } from "./core";

function appendQuery(url: string, query?: Record<string, string | undefined>) {
  if (!query) return url;
  const params = new URLSearchParams();

  for (const [key, value] of Object.entries(query)) {
    if (typeof value === "string" && value.trim().length > 0) {
      params.set(key, value);
    }
  }

  const search = params.toString();
  return search ? `${url}?${search}` : url;
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

export function fetchWorkflowCatalog(
  query?: WorkflowCatalogQuery,
): Promise<WorkflowCatalogPayload> {
  return requestJson<WorkflowCatalogPayload>(appendQuery("/api/v1/workflows/catalog", query), {
    method: "GET",
    cache: "no-store",
  });
}

export function fetchWorkflowOperators(
  query?: WorkflowOperatorCatalogQuery,
): Promise<WorkflowOperatorCatalogPayload> {
  return requestJson<WorkflowOperatorCatalogPayload>(appendQuery("/api/v1/operators", query), {
    method: "GET",
    cache: "no-store",
  });
}

export function submitWorkflowCatalogJob(
  workflowId: string,
  inputArtifacts: Record<string, unknown>,
): Promise<JobEnvelope<WorkflowGraphJobResult>> {
  return requestJson<JobEnvelope<WorkflowGraphJobResult>>(`/api/v1/workflows/catalog/${workflowId}/jobs`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ input_artifacts: inputArtifacts }),
  });
}

export function submitWorkflowGraphJob(
  graph: WorkflowGraphDefinition,
  inputArtifacts: Record<string, unknown>,
): Promise<JobEnvelope<WorkflowGraphJobResult>> {
  return requestJson<JobEnvelope<WorkflowGraphJobResult>>("/api/v1/workflows/graph/jobs", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ graph, input_artifacts: inputArtifacts }),
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
  studyKind:
    | "axial_bar_1d"
    | "thermal_bar_1d"
    | "heat_bar_1d"
    | "electrostatic_plane_triangle_2d"
    | "electrostatic_plane_quad_2d"
    | "heat_plane_triangle_2d"
    | "heat_plane_quad_2d"
    | "thermal_truss_2d"
    | "thermal_truss_3d"
    | "spring_1d"
    | "spring_2d"
    | "spring_3d"
    | "beam_1d"
    | "thermal_beam_1d"
    | "thermal_frame_2d"
    | "torsion_1d"
    | "truss_2d"
    | "truss_3d"
    | "plane_triangle_2d"
    | "thermal_plane_triangle_2d"
    | "plane_quad_2d"
    | "thermal_plane_quad_2d"
    | "frame_2d",
  input: Record<string, unknown>,
  endpoints: string[],
  selectionMode: DirectMeshSelectionMode,
): Promise<
  JobEnvelope<TResult> & {
    direct_mesh: {
      endpoint: string;
      strategy: DirectMeshSelectionMode;
      progress_frames: Array<Record<string, unknown>>;
    };
  }
> {
  return requestJson<
    JobEnvelope<TResult> & {
      direct_mesh: {
        endpoint: string;
        strategy: DirectMeshSelectionMode;
        progress_frames: Array<Record<string, unknown>>;
      };
    }
  >("/api/direct-mesh/solve", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ study_kind: studyKind, input, endpoints, selection_mode: selectionMode }),
  });
}
