"use client";

import {
  createDirectMeshSolve,
  createModel,
  createModelVersion,
  createProject,
  fetchDirectMeshResultRecord,
  fetchHealth,
  fetchJobStatus,
  fetchModelVersion,
  fetchResultRecord,
  submitWorkflowCatalogJob,
  submitWorkflowGraphJob,
  type DirectMeshSelectionMode,
  type WorkflowGraphDefinition,
} from "@/lib/api";
import type { HeadlessExecutionValue, HeadlessWorkflowExecutionBatch } from "@/components/workbench/workbench-headless-workflow-export";

export type HeadlessExecutionEvent = {
  message: string;
};

type HeadlessExecutionStepResult = {
  index: number;
  action: string;
  result: Record<string, unknown>;
};

export type HeadlessExecutionRunResult = {
  steps: HeadlessExecutionStepResult[];
};

function asRecord(value: unknown) {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Record<string, unknown>) : {};
}

function asStringList(value: unknown) {
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string") : [];
}

function asSelectionMode(value: unknown): DirectMeshSelectionMode {
  return value === "healthiest" ? value : "first_reachable";
}

function resolveExecutionValue(value: HeadlessExecutionValue, results: Map<number, Record<string, unknown>>): unknown {
  if (value.kind === "literal") return value.value;
  if (value.kind === "array") return value.items.map((item) => resolveExecutionValue(item, results));
  if (value.kind === "object") {
    return Object.fromEntries(Object.entries(value.fields).map(([key, entry]) => [key, resolveExecutionValue(entry, results)]));
  }
  const stepResult = results.get(value.source.step);
  if (!stepResult) throw new Error(`Missing result for step ${value.source.step}.`);
  if (!(value.source.output in stepResult)) {
    throw new Error(`Step ${value.source.step} does not expose output ${value.source.output}.`);
  }
  return stepResult[value.source.output];
}

async function waitForJob(jobId: string, intervalMs: number, timeoutMs: number, onEvent?: (event: HeadlessExecutionEvent) => void) {
  const startedAt = Date.now();
  while (Date.now() - startedAt <= timeoutMs) {
    const envelope = await fetchJobStatus(jobId);
    onEvent?.({ message: `[job_wait] ${jobId} -> ${envelope.job.status}` });
    if (["completed", "failed", "cancelled"].includes(envelope.job.status)) return envelope;
    await new Promise((resolve) => window.setTimeout(resolve, intervalMs));
  }
  throw new Error(`Timed out while waiting for job ${jobId}.`);
}

async function runExecutionAction(action: string, payload: Record<string, unknown>, onEvent?: (event: HeadlessExecutionEvent) => void) {
  if (action === "service_health") {
    const response = await fetchHealth();
    return { service: response.service, status: response.status };
  }
  if (action === "project_create") {
    const response = await createProject({
      name: String(payload.name ?? "Headless Project"),
      ...(typeof payload.description === "string" ? { description: payload.description } : {}),
    });
    return { project_id: response.project.project_id, name: response.project.name };
  }
  if (action === "model_create") {
    const response = await createModel(String(payload.project_id ?? ""), {
      name: String(payload.name ?? "model"),
      kind: String(payload.kind ?? "truss_3d"),
      payload: asRecord(payload.payload),
      ...(typeof payload.material === "string" ? { material: payload.material } : {}),
      ...(typeof payload.model_schema_version === "string" ? { model_schema_version: payload.model_schema_version } : {}),
    });
    return { model_id: response.model.model_id, kind: response.model.kind };
  }
  if (action === "model_version_create") {
    const response = await createModelVersion(String(payload.model_id ?? ""), {
      payload: asRecord(payload.payload),
      ...(typeof payload.name === "string" ? { name: payload.name } : {}),
      ...(typeof payload.kind === "string" ? { kind: payload.kind } : {}),
      ...(typeof payload.material === "string" ? { material: payload.material } : {}),
      ...(typeof payload.model_schema_version === "string" ? { model_schema_version: payload.model_schema_version } : {}),
    });
    return { model_version_id: response.version.version_id, kind: response.version.kind };
  }
  if (action === "workflow_submit_catalog") {
    const response = await submitWorkflowCatalogJob(String(payload.workflow_id ?? ""), asRecord(payload.input_artifacts));
    return { job_id: response.job.job_id, status: response.job.status };
  }
  if (action === "workflow_submit_graph") {
    const response = await submitWorkflowGraphJob(payload.graph as WorkflowGraphDefinition, asRecord(payload.input_artifacts));
    return { job_id: response.job.job_id, status: response.job.status };
  }
  if (action === "job_fetch") {
    const response = await fetchJobStatus(String(payload.job_id ?? ""));
    return { job_id: response.job.job_id, status: response.job.status, progress: response.job.progress };
  }
  if (action === "job_wait") {
    const response = await waitForJob(
      String(payload.job_id ?? ""),
      typeof payload.interval_ms === "number" ? payload.interval_ms : 1000,
      typeof payload.timeout_ms === "number" ? payload.timeout_ms : 60_000,
      onEvent,
    );
    return { job_id: response.job.job_id, status: response.job.status, progress: response.job.progress };
  }
  if (action === "result_fetch") {
    const jobId = String(payload.job_id ?? "");
    if (payload.prefer_job_result) {
      const envelope = await fetchJobStatus(jobId);
      if (envelope.result && typeof envelope.result === "object") return { job_id: jobId, result: envelope.result as Record<string, unknown> };
    }
    const response = payload.direct_mesh ? await fetchDirectMeshResultRecord(jobId) : await fetchResultRecord(jobId);
    return { job_id: response.job_id, result: response.result };
  }
  if (action === "direct_mesh_solve") {
    const response = await createDirectMeshSolve(
      String(payload.study_kind ?? "truss_3d") as Parameters<typeof createDirectMeshSolve>[0],
      asRecord(payload.input && typeof payload.input === "object" ? payload.input : payload.model_payload),
      asStringList(payload.endpoints),
      asSelectionMode(payload.selection_mode),
    );
    return { job_id: response.job.job_id, status: response.job.status, endpoint: response.direct_mesh.endpoint };
  }
  if (action === "solve_from_model_version" || action === "solve_and_wait_from_model_version") {
    const version = await fetchModelVersion(String(payload.model_version_id ?? ""));
    const solve = await createDirectMeshSolve(
      version.version.kind as Parameters<typeof createDirectMeshSolve>[0],
      version.version.payload,
      asStringList(payload.endpoints),
      asSelectionMode(payload.selection_mode),
    );
    const baseResult: Record<string, unknown> = {
      job_id: solve.job.job_id,
      status: solve.job.status,
      model_version_id: version.version.version_id,
      endpoint: solve.direct_mesh.endpoint,
    };
    if (action === "solve_from_model_version") return baseResult;
    const waited = await waitForJob(
      solve.job.job_id,
      typeof payload.interval_ms === "number" ? payload.interval_ms : 1000,
      typeof payload.timeout_ms === "number" ? payload.timeout_ms : 60_000,
      onEvent,
    );
    const resultRecord = payload.direct_mesh ? await fetchDirectMeshResultRecord(solve.job.job_id) : await fetchResultRecord(solve.job.job_id);
    return { ...baseResult, status: waited.job.status, result: resultRecord.result };
  }
  throw new Error(`Unsupported headless execution action: ${action}`);
}

export async function runHeadlessExecutionBatch(
  batch: HeadlessWorkflowExecutionBatch,
  onEvent?: (event: HeadlessExecutionEvent) => void,
): Promise<HeadlessExecutionRunResult> {
  const results = new Map<number, Record<string, unknown>>();
  const completed: HeadlessExecutionStepResult[] = [];
  for (const step of batch.steps) {
    onEvent?.({ message: `[step ${step.index}] start ${step.action}` });
    const payload = asRecord(resolveExecutionValue(step.payload, results));
    const result = await runExecutionAction(step.action, payload, onEvent);
    results.set(step.index, result);
    completed.push({ index: step.index, action: step.action, result });
    onEvent?.({ message: `[step ${step.index}] done ${step.action}` });
  }
  return { steps: completed };
}
