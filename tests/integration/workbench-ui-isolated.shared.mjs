import { spawn } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, rmSync, symlinkSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import net from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const FRONTEND_ROOT = path.join(ROOT, "apps/frontend");
const requireFromFrontend = createRequire(path.join(FRONTEND_ROOT, "package.json"));
const nextBin = requireFromFrontend.resolve("next/dist/bin/next");
export const { chromium } = requireFromFrontend("playwright");

const FIXED_AT = "2026-08-13T00:00:00.000Z";
const WORKFLOW_ID = "workflow.bar-1d-summary-json";
const JOB_ID = "qualification-workflow-job";

function qualificationProject(overrides = {}) {
  return {
    project_id: "qualification-project",
    name: "Qualification project",
    description: "Isolated browser qualification",
    inserted_at: FIXED_AT,
    updated_at: FIXED_AT,
    models: [],
    ...overrides,
  };
}

function job(status, progress, message) {
  return {
    job_id: JOB_ID,
    status,
    worker_id: status === "queued" ? null : "qualification-agent",
    message,
    progress,
    has_result: status === "completed",
    created_at: FIXED_AT,
    updated_at: FIXED_AT,
  };
}

const workflow = {
  id: WORKFLOW_ID,
  name: "Mechanical bar qualification",
  version: "1.0.0",
  summary: "Mechanical workflow used by the isolated Workbench qualification.",
  domains: ["mechanical"],
  capability_tags: ["mechanical", "qualification", "summary"],
  entry_inputs: [
    {
      node_id: "bar_1d_model",
      artifact_type: "study_model/bar_1d",
      description: "Mechanical bar model",
    },
  ],
  output_artifacts: [
    {
      node_id: "bar_1d_model",
      artifact_type: "result/bar_1d",
      description: "Mechanical bar result",
    },
  ],
  graph: {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: WORKFLOW_ID,
    name: "Mechanical bar qualification",
    version: "1.0.0",
    entry_nodes: ["bar_1d_model"],
    output_nodes: ["bar_1d_model"],
    nodes: [
      {
        id: "bar_1d_model",
        kind: "solve",
        operator_id: "solve.bar_1d",
        inputs: [{ id: "model", artifact_type: "study_model/bar_1d" }],
        outputs: [{ id: "result", artifact_type: "result/bar_1d" }],
      },
    ],
    edges: [],
  },
};

const operator = {
  id: "solve.bar_1d",
  version: "1.0.0",
  domain: "mechanical",
  family: "structural",
  kind: "solver",
  summary: "Solve a one-dimensional mechanical bar.",
  capability_tags: ["mechanical", "structural", "headless_safe"],
  origin: "built_in",
  input_schema: { schema: "kyuubiki.operator.solve.bar_1d.input", version: "1" },
  output_schema: { schema: "kyuubiki.operator.solve.bar_1d.output", version: "1" },
  inputs: [{ id: "model", artifact_type: "study_model/bar_1d", description: "Bar model" }],
  outputs: [{ id: "result", artifact_type: "result/bar_1d", description: "Bar result" }],
  validation: {
    baseline_status: "verified",
    baseline_cases: ["qualification-bar"],
    smoke_paths: ["isolated-workbench-browser"],
  },
};

const completedResult = {
  workflow_id: WORKFLOW_ID,
  current_node: "bar_1d_model",
  completed_nodes: ["bar_1d_model"],
  skipped_nodes: [],
  branch_decisions: [],
  node_runs: [
    {
      node_id: "bar_1d_model",
      kind: "solve",
      operator_id: "solve.bar_1d",
      status: "completed",
      consumed_artifacts: ["bar_1d_model"],
      produced_artifacts: ["bar_result"],
    },
  ],
  artifact_lineage: [
    {
      artifact_key: "bar_result",
      node_id: "bar_1d_model",
      port_id: "result",
      source_artifacts: ["bar_1d_model"],
    },
  ],
  artifacts: {
    "summary.json": {
      artifact_type: "artifact/result_summary",
      node_id: "bar_1d_model",
      port_id: "result",
      payload: {
        contract_version: "kyuubiki.workflow-summary-artifact/v1",
        summary_kind: "mechanical_bar",
        fields: { max_displacement: 0.00125, node_count: 3 },
      },
    },
  },
};

function respondJson(response, status, payload, request) {
  const requestedHeaders = request.headers["access-control-request-headers"];
  response.writeHead(status, {
    "access-control-allow-headers": requestedHeaders || "authorization,content-type,x-kyuubiki-api-token",
    "access-control-allow-methods": "GET,POST,PATCH,DELETE,OPTIONS",
    "access-control-allow-origin": request.headers.origin || "*",
    "content-type": "application/json; charset=utf-8",
  });
  response.end(JSON.stringify(payload));
}

async function readJsonBody(request) {
  const chunks = [];
  for await (const chunk of request) chunks.push(chunk);
  const text = Buffer.concat(chunks).toString("utf8");
  return text ? JSON.parse(text) : null;
}

function createBackendState() {
  return {
    catalogFetches: 0,
    operatorFetches: 0,
    catalogSubmissions: 0,
    graphSubmissions: 0,
    jobPolls: 0,
    historyFetches: 0,
    submissionBodies: [],
    projects: [qualificationProject()],
    projectMutations: [],
    adminJobs: [],
    adminResults: [],
    jobRecordMutations: [],
    resultRecordMutations: [],
  };
}

async function startMockBackend() {
  const state = createBackendState();
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url || "/", "http://127.0.0.1");
      if (request.method === "OPTIONS") return respondJson(response, 204, {}, request);
      if (request.method === "GET" && url.pathname === "/api/v1/workflows/catalog") {
        state.catalogFetches += 1;
        return respondJson(response, 200, { workflows: [workflow] }, request);
      }
      if (request.method === "GET" && url.pathname === "/api/v1/operators") {
        state.operatorFetches += 1;
        return respondJson(response, 200, { modules: [], operators: [operator] }, request);
      }
      if (request.method === "POST" && url.pathname === `/api/v1/workflows/catalog/${WORKFLOW_ID}/jobs`) {
        state.catalogSubmissions += 1;
        state.submissionBodies.push(await readJsonBody(request));
        return respondJson(response, 202, { job: job("queued", 0, "queued") }, request);
      }
      if (request.method === "POST" && url.pathname === "/api/v1/workflows/graph/jobs") {
        state.graphSubmissions += 1;
        state.submissionBodies.push(await readJsonBody(request));
        return respondJson(response, 202, { job: job("queued", 0, "queued") }, request);
      }
      if (request.method === "GET" && url.pathname === `/api/v1/jobs/${JOB_ID}`) {
        state.jobPolls += 1;
        return respondJson(response, 200, {
          job: job("completed", 1, "bar_1d_model"),
          result: completedResult,
        }, request);
      }
      if (request.method === "GET" && url.pathname === "/api/v1/jobs") {
        state.historyFetches += 1;
        const workflowJobs = state.catalogSubmissions + state.graphSubmissions > 0
          ? [job("completed", 1, "bar_1d_model")]
          : [];
        return respondJson(response, 200, { jobs: [...state.adminJobs, ...workflowJobs] }, request);
      }
      if (request.method === "GET" && url.pathname === "/api/health") {
        return respondJson(response, 200, { service: "qualification-runtime", status: "ok" }, request);
      }
      if (request.method === "GET" && url.pathname === "/api/v1/protocol/agents") {
        return respondJson(response, 200, { agents: [] }, request);
      }
      if (request.method === "GET" && url.pathname === "/api/v1/agents") {
        return respondJson(response, 200, {
          agents: [],
          summary: { active_execution_lease_count: 0, stale_execution_lease_count: 0 },
        }, request);
      }
      if (request.method === "GET" && url.pathname === "/api/v1/results") {
        return respondJson(response, 200, { results: state.adminResults }, request);
      }
      if (request.method === "GET" && url.pathname === "/api/v1/projects") {
        return respondJson(response, 200, { projects: state.projects }, request);
      }
      if (request.method === "POST" && url.pathname === "/api/v1/projects") {
        const body = await readJsonBody(request);
        const project = qualificationProject({
          project_id: `qualification-created-project-${state.projectMutations.length + 1}`,
          name: typeof body?.name === "string" ? body.name : "Qualification created project",
          description: typeof body?.description === "string" ? body.description : "",
        });
        state.projects.push(project);
        state.projectMutations.push({ method: "POST", project_id: project.project_id, body });
        return respondJson(response, 201, { project }, request);
      }
      const projectMatch = url.pathname.match(/^\/api\/v1\/projects\/([^/]+)$/u);
      if (projectMatch && request.method === "PATCH") {
        const projectId = decodeURIComponent(projectMatch[1]);
        const index = state.projects.findIndex((entry) => entry.project_id === projectId);
        if (index < 0) return respondJson(response, 404, { error: "project not found" }, request);
        const body = await readJsonBody(request);
        const project = {
          ...state.projects[index],
          ...(typeof body?.name === "string" ? { name: body.name } : {}),
          ...(typeof body?.description === "string" ? { description: body.description } : {}),
          updated_at: FIXED_AT,
        };
        state.projects[index] = project;
        state.projectMutations.push({ method: "PATCH", project_id: projectId, body });
        return respondJson(response, 200, { project }, request);
      }
      if (projectMatch && request.method === "DELETE") {
        const projectId = decodeURIComponent(projectMatch[1]);
        const index = state.projects.findIndex((entry) => entry.project_id === projectId);
        if (index < 0) return respondJson(response, 404, { error: "project not found" }, request);
        const [project] = state.projects.splice(index, 1);
        state.projectMutations.push({ method: "DELETE", project_id: projectId, body: null });
        return respondJson(response, 200, { project }, request);
      }
      const jobRecordMatch = url.pathname.match(/^\/api\/v1\/jobs\/([^/]+)$/u);
      if (jobRecordMatch && request.method === "PATCH") {
        const jobId = decodeURIComponent(jobRecordMatch[1]);
        const index = state.adminJobs.findIndex((entry) => entry.job_id === jobId);
        if (index < 0) return respondJson(response, 404, { error: "job not found" }, request);
        const body = await readJsonBody(request);
        const updated = { ...state.adminJobs[index], ...body, updated_at: FIXED_AT };
        state.adminJobs[index] = updated;
        state.jobRecordMutations.push({ method: "PATCH", job_id: jobId, body });
        return respondJson(response, 200, { job: updated }, request);
      }
      if (jobRecordMatch && request.method === "DELETE") {
        const jobId = decodeURIComponent(jobRecordMatch[1]);
        const index = state.adminJobs.findIndex((entry) => entry.job_id === jobId);
        if (index < 0) return respondJson(response, 404, { error: "job not found" }, request);
        const [deletedJob] = state.adminJobs.splice(index, 1);
        state.jobRecordMutations.push({ method: "DELETE", job_id: jobId, body: null });
        return respondJson(response, 200, { deleted: true, job: deletedJob }, request);
      }
      const resultRecordMatch = url.pathname.match(/^\/api\/v1\/results\/([^/]+)$/u);
      if (resultRecordMatch && request.method === "PATCH") {
        const jobId = decodeURIComponent(resultRecordMatch[1]);
        const index = state.adminResults.findIndex((entry) => entry.job_id === jobId);
        if (index < 0) return respondJson(response, 404, { error: "result not found" }, request);
        const body = await readJsonBody(request);
        const updated = { ...state.adminResults[index], result: body?.result ?? {}, updated_at: FIXED_AT };
        state.adminResults[index] = updated;
        state.resultRecordMutations.push({ method: "PATCH", job_id: jobId, body });
        return respondJson(response, 200, { job_id: jobId, result: updated.result }, request);
      }
      if (resultRecordMatch && request.method === "DELETE") {
        const jobId = decodeURIComponent(resultRecordMatch[1]);
        const index = state.adminResults.findIndex((entry) => entry.job_id === jobId);
        if (index < 0) return respondJson(response, 404, { error: "result not found" }, request);
        const [deletedResult] = state.adminResults.splice(index, 1);
        state.resultRecordMutations.push({ method: "DELETE", job_id: jobId, body: null });
        return respondJson(response, 200, { ...deletedResult, deleted: true }, request);
      }
      if (request.method === "GET" && url.pathname === "/api/v1/security-events") {
        return respondJson(response, 200, { events: [] }, request);
      }
      return respondJson(response, 404, { error: `unhandled qualification route: ${request.method} ${url.pathname}` }, request);
    } catch (error) {
      return respondJson(response, 500, { error: error instanceof Error ? error.message : String(error) }, request);
    }
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("mock backend did not allocate a TCP port");
  return {
    state,
    url: `http://127.0.0.1:${address.port}`,
    stop: () => new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve())),
  };
}

async function reservePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("frontend did not reserve a TCP port");
  const port = address.port;
  await new Promise((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  return port;
}

function collectProcessLog(child) {
  let log = "";
  const append = (chunk) => {
    log = `${log}${chunk.toString("utf8")}`.slice(-24_000);
  };
  child.stdout?.on("data", append);
  child.stderr?.on("data", append);
  return () => log;
}

async function waitForFrontend(url, child, readLog, timeoutMs = 150_000) {
  const deadline = Date.now() + timeoutMs;
  let serverErrorCount = 0;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(`isolated Next process exited with ${child.exitCode}\n${readLog()}`);
    }
    try {
      const response = await fetch(url);
      if (response.ok) return;
      serverErrorCount = response.status >= 500 ? serverErrorCount + 1 : 0;
      if (serverErrorCount >= 3) {
        throw new Error(`isolated Workbench returned ${response.status}\n${readLog()}`);
      }
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 350));
  }
  throw new Error(`timed out waiting for isolated Workbench at ${url}\n${readLog()}`);
}

async function stopChild(child) {
  if (child.exitCode !== null) return;
  child.kill("SIGTERM");
  await Promise.race([
    new Promise((resolve) => child.once("exit", resolve)),
    new Promise((resolve) => setTimeout(resolve, 5_000)),
  ]);
  if (child.exitCode === null) child.kill("SIGKILL");
}

function createIsolatedFrontendWorkspace() {
  const workspaceBase = mkdtempSync(path.join(tmpdir(), "kyuubiki-workbench-ui-"));
  try {
    const repoRoot = path.join(workspaceBase, "repo");
    const workspaceRoot = path.join(repoRoot, "apps/frontend");
    mkdirSync(path.dirname(workspaceRoot), { recursive: true });
    cpSync(FRONTEND_ROOT, workspaceRoot, {
      recursive: true,
      filter(source) {
        const relative = path.relative(FRONTEND_ROOT, source);
        if (!relative) return true;
        const topLevel = relative.split(path.sep)[0];
        return topLevel !== "node_modules" && topLevel !== ".next" && !topLevel.startsWith(".next-qualification-");
      },
    });
    cpSync(path.join(ROOT, "assets"), path.join(repoRoot, "assets"), { recursive: true });
    symlinkSync(
      path.join(FRONTEND_ROOT, "node_modules"),
      path.join(workspaceRoot, "node_modules"),
      process.platform === "win32" ? "junction" : "dir",
    );
    return { workspaceBase, workspaceRoot };
  } catch (error) {
    rmSync(workspaceBase, { force: true, recursive: true });
    throw error;
  }
}

export async function startIsolatedWorkbenchUiRuntime() {
  const { workspaceBase, workspaceRoot } = createIsolatedFrontendWorkspace();
  let backend;
  let child;
  let stopped = false;
  async function cleanup() {
    if (stopped) return;
    stopped = true;
    if (child) await stopChild(child);
    if (backend) await backend.stop().catch(() => undefined);
    rmSync(workspaceBase, { force: true, recursive: true });
  }
  try {
    backend = await startMockBackend();
    const port = await reservePort();
    child = spawn(process.execPath, [nextBin, "dev", "-H", "127.0.0.1", "-p", String(port)], {
      cwd: workspaceRoot,
      env: { ...process.env },
      stdio: ["ignore", "pipe", "pipe"],
    });
    const readLog = collectProcessLog(child);
    const frontendUrl = `http://127.0.0.1:${port}`;
    await waitForFrontend(frontendUrl, child, readLog);
    return {
      backendUrl: backend.url,
      frontendUrl,
      state: backend.state,
      stop: cleanup,
    };
  } catch (error) {
    await cleanup();
    throw error;
  }
}

export function workbenchUrl(runtime) {
  return `${runtime.frontendUrl}?kyuubikiApiBaseUrl=${encodeURIComponent(runtime.backendUrl)}`;
}
