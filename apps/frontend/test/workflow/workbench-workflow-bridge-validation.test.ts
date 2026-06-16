import test from "node:test";
import assert from "node:assert/strict";

import { validateBridgeNodes } from "../../src/components/workbench/workflow/workbench-workflow-bridge-validation.ts";

test("validateBridgeNodes reports bridge semantic mismatches across ports, edges, and seed model shape", () => {
  const graph = {
    dataset_contract: {
      values: [
        { id: "heat_result", semantic_type: "result/heat_plane_quad_2d" },
        { id: "thermo_model", semantic_type: "study_model/thermal_plane_quad_2d" },
        { id: "wrong_input", semantic_type: "result/heat_plane_quad_2d" },
        { id: "wrong_output", semantic_type: "study_model/thermal_plane_quad_2d" },
      ],
    },
    nodes: [
      {
        id: "bridge_1",
        operator_id: "bridge.electrostatic_field_to_heat_quad_2d",
        config: {
          seed_model: {
            nodes: [{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }],
            elements: [{ node_i: 0, node_j: 1, node_k: 2 }],
          },
          contract: {
            version: "kyuubiki.bridge-contract/v1",
            source: {
              field: "temperature",
              distribution: "element_to_nodes",
              node_index_fields: ["node_i", "node_j", "node_k", "node_l"],
            },
            transform: { scale: 1, reduction: "mean", default_value: 0 },
            target: { field: "temperature_delta" },
          },
        },
        inputs: [
          {
            id: "electrostatic_result",
            artifact_type: "result/electrostatic_plane_quad_2d",
            dataset_value: "wrong_input",
          },
        ],
        outputs: [
          {
            id: "heat_model",
            artifact_type: "study_model/heat_plane_quad_2d",
            dataset_value: "wrong_output",
          },
        ],
      },
    ],
    edges: [
      {
        id: "edge_in",
        from: { node: "upstream", port: "result" },
        to: { node: "bridge_1", port: "electrostatic_result" },
        artifact_type: "result/electrostatic_plane_quad_2d",
        dataset_value: "wrong_input",
      },
      {
        id: "edge_out",
        from: { node: "bridge_1", port: "heat_model" },
        to: { node: "downstream", port: "model" },
        artifact_type: "study_model/heat_plane_quad_2d",
        dataset_value: "wrong_output",
      },
    ],
  };

  const issues = validateBridgeNodes(graph as never, [
    {
      id: "bridge.electrostatic_field_to_heat_quad_2d",
      contract_support: {
        source: {
          distributions: {
            element_to_nodes: ["electric_field_magnitude"],
          },
          node_index_fields: ["node_i", "node_j", "node_k", "node_l"],
        },
        transform: {
          reductions: ["mean"],
          default_reduction_by_distribution: { element_to_nodes: "mean" },
        },
        target: { fields: ["heat_load"], default_field: "heat_load" },
      },
    },
  ] as never);

  assert.ok(issues.some((issue) => issue.id === "bridge:semantic:input:bridge_1"));
  assert.ok(issues.some((issue) => issue.id === "bridge:semantic:output:bridge_1"));
  assert.ok(issues.some((issue) => issue.id === "bridge:edge-semantic:input:bridge_1:edge_in"));
  assert.ok(issues.some((issue) => issue.id === "bridge:edge-semantic:output:bridge_1:edge_out"));
  assert.ok(issues.some((issue) => issue.id === "bridge:seed-shape:nodes:bridge_1"));
  assert.ok(issues.some((issue) => issue.id === "bridge:seed-shape:elements:bridge_1"));
  assert.ok(issues.some((issue) => issue.id === "bridge:source-field-semantic:bridge_1"));
  assert.ok(issues.some((issue) => issue.id === "bridge:target-field-seed:bridge_1"));
});

test("validateBridgeNodes accepts aligned bridge semantic contracts", () => {
  const graph = {
    dataset_contract: {
      values: [
        {
          id: "electrostatic_plane_quad_result",
          semantic_type: "result/electrostatic_plane_quad_2d",
        },
        { id: "heat_model", semantic_type: "study_model/heat_plane_quad_2d" },
      ],
    },
    nodes: [
      {
        id: "bridge_ok",
        operator_id: "bridge.electrostatic_field_to_heat_quad_2d",
        config: {
          seed_model: {
            nodes: [{ x: 0, y: 0 }, { x: 1, y: 0 }, { x: 1, y: 1 }, { x: 0, y: 1 }],
            elements: [{ node_i: 0, node_j: 1, node_k: 2, node_l: 3 }],
          },
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
        inputs: [
          {
            id: "electrostatic_result",
            artifact_type: "result/electrostatic_plane_quad_2d",
            dataset_value: "electrostatic_plane_quad_result",
          },
        ],
        outputs: [
          {
            id: "heat_model",
            artifact_type: "study_model/heat_plane_quad_2d",
            dataset_value: "heat_model",
          },
        ],
      },
    ],
    edges: [
      {
        id: "edge_ok",
        from: { node: "upstream", port: "result" },
        to: { node: "bridge_ok", port: "electrostatic_result" },
        artifact_type: "result/electrostatic_plane_quad_2d",
        dataset_value: "electrostatic_plane_quad_result",
      },
    ],
  };

  const issues = validateBridgeNodes(graph as never, [
    {
      id: "bridge.electrostatic_field_to_heat_quad_2d",
      contract_support: {
        source: {
          distributions: {
            element_to_nodes: ["electric_field_magnitude"],
          },
          node_index_fields: ["node_i", "node_j", "node_k", "node_l"],
        },
        transform: {
          reductions: ["mean"],
          default_reduction_by_distribution: { element_to_nodes: "mean" },
        },
        target: { fields: ["heat_load"], default_field: "heat_load" },
      },
    },
  ] as never);

  assert.equal(
    issues.filter((issue) => issue.id.startsWith("bridge:semantic:")).length,
    0,
  );
  assert.equal(
    issues.filter((issue) => issue.id.startsWith("bridge:edge-semantic:")).length,
    0,
  );
  assert.equal(
    issues.filter((issue) => issue.id.startsWith("bridge:seed-shape:")).length,
    0,
  );
});
