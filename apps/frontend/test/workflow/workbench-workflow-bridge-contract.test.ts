import test from "node:test";
import assert from "node:assert/strict";

import { resolveBridgeContractForOperator } from "../../src/lib/workbench/workflow-bridge-contract.ts";
import { listBridgeContractNormalizationAdjustments } from "../../src/lib/workbench/workflow-bridge-contract-support.ts";

test("sparse catalog bridge contracts inherit operator defaults before rendering", () => {
  const contract = resolveBridgeContractForOperator(
    "bridge.temperature_field_to_thermo_quad_2d",
    {
      contract: {
        version: "kyuubiki.bridge-contract/v1",
        source: { field: "temperature" },
        transform: { scale: 1, default_value: 0 },
        target: { field: "temperature_delta" },
      },
    },
  );

  assert.deepEqual(contract?.source.node_index_fields, []);
  assert.equal(contract?.source.distribution, "node_to_node");
  assert.equal(contract?.transform.reduction, "copy");
  assert.doesNotThrow(() => listBridgeContractNormalizationAdjustments(contract!, {
    source: {
      fields: ["temperature"],
      distributions: { node_to_node: ["temperature"] },
      node_index_fields: [],
    },
    transform: { reductions: ["copy"] },
    target: { fields: ["temperature_delta"] },
  }));
});
