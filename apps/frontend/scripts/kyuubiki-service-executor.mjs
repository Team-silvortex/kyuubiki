import { findAutomationActionContract } from "./kyuubiki-automation-actions.mjs";
import { resolveStudyKindAndInput } from "./kyuubiki-study-resolver.mjs";

const DEFAULT_API_BASE_URL = "http://127.0.0.1:3000";
const TERMINAL_JOB_STATUSES = new Set(["completed", "failed", "cancelled"]);

function compactWorkflowResponseOptions() {
  return {
    include_artifact_lineage: false,
    include_artifacts: false,
    include_branch_decisions: false,
    include_dataset_lineage: false,
    include_node_runs: false,
  };
}

function pickFirstString(payload, keys) {
  for (const key of keys) {
    if (typeof payload?.[key] === "string" && payload[key].trim()) return payload[key].trim();
  }
  return null;
}

function pickNumber(payload, keys, fallback) {
  for (const key of keys) {
    if (typeof payload?.[key] === "number" && Number.isFinite(payload[key])) return payload[key];
    if (typeof payload?.[key] === "string" && payload[key].trim()) {
      const value = Number(payload[key]);
      if (Number.isFinite(value)) return value;
    }
  }
  return fallback;
}

function resolveApiUrl(baseUrl, pathname) {
  return new URL(pathname, baseUrl.endsWith("/") ? baseUrl : `${baseUrl}/`).toString();
}

async function requestJson(baseUrl, pathname, init = {}) {
  const response = await fetch(resolveApiUrl(baseUrl, pathname), {
    ...init,
    headers: {
      Accept: "application/json",
      ...(init.body ? { "Content-Type": "application/json" } : {}),
      ...(init.headers ?? {}),
    },
  });
  const text = await response.text();
  const payload = text ? JSON.parse(text) : null;
  if (!response.ok) {
    throw new Error(`Service request failed ${response.status} ${response.statusText}: ${pathname}`);
  }
  return payload;
}

function normalizeActionName(action) {
  return String(action ?? "")
    .trim()
    .toLowerCase()
    .replaceAll(/[.\s-]+/g, "_");
}

async function executeServiceHealth(step, runtime) {
  const payload = step.payload ?? {};
  const requestPath = pickFirstString(payload, ["path"]) ?? "/api/health";
  const result = await requestJson(runtime.baseUrl, requestPath, { method: "GET" });
  return { message: `Fetched service health from ${requestPath}`, result };
}

async function executeProjectCreate(step, runtime) {
  const payload = step.payload ?? {};
  const result = await requestJson(runtime.baseUrl, "/api/v1/projects", {
    method: "POST",
    body: JSON.stringify({
      name: payload.name,
      ...(typeof payload.description === "string" ? { description: payload.description } : {}),
    }),
  });
  return { message: `Created project ${result?.project?.project_id ?? payload.name}`, result };
}

async function executeProjectUpdate(step, runtime) {
  const payload = step.payload ?? {};
  const projectId = pickFirstString(payload, ["project_id", "projectId"]);
  const result = await requestJson(runtime.baseUrl, `/api/v1/projects/${projectId}`, {
    method: "PATCH",
    body: JSON.stringify({
      ...(typeof payload.name === "string" ? { name: payload.name } : {}),
      ...(typeof payload.description === "string" ? { description: payload.description } : {}),
    }),
  });
  return { message: `Updated project ${projectId}`, result };
}

async function executeProjectDelete(step, runtime) {
  const payload = step.payload ?? {};
  const projectId = pickFirstString(payload, ["project_id", "projectId"]);
  const result = await requestJson(runtime.baseUrl, `/api/v1/projects/${projectId}`, { method: "DELETE" });
  return { message: `Deleted project ${projectId}`, result };
}

async function executeModelCreate(step, runtime) {
  const payload = step.payload ?? {};
  const projectId = pickFirstString(payload, ["project_id", "projectId"]);
  const result = await requestJson(runtime.baseUrl, `/api/v1/projects/${projectId}/models`, {
    method: "POST",
    body: JSON.stringify({
      name: payload.name,
      kind: payload.kind,
      payload: payload.payload,
      ...(typeof payload.material === "string" ? { material: payload.material } : {}),
      ...(typeof payload.model_schema_version === "string"
        ? { model_schema_version: payload.model_schema_version }
        : {}),
    }),
  });
  return { message: `Created model under project ${projectId}`, result };
}

async function executeModelVersionCreate(step, runtime) {
  const payload = step.payload ?? {};
  const modelId = pickFirstString(payload, ["model_id", "modelId"]);
  const result = await requestJson(runtime.baseUrl, `/api/v1/models/${modelId}/versions`, {
    method: "POST",
    body: JSON.stringify({
      payload: payload.payload,
      ...(typeof payload.name === "string" ? { name: payload.name } : {}),
      ...(typeof payload.kind === "string" ? { kind: payload.kind } : {}),
      ...(typeof payload.material === "string" ? { material: payload.material } : {}),
      ...(typeof payload.model_schema_version === "string"
        ? { model_schema_version: payload.model_schema_version }
        : {}),
    }),
  });
  return { message: `Created model version under model ${modelId}`, result };
}

async function executeWorkflowSubmitCatalog(step, runtime) {
  const payload = step.payload ?? {};
  const workflowId = pickFirstString(payload, ["workflow_id", "workflowId"]);
  const result = await requestJson(runtime.baseUrl, `/api/v1/workflows/catalog/${workflowId}/jobs`, {
    method: "POST",
    body: JSON.stringify({
      input_artifacts: payload.input_artifacts && typeof payload.input_artifacts === "object" ? payload.input_artifacts : {},
      response_options: compactWorkflowResponseOptions(),
    }),
  });
  return { message: `Submitted catalog workflow ${workflowId}`, result };
}

async function executeWorkflowSubmitGraph(step, runtime) {
  const payload = step.payload ?? {};
  const result = await requestJson(runtime.baseUrl, "/api/v1/workflows/graph/jobs", {
    method: "POST",
    body: JSON.stringify({
      graph: payload.graph,
      input_artifacts: payload.input_artifacts && typeof payload.input_artifacts === "object" ? payload.input_artifacts : {},
      response_options: compactWorkflowResponseOptions(),
    }),
  });
  return { message: "Submitted workflow graph job", result };
}

async function executeJobWait(step, runtime) {
  const payload = step.payload ?? {};
  const jobId = pickFirstString(payload, ["job_id", "jobId"]);
  const intervalMs = pickNumber(payload, ["interval_ms", "intervalMs"], 1000);
  const timeoutMs = pickNumber(payload, ["timeout_ms", "timeoutMs"], 60000);
  const deadline = Date.now() + timeoutMs;

  while (Date.now() <= deadline) {
    const result = await requestJson(runtime.baseUrl, `/api/v1/jobs/${jobId}`, { method: "GET" });
    if (TERMINAL_JOB_STATUSES.has(result?.job?.status)) {
      return { message: `Job ${jobId} reached ${result.job.status}`, result };
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }

  throw new Error(`Timed out waiting for job ${jobId}`);
}

async function executeJobFetch(step, runtime) {
  const payload = step.payload ?? {};
  const jobId = pickFirstString(payload, ["job_id", "jobId"]);
  const result = await requestJson(runtime.baseUrl, `/api/v1/jobs/${jobId}`, { method: "GET" });
  return { message: `Fetched job ${jobId}`, result };
}

async function executeResultFetch(step, runtime) {
  const payload = step.payload ?? {};
  const jobId = pickFirstString(payload, ["job_id", "jobId"]);
  const preferJobResult = payload.prefer_job_result !== false;
  if (preferJobResult) {
    const envelope = await requestJson(runtime.baseUrl, `/api/v1/jobs/${jobId}`, { method: "GET" });
    if (envelope?.result) {
      return { message: `Fetched result from job envelope ${jobId}`, result: envelope };
    }
  }
  if (payload.direct_mesh === true) {
    const nodes = await requestJson(runtime.baseUrl, `/api/direct-mesh/results/${jobId}/chunks/nodes`, { method: "GET" });
    const elements = await requestJson(runtime.baseUrl, `/api/direct-mesh/results/${jobId}/chunks/elements`, { method: "GET" });
    return {
      message: `Fetched direct-mesh result chunks for ${jobId}`,
      result: { job_id: jobId, nodes, elements },
    };
  }
  const result = await requestJson(runtime.baseUrl, `/api/v1/results/${jobId}`, { method: "GET" });
  return { message: `Fetched result record ${jobId}`, result };
}

async function executeDirectMeshSolve(step, runtime) {
  const payload = step.payload ?? {};
  const modelLike = typeof payload.model_version_id === "string"
    ? (await requestJson(runtime.baseUrl, `/api/v1/model-versions/${payload.model_version_id}`, { method: "GET" }))?.version ?? null
    : typeof payload.model_id === "string"
      ? (await requestJson(runtime.baseUrl, `/api/v1/models/${payload.model_id}`, { method: "GET" }))?.model ?? null
      : null;
  const { studyKind, input } = resolveStudyKindAndInput(payload, modelLike);
  const result = await requestJson(runtime.baseUrl, "/api/direct-mesh/solve", {
    method: "POST",
    body: JSON.stringify({
      study_kind: studyKind,
      input,
      endpoints: payload.endpoints,
      selection_mode: pickFirstString(payload, ["selection_mode", "selectionMode"]) ?? "first_reachable",
    }),
  });
  return { message: `Submitted direct-mesh solve for ${studyKind}`, result };
}

async function executeSolveFromModelVersion(step, runtime) {
  const payload = step.payload ?? {};
  const modelVersionId = pickFirstString(payload, ["model_version_id", "modelVersionId"]);
  const versionEnvelope = await requestJson(runtime.baseUrl, `/api/v1/model-versions/${modelVersionId}`, {
    method: "GET",
  });
  const version = versionEnvelope?.version;
  if (!version || typeof version !== "object") {
    throw new Error(`Could not load model version ${modelVersionId}`);
  }

  const result = await executeDirectMeshSolve(
    {
      ...step,
      payload: {
        ...payload,
        model_version_id: modelVersionId,
        model_payload: version.payload,
        study_kind:
          pickFirstString(payload, ["study_kind", "studyKind"]) ??
          (typeof version.kind === "string" ? version.kind : undefined),
        project_id:
          pickFirstString(payload, ["project_id", "projectId"]) ??
          (typeof version.project_id === "string" ? version.project_id : undefined),
      },
    },
    runtime,
  );

  return {
    ...result,
    message: `Submitted direct-mesh solve from model version ${modelVersionId}`,
  };
}

async function executeSolveAndWaitFromModelVersion(step, runtime) {
  const payload = step.payload ?? {};
  const solveResult = await executeSolveFromModelVersion(step, runtime);
  const jobId =
    solveResult?.result?.job?.job_id ??
    solveResult?.result?.job_id ??
    null;
  if (typeof jobId !== "string" || !jobId.trim()) {
    throw new Error("solve_and_wait_from_model_version could not resolve a job_id from solve response.");
  }

  const waited = await executeJobWait(
    {
      ...step,
      payload: {
        job_id: jobId,
        ...(payload.interval_ms !== undefined ? { interval_ms: payload.interval_ms } : {}),
        ...(payload.timeout_ms !== undefined ? { timeout_ms: payload.timeout_ms } : {}),
      },
    },
    runtime,
  );

  const fetched = await executeResultFetch(
    {
      ...step,
      payload: {
        job_id: jobId,
        ...(payload.prefer_job_result !== undefined ? { prefer_job_result: payload.prefer_job_result } : {}),
        ...(payload.direct_mesh !== undefined ? { direct_mesh: payload.direct_mesh } : {}),
      },
    },
    runtime,
  );

  return {
    message: `Solved, waited, and fetched result from model version ${payload.model_version_id}`,
    result: {
      solve: solveResult.result,
      wait: waited.result,
      result: fetched.result,
    },
  };
}

function resolveServiceActionExecutor(step) {
  const contract = findAutomationActionContract(step.action);
  switch (normalizeActionName(contract?.id ?? step.action)) {
    case "service_health":
      return executeServiceHealth;
    case "project_create":
      return executeProjectCreate;
    case "project_update":
      return executeProjectUpdate;
    case "project_delete":
      return executeProjectDelete;
    case "model_create":
      return executeModelCreate;
    case "model_version_create":
      return executeModelVersionCreate;
    case "workflow_submit_catalog":
      return executeWorkflowSubmitCatalog;
    case "workflow_submit_graph":
      return executeWorkflowSubmitGraph;
    case "job_wait":
      return executeJobWait;
    case "job_fetch":
      return executeJobFetch;
    case "result_fetch":
      return executeResultFetch;
    case "direct_mesh_solve":
      return executeDirectMeshSolve;
    case "solve_from_model_version":
      return executeSolveFromModelVersion;
    case "solve_and_wait_from_model_version":
      return executeSolveAndWaitFromModelVersion;
    default:
      return null;
  }
}

export async function createServiceExecutor(options = {}) {
  const runtime = {
    baseUrl: typeof options.baseUrl === "string" && options.baseUrl.trim() ? options.baseUrl.trim() : DEFAULT_API_BASE_URL,
  };

  return {
    baseUrl: runtime.baseUrl,
    executor: async (step) => {
      const actionExecutor = resolveServiceActionExecutor(step);
      if (!actionExecutor) {
        throw new Error(`Unsupported service action: ${step.action}`);
      }
      const result = await actionExecutor(step, runtime);
      return { executor: "service", ...result };
    },
    dispose: async () => {},
  };
}
