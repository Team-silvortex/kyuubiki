import test from "node:test";
import assert from "node:assert/strict";

import {
  WORKBENCH_UI_STREAMING_CONTRACT_VERSION,
  listWorkbenchUiChunkContracts,
  resolveWorkbenchUiChunkLoadDecision,
  resolveWorkbenchUiChunkLoadPlan,
  resolveWorkbenchUiStreamingState,
} from "../../src/components/workbench/workbench-ui-streaming.ts";
import {
  buildWorkbenchUiChunkRuntimeAttrs,
  createWorkbenchUiStreamingRuntime,
} from "../../src/components/workbench/workbench-ui-streaming-runtime.ts";

test("workbench UI streaming contract exposes grouped chunks", () => {
  const chunks = listWorkbenchUiChunkContracts();
  const ids = chunks.map((chunk) => chunk.id);

  assert.equal(WORKBENCH_UI_STREAMING_CONTRACT_VERSION, "kyuubiki.workbench-ui-streaming/v1");
  assert.equal(new Set(ids).size, ids.length);
  assert(ids.includes("section.workflow"));
  assert(ids.includes("runtime.wasm-python"));
  assert(chunks.every((chunk) => chunk.estimatedWeight > 0));
});

test("workflow streaming state activates workflow and prefetches wasm automation", () => {
  const state = resolveWorkbenchUiStreamingState("workflow");

  assert.equal(state.budgetStatus, "ok");
  assert(state.activeChunks.includes("shell.rail"));
  assert(state.activeChunks.includes("workspace.viewport"));
  assert(state.activeChunks.includes("section.workflow"));
  assert(state.prefetchChunks.includes("runtime.wasm-python"));
  assert(state.evictableChunks.includes("section.model"));
});

test("model streaming state keeps 3d renderer warm but evictable", () => {
  const state = resolveWorkbenchUiStreamingState("model");

  assert(state.activeChunks.includes("section.model"));
  assert(state.prefetchChunks.includes("renderer.truss3d"));
  assert(state.evictableChunks.includes("section.workflow"));
  assert(!state.evictableChunks.includes("shell.sidebar"));
});

test("workflow chunk load plan separates immediate load from idle prefetch", () => {
  const plan = resolveWorkbenchUiChunkLoadPlan("workflow");

  assert(plan.loadNow.includes("section.workflow"));
  assert(plan.loadNow.includes("workspace.viewport"));
  assert(plan.prefetchWhenIdle.includes("runtime.wasm-python"));
  assert(plan.prefetchWhenIdle.includes("workspace.console"));
  assert(plan.evictable.includes("section.model"));
  assert(!plan.evictable.includes("shell.rail"));
});

test("chunk load plan is priority ordered for deterministic scheduling", () => {
  const plan = resolveWorkbenchUiChunkLoadPlan("model");
  const priorities = plan.decisions.map((entry) => entry.priority);
  const sorted = [...priorities].sort((left, right) => right - left);

  assert.deepEqual(priorities, sorted);
  assert.equal(plan.decisions[0]?.id, "shell.rail");
  assert(plan.prefetchWhenIdle.includes("renderer.truss3d"));
});

test("chunk load decision exposes the same phase used by chunk wrappers", () => {
  const workflowChunk = resolveWorkbenchUiChunkLoadDecision("workflow", "section.workflow");
  const modelChunk = resolveWorkbenchUiChunkLoadDecision("workflow", "section.model");
  const consoleChunk = resolveWorkbenchUiChunkLoadDecision("workflow", "workspace.console");

  assert.equal(workflowChunk.phase, "load");
  assert.equal(modelChunk.phase, "evict");
  assert.equal(consoleChunk.phase, "prefetch");
  assert.equal(workflowChunk.renderPolicy, "visible");
  assert.equal(modelChunk.renderPolicy, "hidden");
  assert.equal(consoleChunk.renderPolicy, "idle");
  assert(workflowChunk.priority > modelChunk.priority);
});

test("streaming runtime exposes shell and chunk attributes from one plan", () => {
  const runtime = createWorkbenchUiStreamingRuntime("workflow");
  const workflowAttrs = runtime.chunkAttrs("section.workflow");
  const modelAttrs = buildWorkbenchUiChunkRuntimeAttrs("workflow", "section.model");

  assert.equal(runtime.shellAttrs["data-workbench-ui-streaming-contract"], WORKBENCH_UI_STREAMING_CONTRACT_VERSION);
  assert(runtime.shellAttrs["data-workbench-ui-load-now"].includes("section.workflow"));
  assert(runtime.shellAttrs["data-workbench-ui-prefetch-idle"].includes("runtime.wasm-python"));
  assert.equal(workflowAttrs["data-workbench-ui-render-policy"], "visible");
  assert.equal(modelAttrs["data-workbench-ui-render-policy"], "hidden");
});
