import test from "node:test";
import assert from "node:assert/strict";

import {
  asWorkflowDatasetContract,
  asWorkflowGraphDefinition,
  mergeDatasetContractIntoGraph,
  normalizeImportedWorkflowGraph,
} from "../../src/components/workbench/workflow/workbench-workflow-builder-import.ts";

test("asWorkflowGraphDefinition rejects malformed graph structure at the import boundary", () => {
  const base = {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.import-boundary",
    nodes: [{ id: "solve", kind: "solver" }],
    edges: [],
  };

  assert.ok(asWorkflowGraphDefinition(base));
  assert.equal(asWorkflowGraphDefinition({ ...base, id: "" }), null);
  assert.equal(asWorkflowGraphDefinition({ ...base, nodes: [null] }), null);
  assert.equal(asWorkflowGraphDefinition({ ...base, nodes: [{ id: "solve" }] }), null);
  assert.equal(
    asWorkflowGraphDefinition({ ...base, nodes: [{ id: "solve", kind: "solver", config: [] }] }),
    null,
  );
  assert.equal(
    asWorkflowGraphDefinition({
      ...base,
      nodes: [{ id: "solve", kind: "solver", inputs: [{ id: "in" }] }],
    }),
    null,
  );
  assert.equal(
    asWorkflowGraphDefinition({
      ...base,
      edges: [{ id: "edge", from: { node: "a" }, to: { node: "b", port: "in" }, artifact_type: "artifact/json" }],
    }),
    null,
  );
  assert.equal(
    asWorkflowGraphDefinition({
      ...base,
      dataset_contract: {
        schema_version: "kyuubiki.workflow-dataset/v1",
        id: "dataset.invalid",
        version: "1.0.0",
        values: [null],
      },
    }),
    null,
  );
});

test("asWorkflowDatasetContract validates nested values and metadata", () => {
  const contract = {
    schema_version: "kyuubiki.workflow-dataset/v1",
    id: "dataset.import-boundary",
    version: "1.0.0",
    values: [
      {
        id: "temperature",
        data_class: "field",
        element_type: "float64",
        shape: { axes: [{ id: "nodes", size: 4 }] },
      },
    ],
    metadata: { owner: "test" },
  };

  assert.ok(asWorkflowDatasetContract(contract));
  assert.equal(
    asWorkflowDatasetContract({
      ...contract,
      values: [{ ...contract.values[0], shape: { axes: [{ id: "nodes", size: -1 }] } }],
    }),
    null,
  );
  assert.equal(asWorkflowDatasetContract({ ...contract, metadata: { owner: 4 } }), null);
});

test("mergeDatasetContractIntoGraph does not retain mutable contract references", () => {
  const graph = {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.contract-ownership",
    nodes: [],
  };
  const contract = {
    schema_version: "kyuubiki.workflow-dataset/v1",
    id: "dataset.contract-ownership",
    version: "1.0.0",
    values: [
      {
        id: "temperature",
        data_class: "field",
        element_type: "float64",
        shape: { axes: [{ id: "nodes", size: 4 }] },
      },
    ],
    metadata: { owner: "original" },
  };

  const merged = mergeDatasetContractIntoGraph(graph, contract);
  assert.ok(merged?.dataset_contract);
  contract.values[0].shape.axes[0].size = 8;
  contract.metadata.owner = "mutated";

  assert.equal(merged.dataset_contract.values[0]?.shape.axes?.[0]?.size, 4);
  assert.equal(merged.dataset_contract.metadata?.owner, "original");
});

test("normalizeImportedWorkflowGraph keeps bridge normalization diagnostics and clones the graph", () => {
  const graph = {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.bridge-import",
    name: "workflow bridge import",
    version: "2.0.0",
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
