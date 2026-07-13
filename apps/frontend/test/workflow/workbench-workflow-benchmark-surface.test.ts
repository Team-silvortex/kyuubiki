import test from "node:test";
import assert from "node:assert/strict";

import {
  WORKBENCH_WORKFLOW_BENCHMARK_SURFACE_VERSION,
  workbenchWorkflowBenchmarkSurface,
} from "../../src/components/workbench/workflow/workbench-workflow-benchmark-surface.ts";
import {
  WORKFLOW_CATALOG_RENDER_LIMIT,
  WORKFLOW_RUN_RENDER_LIMIT,
} from "../../src/components/workbench/workflow/workbench-workflow-render-budget.ts";

test("workbench workflow benchmark surface exposes render and perf lanes", () => {
  const surface = workbenchWorkflowBenchmarkSurface();
  const laneIds = surface.lanes.map((lane) => lane.id);

  assert.equal(surface.schemaVersion, WORKBENCH_WORKFLOW_BENCHMARK_SURFACE_VERSION);
  assert.equal(surface.owner, "workbench-shell");
  assert.equal(surface.renderLimits.catalogEntries, WORKFLOW_CATALOG_RENDER_LIMIT);
  assert.equal(surface.renderLimits.runRecords, WORKFLOW_RUN_RENDER_LIMIT);
  assert.deepEqual(new Set(laneIds).size, laneIds.length);
  assert(laneIds.includes("workflow_surface_switch"));
  assert(laneIds.includes("workflow_catalog_render"));
  assert(surface.lanes.every((lane) => lane.budgetMs > 0));
  assert(
    surface.lanes.some((lane) =>
      lane.evidence.includes("measureWorkflowTraceCardReady"),
    ),
  );
});
