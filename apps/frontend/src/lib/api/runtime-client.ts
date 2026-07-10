import type {
  DirectMeshAgentListPayload,
  DirectMeshSelectionMode,
  AssetStoreEntryEnvelope,
  AssetStoreEntryKind,
  AssetStorePayload,
  AssetStoreSourceListPayload,
  HealthPayload,
  ProtocolAgentListPayload,
  RegisteredAgentRegistryPayload,
} from "./runtime-types.ts";
import type {
  WorkflowCatalogQuery,
  WorkflowCatalogPayload,
  WorkflowGraphDefinition,
  WorkflowGraphJobResult,
  WorkflowGraphResponseOptions,
  WorkflowOperatorCatalogQuery,
  WorkflowOperatorCatalogPayload,
} from "./workflow-types.ts";
import type { JobEnvelope, JobHistoryPayload, JobState } from "./fem-shared.ts";
import { requestJson } from "./core.ts";

const LARGE_WORKFLOW_COMPACT_THRESHOLD = 1024;
type RuntimeRequestJson = <T>(url: string, init?: RequestInit, timeoutMs?: number) => Promise<T>;
export type DirectMeshStudyKind =
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
  | "frame_2d";

function compactWorkflowResponseOptions(): WorkflowGraphResponseOptions {
  return {
    include_artifact_lineage: false,
    include_artifacts: false,
    include_branch_decisions: false,
    include_dataset_lineage: false,
    include_node_runs: false,
  };
}

function defaultWorkflowGraphResponseOptions(
  graph?: WorkflowGraphDefinition,
): WorkflowGraphResponseOptions | undefined {
  return graph && graph.nodes.length >= LARGE_WORKFLOW_COMPACT_THRESHOLD
    ? compactWorkflowResponseOptions()
    : undefined;
}

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
  return defaultRuntimeApiClient.fetchJobStatus<TResult>(jobId);
}

export function fetchJobHistory(): Promise<JobHistoryPayload> {
  return defaultRuntimeApiClient.fetchJobHistory();
}

export function fetchWorkflowCatalog(
  query?: WorkflowCatalogQuery,
): Promise<WorkflowCatalogPayload> {
  return defaultRuntimeApiClient.fetchWorkflowCatalog(query);
}

export function fetchWorkflowOperators(
  query?: WorkflowOperatorCatalogQuery,
): Promise<WorkflowOperatorCatalogPayload> {
  return defaultRuntimeApiClient.fetchWorkflowOperators(query);
}

export function fetchAssetStore(query?: {
  kind?: AssetStoreEntryKind;
  q?: string;
  source_id?: string;
}): Promise<AssetStorePayload> {
  return defaultRuntimeApiClient.fetchAssetStore(query);
}

export function fetchAssetStoreSources(): Promise<AssetStoreSourceListPayload> {
  return defaultRuntimeApiClient.fetchAssetStoreSources();
}

export function fetchAssetStoreEntry(
  kind: AssetStoreEntryKind,
  entryId: string,
): Promise<AssetStoreEntryEnvelope> {
  return defaultRuntimeApiClient.fetchAssetStoreEntry(kind, entryId);
}

export function submitWorkflowCatalogJob(
  workflowId: string,
  inputArtifacts: Record<string, unknown>,
  responseOptions?: WorkflowGraphResponseOptions,
): Promise<JobEnvelope<WorkflowGraphJobResult>> {
  return defaultRuntimeApiClient.submitWorkflowCatalogJob(workflowId, inputArtifacts, responseOptions);
}

export function submitWorkflowGraphJob(
  graph: WorkflowGraphDefinition,
  inputArtifacts: Record<string, unknown>,
  responseOptions: WorkflowGraphResponseOptions | undefined = defaultWorkflowGraphResponseOptions(graph),
): Promise<JobEnvelope<WorkflowGraphJobResult>> {
  return defaultRuntimeApiClient.submitWorkflowGraphJob(graph, inputArtifacts, responseOptions);
}

export { compactWorkflowResponseOptions };

export function updateJobRecord(
  jobId: string,
  input: Partial<{
    project_id: string;
    model_version_id: string;
    simulation_case_id: string;
    message: string;
  }>,
): Promise<JobEnvelope> {
  return defaultRuntimeApiClient.updateJobRecord(jobId, input);
}

export function deleteJobRecord(jobId: string): Promise<{ job: JobState; deleted: boolean }> {
  return defaultRuntimeApiClient.deleteJobRecord(jobId);
}

export function cancelJob(jobId: string): Promise<JobEnvelope> {
  return defaultRuntimeApiClient.cancelJob(jobId);
}

export function fetchHealth(): Promise<HealthPayload> {
  return defaultRuntimeApiClient.fetchHealth();
}

export function fetchProtocolAgents(): Promise<ProtocolAgentListPayload> {
  return defaultRuntimeApiClient.fetchProtocolAgents();
}

export function fetchRegisteredAgents(): Promise<RegisteredAgentRegistryPayload> {
  return defaultRuntimeApiClient.fetchRegisteredAgents();
}

export function fetchDirectMeshAgents(endpoints: string[]): Promise<DirectMeshAgentListPayload> {
  return defaultRuntimeApiClient.fetchDirectMeshAgents(endpoints);
}

export function createDirectMeshSolve<TResult>(
  studyKind: DirectMeshStudyKind,
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
  return defaultRuntimeApiClient.createDirectMeshSolve<TResult>(studyKind, input, endpoints, selectionMode);
}

export function createRuntimeApiClient(request: RuntimeRequestJson) {
  return {
    fetchJobStatus<TResult>(jobId: string) {
      return request<JobEnvelope<TResult>>(`/api/v1/jobs/${jobId}`, { method: "GET", cache: "no-store" });
    },
    fetchJobHistory() {
      return request<JobHistoryPayload>("/api/v1/jobs", { method: "GET", cache: "no-store" });
    },
    fetchWorkflowCatalog(query?: WorkflowCatalogQuery) {
      return request<WorkflowCatalogPayload>(appendQuery("/api/v1/workflows/catalog", query), {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchWorkflowOperators(query?: WorkflowOperatorCatalogQuery) {
      return request<WorkflowOperatorCatalogPayload>(appendQuery("/api/v1/operators", query), {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchAssetStore(query?: { kind?: AssetStoreEntryKind; q?: string; source_id?: string }) {
      return request<AssetStorePayload>(appendQuery("/api/v1/store", query), {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchAssetStoreSources() {
      return request<AssetStoreSourceListPayload>("/api/v1/store/sources", {
        method: "GET",
        cache: "no-store",
      });
    },
    fetchAssetStoreEntry(kind: AssetStoreEntryKind, entryId: string) {
      return request<AssetStoreEntryEnvelope>(`/api/v1/store/${kind}/${encodeURIComponent(entryId)}`, {
        method: "GET",
        cache: "no-store",
      });
    },
    submitWorkflowCatalogJob(
      workflowId: string,
      inputArtifacts: Record<string, unknown>,
      responseOptions?: WorkflowGraphResponseOptions,
    ) {
      return request<JobEnvelope<WorkflowGraphJobResult>>(`/api/v1/workflows/catalog/${workflowId}/jobs`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          input_artifacts: inputArtifacts,
          ...(responseOptions ? { response_options: responseOptions } : {}),
        }),
      });
    },
    submitWorkflowGraphJob(
      graph: WorkflowGraphDefinition,
      inputArtifacts: Record<string, unknown>,
      responseOptions: WorkflowGraphResponseOptions | undefined = defaultWorkflowGraphResponseOptions(graph),
    ) {
      return request<JobEnvelope<WorkflowGraphJobResult>>("/api/v1/workflows/graph/jobs", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ graph, input_artifacts: inputArtifacts, ...(responseOptions ? { response_options: responseOptions } : {}) }),
      });
    },
    updateJobRecord(
      jobId: string,
      input: Partial<{
        project_id: string;
        model_version_id: string;
        simulation_case_id: string;
        message: string;
      }>,
    ) {
      return request<JobEnvelope>(`/api/v1/jobs/${jobId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(input),
      });
    },
    deleteJobRecord(jobId: string) {
      return request<{ job: JobState; deleted: boolean }>(`/api/v1/jobs/${jobId}`, { method: "DELETE" });
    },
    cancelJob(jobId: string) {
      return request<JobEnvelope>(`/api/v1/jobs/${jobId}/cancel`, { method: "POST" });
    },
    fetchHealth() {
      return request<HealthPayload>("/api/health", { method: "GET", cache: "no-store" });
    },
    fetchProtocolAgents() {
      return request<ProtocolAgentListPayload>("/api/v1/protocol/agents", { method: "GET", cache: "no-store" });
    },
    fetchRegisteredAgents() {
      return request<RegisteredAgentRegistryPayload>("/api/v1/agents", { method: "GET", cache: "no-store" });
    },
    fetchDirectMeshAgents(endpoints: string[]) {
      return request<DirectMeshAgentListPayload>("/api/direct-mesh/agents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ endpoints }),
        cache: "no-store",
      });
    },
    createDirectMeshSolve<TResult>(
      studyKind: DirectMeshStudyKind,
      input: Record<string, unknown>,
      endpoints: string[],
      selectionMode: DirectMeshSelectionMode,
    ) {
      return request<Awaited<ReturnType<typeof createDirectMeshSolve<TResult>>>>("/api/direct-mesh/solve", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ study_kind: studyKind, input, endpoints, selection_mode: selectionMode }),
      });
    },
  };
}

export type RuntimeApiClient = ReturnType<typeof createRuntimeApiClient>;

export const defaultRuntimeApiClient = createRuntimeApiClient(requestJson);
