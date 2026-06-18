import test from "node:test";
import assert from "node:assert/strict";

import {
  describeWorkflowCatalogEntrySearchMatches,
  scoreWorkflowCatalogEntrySearch,
  suggestWorkflowCatalogEntries,
} from "../../src/components/workbench/workflow/workbench-workflow-catalog-search.ts";

const WORKFLOWS = [
  {
    id: "workflow.bridge-thermal-export",
    name: "Bridge Thermal Export",
    summary: "Bridge thermal diagnostics into an export artifact.",
    version: "v1",
    domains: ["thermal"],
    capability_tags: ["bridge", "export", "diagnostics"],
    entry_inputs: [{ artifact_type: "artifact/thermal-model" }],
    output_artifacts: [{ artifact_type: "artifact/thermal-export" }],
    runtime_manifest: { required_operator_ids: ["thermal.bridge.export"] },
    graph: { nodes: [{ operator_id: "thermal.bridge.export" }] },
    local: null,
  },
  {
    id: "workflow.summary-bundle",
    name: "Summary Bundle",
    summary: "Thermal bridge export bundle.",
    version: "v1",
    domains: ["thermal"],
    capability_tags: ["export", "bundle"],
    entry_inputs: [{ artifact_type: "artifact/heat-model" }],
    output_artifacts: [{ artifact_type: "artifact/thermal-export" }],
    runtime_manifest: { required_operator_ids: ["thermal.export.bundle"] },
    graph: { nodes: [{ operator_id: "thermal.export.bundle" }] },
    local: null,
  },
  {
    id: "workflow.electrostatic-review",
    name: "Electrostatic Review",
    summary: "Review electrostatic diagnostics.",
    version: "v1",
    domains: ["electromagnetic"],
    capability_tags: ["electrostatic", "review"],
    entry_inputs: [{ artifact_type: "artifact/electrostatic-model" }],
    output_artifacts: [{ artifact_type: "artifact/review-note" }],
    runtime_manifest: { required_operator_ids: ["electrostatic.review"] },
    graph: { nodes: [{ operator_id: "electrostatic.review" }] },
    local: { tags: ["review_ready"] },
  },
] as any;

test("suggestWorkflowCatalogEntries ranks the closest workflow first", () => {
  const suggestions = suggestWorkflowCatalogEntries(WORKFLOWS, "bridge thermal export");
  assert.equal(suggestions[0]?.workflow.id, "workflow.bridge-thermal-export");
  assert.ok((suggestions[0]?.score ?? 0) > (suggestions[1]?.score ?? 0));
});

test("scoreWorkflowCatalogEntrySearch supports underscore and hyphen tokenization", () => {
  const score = scoreWorkflowCatalogEntrySearch(WORKFLOWS[0], "bridge_thermal-export");
  assert.ok((score ?? 0) > 0);
});

test("describeWorkflowCatalogEntrySearchMatches reports name and id reasons", () => {
  const matches = describeWorkflowCatalogEntrySearchMatches(WORKFLOWS[2], "review");
  assert.ok(matches.some((entry) => entry.startsWith("name:") || entry.startsWith("id:")));
});

test("describeWorkflowCatalogEntrySearchMatches reports local tag reasons", () => {
  const matches = describeWorkflowCatalogEntrySearchMatches(WORKFLOWS[2], "review_ready");
  assert.ok(matches.some((entry) => entry.startsWith("tag:")));
});
