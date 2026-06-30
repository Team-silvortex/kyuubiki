import { describeDirectMeshAgents, solveViaDirectMesh } from "@/lib/direct-mesh/rpc";
import { putDirectMeshResult } from "@/lib/direct-mesh/results";
import { resolveAuthorizedDirectMeshEndpoints } from "@/lib/direct-mesh/security";
import {
  directMeshMethodForStudyKind,
  normalizeDirectMeshStudyInput,
  type DirectMeshSolveBody,
} from "@/lib/runtime-gateway/direct-mesh-study-contract";

export async function inspectDirectMeshRuntime(input: { endpoints?: string[] }) {
  const endpoints = resolveAuthorizedDirectMeshEndpoints(input.endpoints);
  const { agents } = await describeDirectMeshAgents(endpoints);

  return {
    mode: "direct_mesh_gui",
    gateway_contract: "kyuubiki.frontend-runtime-gateway/direct-mesh-v1",
    discovery: "manual",
    endpoint_count: endpoints.length,
    agents,
  };
}

export async function runDirectMeshStudy(body: DirectMeshSolveBody) {
  const startedAt = new Date().toISOString();
  const method = directMeshMethodForStudyKind(body.study_kind);
  const normalizedInput = normalizeDirectMeshStudyInput(body.study_kind, body.input);
  const endpoints = resolveAuthorizedDirectMeshEndpoints(body.endpoints);
  const solved = await solveViaDirectMesh(
    method,
    normalizedInput,
    endpoints,
    body.selection_mode ?? "healthiest",
  );
  const completedAt = new Date().toISOString();
  const jobId = `direct-${Date.now().toString(36)}`;

  putDirectMeshResult(jobId, {
    studyKind: body.study_kind,
    result: (solved.result ?? {}) as Record<string, unknown>,
    endpoint: solved.endpoint,
    storedAt: completedAt,
  });

  return {
    job: {
      job_id: jobId,
      status: "completed",
      worker_id: `direct-mesh@${solved.endpoint}`,
      progress: 1,
      message: "completed through direct mesh runtime gateway",
      created_at: startedAt,
      updated_at: completedAt,
      has_result: true,
    },
    result: solved.result,
    direct_mesh: {
      endpoint: solved.endpoint,
      strategy: solved.strategy,
      progress_frames: solved.progress_frames,
    },
    gateway_contract: "kyuubiki.frontend-runtime-gateway/direct-mesh-v1",
  };
}
