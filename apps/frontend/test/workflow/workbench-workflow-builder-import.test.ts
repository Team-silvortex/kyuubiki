import test from "node:test";
import assert from "node:assert/strict";

import { normalizeImportedWorkflowGraph } from "../../src/components/workbench/workflow/workbench-workflow-builder-import.ts";

test("normalizeImportedWorkflowGraph keeps bridge normalization diagnostics and clones the graph", () => {
  const graph = {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.bridge-import",
    name: "workflow bridge import",
    version: "1.14.0",
    nodes: [
      {
        id: "bridge_1",
        kind: "transform",
        operator_id: "bridge.temperature_field_to_thermo_quad_2d",
        config: {
          contract_normalization: [
            {
              field: "target.field",
              previous: "temp_peak",
              next: "thermal_temperature_max",
            },
          ],
        },
      },
    ],
    edges: [],
  };

  const normalized = normalizeImportedWorkflowGraph(graph as never, []);

  assert.ok(normalized.graph);
  assert.notEqual(normalized.graph, graph);
  assert.deepEqual(normalized.autoReconnectEdgeIds, []);
  assert.equal(normalized.diagnostics.length, 1);
  assert.match(
    normalized.diagnostics[0]?.message ?? "",
    /Bridge contract normalized at bridge_1/,
  );
});
