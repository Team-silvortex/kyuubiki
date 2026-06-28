import test from "node:test";
import assert from "node:assert/strict";

import {
  WORKBENCH_UI_STREAMING_CONTRACT_VERSION,
  listWorkbenchUiChunkContracts,
  resolveWorkbenchUiStreamingState,
} from "../../src/components/workbench/workbench-ui-streaming.ts";

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
