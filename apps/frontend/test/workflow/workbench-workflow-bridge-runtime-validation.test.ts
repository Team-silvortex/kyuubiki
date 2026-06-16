import test from "node:test";
import assert from "node:assert/strict";

import {
  inspectWorkflowBridgeRuntimePaths,
  validateWorkflowBridgeRuntimeContracts,
} from "../../src/components/workbench/workflow/workbench-workflow-bridge-runtime-validation.ts";

test("validateWorkflowBridgeRuntimeContracts reports missing source and target fields in runtime artifacts", () => {
  const graph = {
    nodes: [
      {
        id: "bridge_1",
        operator_id: "bridge.electrostatic_field_to_heat_quad_2d",
        config: {
          contract: {
            version: "kyuubiki.bridge-contract/v1",
            source: {
              field: "electric_field_magnitude",
              distribution: "element_to_nodes",
              node_index_fields: ["node_i", "node_j", "node_k", "node_l"],
            },
            transform: { scale: 1, reduction: "mean", default_value: 0 },
            target: { field: "heat_load" },
          },
        },
      },
    ],
    edges: [
      {
        id: "edge_1",
        from: { node: "electro_solver", port: "result" },
        to: { node: "bridge_1", port: "electrostatic_result" },
      },
      {
        id: "edge_2",
        from: { node: "bridge_1", port: "heat_model" },
        to: { node: "heat_solver", port: "model" },
      },
    ],
  };
  const result = {
    workflow_id: "wf.bridge-runtime",
    completed_nodes: ["electro_solver", "bridge_1"],
    artifacts: {
      "electro_solver.result": {
        node_id: "electro_solver",
        port_id: "result",
        payload: {
          elements: [{ electric_potential: 5 }],
        },
      },
      "bridge_1.heat_model": {
        node_id: "bridge_1",
        port_id: "heat_model",
        payload: {
          nodes: [{ temperature: 20 }],
        },
      },
    },
  };

  const issues = validateWorkflowBridgeRuntimeContracts(
    graph as never,
    result as never,
  );

  const sourceIssue = issues.find((issue) => issue.id === "bridge:runtime:source-field:bridge_1");
  const targetIssue = issues.find((issue) => issue.id === "bridge:runtime:target-field:bridge_1");

  assert.ok(sourceIssue);
  assert.equal(sourceIssue.upstreamNodeId, "electro_solver");
  assert.equal(sourceIssue.inputEdgeId, "edge_1");
  assert.deepEqual(sourceIssue.downstreamNodeIds, ["heat_solver"]);
  assert.deepEqual(sourceIssue.outputEdgeIds, ["edge_2"]);
  assert.equal(targetIssue?.inputEdgeId, "edge_1");
  assert.deepEqual(targetIssue?.downstreamNodeIds, ["heat_solver"]);
});

test("validateWorkflowBridgeRuntimeContracts accepts runtime artifacts with bridge fields present", () => {
  const graph = {
    nodes: [
      {
        id: "bridge_ok",
        operator_id: "bridge.temperature_field_to_thermo_quad_2d",
        config: {
          contract: {
            version: "kyuubiki.bridge-contract/v1",
            source: {
              field: "temperature",
              distribution: "node_to_node",
              node_index_fields: [],
            },
            transform: { scale: 1, reduction: "copy", default_value: 0 },
            target: { field: "temperature_delta" },
          },
        },
      },
    ],
    edges: [
      {
        id: "edge_ok",
        from: { node: "heat_solver", port: "result" },
        to: { node: "bridge_ok", port: "heat_result" },
      },
    ],
  };
  const result = {
    workflow_id: "wf.bridge-runtime-ok",
    completed_nodes: ["heat_solver", "bridge_ok"],
    artifacts: {
      "heat_solver.result": {
        node_id: "heat_solver",
        port_id: "result",
        payload: {
          nodes: [{ temperature: 100 }, { temperature: 20 }],
        },
      },
      "bridge_ok.thermo_model": {
        node_id: "bridge_ok",
        port_id: "thermo_model",
        payload: {
          nodes: [{ temperature_delta: 80 }, { temperature_delta: 0 }],
        },
      },
    },
  };

  const issues = validateWorkflowBridgeRuntimeContracts(
    graph as never,
    result as never,
  );

  assert.deepEqual(issues, []);
});

test("inspectWorkflowBridgeRuntimePaths returns bridge runtime mapping details", () => {
  const graph = {
    nodes: [
      {
        id: "bridge_ok",
        operator_id: "bridge.temperature_field_to_thermo_quad_2d",
        config: {
          contract: {
            version: "kyuubiki.bridge-contract/v1",
            source: { field: "temperature", distribution: "node_to_node", node_index_fields: [] },
            transform: { scale: 1.5, reduction: "copy", default_value: 0 },
            target: { field: "temperature_delta" },
          },
        },
      },
    ],
    edges: [
      { id: "edge_in", from: { node: "heat_solver", port: "result" }, to: { node: "bridge_ok", port: "heat_result" } },
      { id: "edge_out", from: { node: "bridge_ok", port: "thermo_model" }, to: { node: "thermo_solver", port: "model" } },
    ],
  };
  const result = {
    workflow_id: "wf.bridge-runtime-map",
    completed_nodes: ["heat_solver", "bridge_ok"],
    artifacts: {
      "heat_solver.result": { node_id: "heat_solver", port_id: "result", payload: { nodes: [{ temperature: 100 }] } },
      "bridge_ok.thermo_model": { node_id: "bridge_ok", port_id: "thermo_model", payload: { nodes: [{ temperature_delta: 80 }] } },
    },
  };

  const records = inspectWorkflowBridgeRuntimePaths(graph as never, result as never);

  assert.equal(records.length, 1);
  assert.deepEqual(records[0], {
    nodeId: "bridge_ok",
    operatorId: "bridge.temperature_field_to_thermo_quad_2d",
    upstreamNodeId: "heat_solver",
    downstreamNodeIds: ["thermo_solver"],
    inputArtifactKey: "heat_solver.result",
    outputArtifactKey: "bridge_ok.thermo_model",
    sourceField: "temperature",
    targetField: "temperature_delta",
    reduction: "copy",
    scale: 1.5,
    sourceFieldExposed: true,
    targetFieldExposed: true,
  });
});
