import test from "node:test";
import assert from "node:assert/strict";

import type { WorkflowCatalogEntry, WorkflowGraphDefinition } from "../../src/lib/api/workflow-types.ts";
import {
  asWorkflowPackage,
  buildWorkflowPackage,
  buildWorkflowPackageContractManifest,
} from "../../src/components/workbench/workflow/workbench-workflow-package.ts";

function buildPackageFixture() {
  const graph: WorkflowGraphDefinition = {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.package-boundary",
    nodes: [
      {
        id: "solve",
        kind: "solve",
        operator_id: "solve.heat_plane_quad_2d",
        outputs: [{ id: "result", artifact_type: "artifact/result", dataset_value: "result" }],
      },
    ],
    edges: [],
    output_artifacts: [{ node_id: "solve", artifact_type: "artifact/result", description: "result" }],
    dataset_contract: {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: "package-boundary.dataset",
      version: "1.0.0",
      values: [{ id: "result", data_class: "result", element_type: "json_object", shape: {} }],
    },
  };
  const workflow: WorkflowCatalogEntry = {
    id: graph.id,
    name: "Package boundary",
    version: "1.0.0",
    summary: "Package parser fixture",
    graph,
    entry_inputs: [],
    output_artifacts: graph.output_artifacts ?? [],
  };
  return buildWorkflowPackage({
    workflow,
    graph,
    inputArtifactTexts: { model: "{}" },
    templateChainPreferences: {
      favoriteChainIds: ["chain.a"],
      favoriteChainAliases: { "chain.a": "Primary" },
    },
  });
}

test("package contracts bind entry and output artifacts to ports in the correct direction", () => {
  const graph: WorkflowGraphDefinition = {
    schema_version: "kyuubiki.workflow/v1",
    id: "workflow.directional-contract",
    nodes: [
      {
        id: "transform",
        kind: "transform",
        inputs: [{ id: "in", artifact_type: "artifact/json", dataset_value: "input_value" }],
        outputs: [{ id: "out", artifact_type: "artifact/json", dataset_value: "output_value" }],
      },
    ],
    edges: [],
    entry_inputs: [{ node_id: "transform", artifact_type: "artifact/json", description: "input" }],
    output_artifacts: [{ node_id: "transform", artifact_type: "artifact/json", description: "output" }],
    dataset_contract: {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: "directional.dataset",
      version: "1",
      values: [
        { id: "input_value", data_class: "field", element_type: "scalar", shape: {} },
        { id: "output_value", data_class: "field", element_type: "scalar", shape: {} },
      ],
    },
  };

  const manifest = buildWorkflowPackageContractManifest(graph);

  assert.equal(manifest.entry_contracts[0]?.dataset_value, "input_value");
  assert.equal(manifest.output_contracts[0]?.dataset_value, "output_value");
});

test("workflow package parser rejects missing or fabricated required manifests", () => {
  const packageValue = buildPackageFixture();
  assert.ok(asWorkflowPackage(packageValue));

  assert.equal(asWorkflowPackage({ ...packageValue, exported_at: "not-a-date" }), null);
  assert.equal(asWorkflowPackage({ ...packageValue, search_index: undefined }), null);
  assert.equal(asWorkflowPackage({ ...packageValue, contract_manifest: undefined }), null);
  assert.equal(asWorkflowPackage({ ...packageValue, runtime_manifest: undefined }), null);
  assert.equal(
    asWorkflowPackage({
      ...packageValue,
      runtime_manifest: {
        ...packageValue.runtime_manifest,
        dispatch_policy: {
          ...packageValue.runtime_manifest.dispatch_policy,
          agent_library_replication: "allowed",
        },
      },
    }),
    null,
  );
});

test("workflow package parser rejects malformed required entries instead of dropping them", () => {
  const packageValue = buildPackageFixture();

  assert.equal(
    asWorkflowPackage({ ...packageValue, tags: ["thermal", 4] }),
    null,
  );
  assert.equal(
    asWorkflowPackage({
      ...packageValue,
      runtime_manifest: {
        ...packageValue.runtime_manifest,
        operator_fetch_plan: [...packageValue.runtime_manifest.operator_fetch_plan, { operator_id: "broken" }],
      },
    }),
    null,
  );
  assert.equal(
    asWorkflowPackage({
      ...packageValue,
      workflow: { ...packageValue.workflow, input_artifact_texts: { model: { invalid: true } } },
    }),
    null,
  );
});

test("workflow package build and parse results own their nested state", () => {
  const packageValue = buildPackageFixture();
  const parsed = asWorkflowPackage(packageValue);
  assert.ok(parsed);

  packageValue.workflow.graph.nodes[0]!.config = { mutated: true };
  packageValue.workflow.input_artifact_texts!.model = "mutated";

  assert.equal(parsed.workflow.graph.nodes[0]?.config, undefined);
  assert.equal(parsed.workflow.input_artifact_texts?.model, "{}");
});
