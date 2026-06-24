import { chromium } from "playwright";
import { isRestrictedPlaywrightLaunchError, reportRestrictedPlaywrightSkip } from "./playwright-runtime-guard.mjs";

const baseUrl = process.env.WORKFLOW_BENCHMARK_URL || "http://127.0.0.1:3000/workflow-benchmark";
const iterations = Number(process.env.WORKFLOW_BENCHMARK_ITERATIONS || 5);
const tabs = ["overview", "catalog", "builder", "runs"];
const benchmarkWorkflowId = "workflow.synthetic.benchmark";

function round(value) {
  return Math.round(value * 1000) / 1000;
}

function buildSyntheticRun(workflowId) {
  const emittedAt = new Date().toISOString();
  return [{
    jobId: `bench-${Date.now()}`,
    workflowId,
    status: "completed",
    progress: 1,
    currentNode: "export.summary",
    summary: "Synthetic workflow benchmark run",
    updatedAt: emittedAt,
    skippedNodes: ["condition.route"],
    branchDecisions: [{ node_id: "condition.route", chosen_output: "if_true", predicate_result: true }],
    nodeRuns: [
      { node_id: "input.source", kind: "input", status: "completed", produced_artifacts: ["artifact.source"] },
      { node_id: "extract.summary", kind: "extract", operator_id: "extract.result_summary", status: "completed", consumed_artifacts: ["artifact.source"], produced_artifacts: ["artifact.summary"] },
      { node_id: "transform.normalize", kind: "transform", operator_id: "transform.normalize_summary_fields", status: "completed", consumed_artifacts: ["artifact.summary"], produced_artifacts: ["artifact.normalized"] },
      { node_id: "condition.route", kind: "condition", status: "skipped", consumed_artifacts: ["artifact.normalized"] },
      { node_id: "export.summary", kind: "export", operator_id: "export.summary_json", status: "completed", consumed_artifacts: ["artifact.normalized"], produced_artifacts: ["artifact.export"] },
    ],
    artifactLineage: [
      { artifact_key: "artifact.source", node_id: "input.source", port_id: "out" },
      { artifact_key: "artifact.summary", node_id: "extract.summary", port_id: "summary", source_artifacts: ["artifact.source"] },
      { artifact_key: "artifact.normalized", node_id: "transform.normalize", port_id: "normalized", source_artifacts: ["artifact.summary"] },
      { artifact_key: "artifact.export", node_id: "export.summary", port_id: "file", source_artifacts: ["artifact.normalized"] },
    ],
    traceSummary: {
      branchDecisionCount: 1,
      completedNodeRunCount: 4,
      skippedNodeRunCount: 1,
      rootArtifactCount: 1,
      derivedArtifactCount: 3,
      progressEventCount: 6,
      latestProgressLabel: "completed",
      recentProgressEvents: [
        { stage: "completed", progress: 1, label: "workflow completed", emittedAt, kind: "workflow" },
        { stage: "postprocessing", progress: 0.92, label: "export summary", emittedAt, nodeId: "export.summary", kind: "export" },
        { stage: "solving", progress: 0.7, label: "normalize summary", emittedAt, nodeId: "transform.normalize", kind: "transform" },
        { stage: "preprocessing", progress: 0.44, label: "extract result summary", emittedAt, nodeId: "extract.summary", kind: "extract" },
        { stage: "queued", progress: 0.12, label: "input accepted", emittedAt, nodeId: "input.source", kind: "input" },
      ],
      latestBranchPredicate: true,
    },
  }];
}

function buildMockWorkflowCatalog() {
  return {
    workflows: [
      {
        id: benchmarkWorkflowId,
        name: "Synthetic Benchmark Workflow",
        version: "1.11.0-bench",
        summary: "Synthetic workflow used by the local benchmark harness.",
        domains: ["benchmark"],
        capability_tags: ["contract_health:clean", "benchmark"],
        graph: {
          schema_version: "kyuubiki.workflow-graph/v1",
          id: benchmarkWorkflowId,
          name: "Synthetic Benchmark Workflow",
          version: "1.11.0-bench",
          entry_inputs: [{ node_id: "input.source", artifact_type: "study.result", description: "Synthetic result input" }],
          output_artifacts: [{ node_id: "export.summary", artifact_type: "report.html", description: "Synthetic export artifact" }],
          entry_nodes: ["input.source"],
          output_nodes: ["export.summary"],
          nodes: [
            { id: "input.source", kind: "input", outputs: [{ id: "out", artifact_type: "study.result", description: "result payload" }] },
            { id: "extract.summary", kind: "extract", operator_id: "extract.result_summary", inputs: [{ id: "source", artifact_type: "study.result", description: "raw result" }], outputs: [{ id: "summary", artifact_type: "workflow.summary", description: "summary payload" }] },
            { id: "transform.normalize", kind: "transform", operator_id: "transform.normalize_summary_fields", inputs: [{ id: "summary", artifact_type: "workflow.summary", description: "summary" }], outputs: [{ id: "normalized", artifact_type: "workflow.summary", description: "normalized summary" }] },
            { id: "export.summary", kind: "export", operator_id: "export.summary_json", inputs: [{ id: "summary", artifact_type: "workflow.summary", description: "normalized summary" }], outputs: [{ id: "file", artifact_type: "report.html", description: "export file" }] },
          ],
          edges: [
            { id: "edge.input.extract", from: { node: "input.source", port: "out" }, to: { node: "extract.summary", port: "source" }, artifact_type: "study.result" },
            { id: "edge.extract.transform", from: { node: "extract.summary", port: "summary" }, to: { node: "transform.normalize", port: "summary" }, artifact_type: "workflow.summary" },
            { id: "edge.transform.export", from: { node: "transform.normalize", port: "normalized" }, to: { node: "export.summary", port: "summary" }, artifact_type: "workflow.summary" },
          ],
        },
        entry_inputs: [{ node_id: "input.source", artifact_type: "study.result", description: "Synthetic result input" }],
        output_artifacts: [{ node_id: "export.summary", artifact_type: "report.html", description: "Synthetic export artifact" }],
      },
    ],
  };
}

function buildMockHealth() {
  return {
    service: "kyuubiki-benchmark",
    status: "healthy",
    protocol: {
      program: "benchmark",
      role: "mock",
      protocol: { name: "http", version: 1, transport: { kind: "http", encoding: "json" } },
      compatible_solver_rpc: { name: "solver-rpc", rpc_version: 1, transport: { kind: "tcp", framing: "jsonl", encoding: "json" }, methods: [] },
    },
    deployment: { mode: "benchmark", discovery: "mock", manifest_path: null, endpoint_count: 0 },
    remote_solver_registry: { active_agents: 0 },
    security: {
      api_token_configured: false,
      cluster_token_configured: false,
      cluster_agent_allowlist_enabled: false,
      cluster_agent_allowlist_count: 0,
      cluster_cluster_allowlist_enabled: false,
      cluster_cluster_allowlist_count: 0,
      cluster_fingerprint_required: false,
      cluster_timestamp_window_ms: 30000,
      protect_reads: false,
      mutating_routes_protected: false,
      cluster_routes_protected: false,
    },
    watchdog: { scan_interval_ms: 1000, stale_job_ms: 30000, job_timeout_ms: 120000, active_jobs: 0, stalled_jobs: 0, timed_out_jobs: 0 },
    transport: { http: 4000, solver_agent_tcp: 5001 },
    solver_agents: [],
  };
}

async function installApiMocks(page) {
  const mockWorkflowCatalog = buildMockWorkflowCatalog();
  const mockHealth = buildMockHealth();
  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    const { pathname } = url;
    const fulfillJson = (body) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(body),
      });
    if (pathname === "/api/health") return fulfillJson(mockHealth);
    if (pathname === "/api/v1/protocol/agents") return fulfillJson({ agents: [] });
    if (pathname === "/api/v1/jobs") return fulfillJson({ jobs: [] });
    if (pathname.startsWith("/api/v1/jobs/")) return fulfillJson({ job: { job_id: pathname.split("/").pop() ?? "job-bench", status: "completed", worker_id: null, progress: 1 } });
    if (pathname === "/api/v1/projects") return fulfillJson({ projects: [] });
    if (pathname === "/api/v1/workflows/catalog") return fulfillJson(mockWorkflowCatalog);
    if (pathname === "/api/v1/operators") return fulfillJson({ operators: [] });
    if (pathname.endsWith("/jobs") && pathname.includes("/api/v1/workflows/catalog/")) {
      return fulfillJson({ job: { job_id: `job-${Date.now()}`, status: "completed", worker_id: null, progress: 1 }, result: { workflow_id: benchmarkWorkflowId, completed_nodes: [], artifacts: {} } });
    }
    if (pathname === "/api/v1/workflows/graph/jobs") {
      return fulfillJson({ job: { job_id: `job-${Date.now()}`, status: "completed", worker_id: null, progress: 1 }, result: { workflow_id: benchmarkWorkflowId, completed_nodes: [], artifacts: {} } });
    }
    if (pathname === "/api/v1/security-events") return fulfillJson({ events: [] });
    return fulfillJson({});
  });
}

function summarize(values) {
  const sorted = [...values].sort((left, right) => left - right);
  const avg = sorted.reduce((sum, value) => sum + value, 0) / sorted.length;
  const p95Index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(sorted.length * 0.95) - 1));
  return {
    count: sorted.length,
    min: round(sorted[0] ?? 0),
    avg: round(avg || 0),
    p95: round(sorted[p95Index] ?? 0),
    max: round(sorted[sorted.length - 1] ?? 0),
  };
}

async function waitForAppReady(page) {
  await page.waitForFunction(() => {
    return Boolean(
      window.__kyuubikiWorkflowDebug &&
      window.__kyuubikiPerf?.workflow,
    );
  }, { timeout: 30_000 });
}

async function resetPerfState(page) {
  await page.evaluate(() => {
    if (!window.__kyuubikiPerf?.workflow) return;
    window.__kyuubikiPerf.workflow.surfaceIntent = {};
    window.__kyuubikiPerf.workflow.surfaceMeasures = {};
    window.__kyuubikiPerf.workflow.traceCardMs = null;
    window.__kyuubikiPerf.workflow.updatedAt = null;
  });
}

async function measureTab(page, tab, workflowId, syntheticRuns) {
  await resetPerfState(page);
  const result = await page.evaluate(async ({ currentTab, currentWorkflowId, runs, tabsToMeasure }) => {
    const startedAt = performance.now();
    const toggleTab = tabsToMeasure.find((entry) => entry !== currentTab) ?? currentTab;
    const waitForPaint = () =>
      new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    if (currentWorkflowId) window.__kyuubikiWorkflowDebug?.setSelectedWorkflowId(currentWorkflowId);
    if (currentTab === "runs") window.__kyuubikiWorkflowDebug?.replaceRuns(runs);
    window.__kyuubikiWorkflowDebug?.setSurfaceTab(toggleTab);
    await waitForPaint();
    window.__kyuubikiWorkflowDebug?.setSurfaceTab(currentTab);
    await waitForPaint();
    if (currentTab === "runs") {
      await new Promise((resolve) => window.setTimeout(resolve, 220));
    }
    const measuredSurfaceMs = window.__kyuubikiPerf?.workflow?.surfaceMeasures?.[currentTab] ?? null;
    return {
      surfaceMs: typeof measuredSurfaceMs === "number" ? measuredSurfaceMs : performance.now() - startedAt,
      traceCardMs: currentTab === "runs" ? window.__kyuubikiPerf?.workflow?.traceCardMs ?? null : null,
    };
  }, { currentTab: tab, currentWorkflowId: workflowId, runs: syntheticRuns, tabsToMeasure: tabs });
  return result;
}

async function waitForDoublePaint(page) {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function openBuilderTab(page, workflowId) {
  await resetPerfState(page);
  await page.evaluate(async ({ currentWorkflowId }) => {
    const waitForPaint = () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    if (currentWorkflowId) window.__kyuubikiWorkflowDebug?.setSelectedWorkflowId(currentWorkflowId);
    window.__kyuubikiWorkflowDebug?.setSurfaceTab("catalog");
    await waitForPaint();
    window.__kyuubikiWorkflowDebug?.setSurfaceTab("builder");
  }, { currentWorkflowId: workflowId });
  await waitForDoublePaint(page);
}

async function measureInputEdit(page, selector, nextValue) {
  const locator = page.locator(selector);
  await locator.waitFor({ state: "visible", timeout: 10_000 });
  const result = await page.evaluate(async ({ fieldSelector, value }) => {
    const input = document.querySelector(fieldSelector);
    if (!(input instanceof HTMLInputElement)) return null;
    const startedAt = performance.now();
    input.focus();
    input.value = value;
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
    input.blur();
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    return performance.now() - startedAt;
  }, { fieldSelector: selector, value: nextValue });
  const committedValue = await locator.inputValue();
  return {
    durationMs: typeof result === "number" ? result : 0,
    committedValue,
  };
}

async function measureBuilderEdits(page, workflowId) {
  await openBuilderTab(page, workflowId);
  const nodeField = '[data-workflow-port-field="extract.summary:outputs:summary:description"]';
  const edgeField = '[data-workflow-edge-field="edge.extract.transform:artifact_type"]';
  const nodeSamples = [];
  const edgeSamples = [];
  for (let index = 0; index < iterations; index += 1) {
    const nodeValue = `summary payload bench ${index}`;
    const edgeValue = `workflow.summary.bench.${index}`;
    const nodeResult = await measureInputEdit(page, nodeField, nodeValue);
    const edgeResult = await measureInputEdit(page, edgeField, edgeValue);
    if (nodeResult.committedValue === nodeValue) nodeSamples.push(nodeResult.durationMs);
    if (edgeResult.committedValue === edgeValue) edgeSamples.push(edgeResult.durationMs);
  }
  return {
    nodePortDescription: summarize(nodeSamples),
    edgeArtifactType: summarize(edgeSamples),
  };
}

async function run() {
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
  } catch (error) {
    if (isRestrictedPlaywrightLaunchError(error)) {
      reportRestrictedPlaywrightSkip("Workflow benchmark", error);
      return;
    }
    throw error;
  }
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  try {
    await installApiMocks(page);
    const response = await page.goto(baseUrl, { waitUntil: "networkidle", timeout: 30_000 });
    if (!response?.ok()) throw new Error(`page responded with HTTP ${response?.status() ?? "unknown"}`);
    await waitForAppReady(page);
    const workflowId = await page.evaluate(() => {
      const state = window.__kyuubikiWorkflowDebug?.getState();
      return state?.selectedWorkflowId ?? state?.catalogWorkflowIds?.[0] ?? benchmarkWorkflowId;
    });
    const syntheticRuns = buildSyntheticRun(workflowId);
    const samples = new Map(tabs.map((tab) => [tab, []]));
    const traceCardSamples = [];
    for (let index = 0; index < iterations; index += 1) {
      for (const tab of tabs) {
        const result = await measureTab(page, tab, workflowId, syntheticRuns);
        samples.get(tab)?.push(result.surfaceMs ?? 0);
        if (tab === "runs" && typeof result.traceCardMs === "number") traceCardSamples.push(result.traceCardMs);
      }
    }
    const builderEdits = await measureBuilderEdits(page, workflowId);
    const summary = Object.fromEntries(
      [...samples.entries()].map(([tab, values]) => [tab, summarize(values)]),
    );
    const output = {
      url: baseUrl,
      workflowId,
      iterations,
      surfaceTabs: summary,
      builderEdits,
      traceCard: traceCardSamples.length > 0 ? summarize(traceCardSamples) : null,
    };
    console.log("Workflow benchmark summary");
    console.table(Object.entries(summary).map(([tab, metrics]) => ({ tab, ...metrics })));
    console.table([
      { tab: "builder-edit:node-port-description", ...builderEdits.nodePortDescription },
      { tab: "builder-edit:edge-artifact-type", ...builderEdits.edgeArtifactType },
    ]);
    if (output.traceCard) console.table([{ tab: "trace-card", ...output.traceCard }]);
    console.log(JSON.stringify(output, null, 2));
  } finally {
    await page.close();
    await browser.close();
  }
}

run().catch((error) => {
  if (isRestrictedPlaywrightLaunchError(error)) {
    reportRestrictedPlaywrightSkip("Workflow benchmark", error);
    process.exit(0);
  }
  console.error(`Workflow benchmark failed: ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
