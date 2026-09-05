import test from "node:test";
import assert from "node:assert/strict";

import type { WorkflowGraphDefinition } from "../../src/lib/api/workflow-types.ts";
import {
  asWorkflowDraftBundle,
  buildWorkflowDraftBundle,
} from "../../src/components/workbench/workflow/workbench-workflow-draft-bundle.ts";

function graph(): WorkflowGraphDefinition {
  return {
    schema_version: "kyuubiki.workflow-graph/v1",
    id: "workflow.bundle",
    nodes: [{ id: "solve", kind: "solve", config: { gain: 1 } }],
    edges: [],
  };
}

test("workflow draft bundle captures independent export state", () => {
  const sourceGraph = graph();
  const inputArtifactTexts = { model: "{\"gain\":1}" };
  const templateChainPreferences = {
    favoriteChainIds: ["chain.a"],
    favoriteChainAliases: { "chain.a": "Primary" },
  };
  const bundle = buildWorkflowDraftBundle({
    graph: sourceGraph,
    inputArtifactTexts,
    templateChainPreferences,
  });

  sourceGraph.nodes[0]!.config = { gain: 99 };
  inputArtifactTexts.model = "mutated";
  templateChainPreferences.favoriteChainIds.push("chain.b");

  assert.deepEqual(bundle.graph.nodes[0]?.config, { gain: 1 });
  assert.equal(bundle.input_artifact_texts?.model, "{\"gain\":1}");
  assert.deepEqual(bundle.template_chain_preferences?.favoriteChainIds, ["chain.a"]);
});

test("workflow draft bundle rejects fabricated provenance and malformed inputs", () => {
  const bundle = buildWorkflowDraftBundle({ graph: graph(), inputArtifactTexts: { model: "{}" } });

  assert.equal(asWorkflowDraftBundle({ ...bundle, exported_at: "not-a-date" }), null);
  assert.equal(asWorkflowDraftBundle({ ...bundle, exported_at: undefined }), null);
  assert.equal(
    asWorkflowDraftBundle({ ...bundle, input_artifact_texts: { model: { nested: true } } }),
    null,
  );
  assert.equal(
    asWorkflowDraftBundle({
      ...bundle,
      template_chain_preferences: {
        favoriteChainIds: ["chain.a", 4],
        favoriteChainAliases: { "chain.a": "Primary" },
      },
    }),
    null,
  );
});

test("parsed workflow draft bundles are detached from the input document", () => {
  const document = buildWorkflowDraftBundle({ graph: graph(), inputArtifactTexts: { model: "{}" } });
  const parsed = asWorkflowDraftBundle(document);
  assert.ok(parsed);

  document.graph.nodes[0]!.config = { gain: 7 };
  document.input_artifact_texts!.model = "mutated";

  assert.deepEqual(parsed.graph.nodes[0]?.config, { gain: 1 });
  assert.equal(parsed.input_artifact_texts?.model, "{}");
});
