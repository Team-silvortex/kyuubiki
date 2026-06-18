import test from "node:test";
import assert from "node:assert/strict";

import {
  describeWorkflowNodeTemplatePresetSearchMatches,
  scoreWorkflowNodeTemplatePresetSearch,
  suggestWorkflowNodeTemplatePresets,
} from "../../src/components/workbench/workflow/workbench-workflow-operator-search-match.ts";

const PRESETS = [
  {
    id: "thermal_bridge_export",
    label: "Thermal Bridge Export",
    kind: "extract",
    operatorId: "thermal.bridge.export",
    inputs: [{ id: "input", artifact_type: "artifact/thermal-model", description: "thermal model" }],
    outputs: [{ id: "output", artifact_type: "artifact/thermal-export", description: "thermal export" }],
  },
  {
    id: "thermal_summary_bundle",
    label: "Thermal Summary Bundle",
    kind: "export",
    operatorId: "thermal.summary.bundle",
    inputs: [{ id: "input", artifact_type: "artifact/thermal-summary", description: "thermal summary" }],
    outputs: [{ id: "output", artifact_type: "artifact/summary-bundle", description: "summary bundle" }],
  },
] as any;

const DESCRIPTORS = new Map([
  [
    "thermal.bridge.export",
    {
      summary: "Bridge thermal diagnostics into an export payload.",
      family: "bridge",
      domain: "thermal",
      capability_tags: ["bridge", "export", "diagnostics"],
      validation: { baseline_status: "verified" },
    },
  ],
  [
    "thermal.summary.bundle",
    {
      summary: "Package a thermal summary bundle.",
      family: "summary",
      domain: "thermal",
      capability_tags: ["summary", "bundle"],
      validation: { baseline_status: "partial" },
    },
  ],
]) as any;

test("suggestWorkflowNodeTemplatePresets ranks the closest operator first", () => {
  const suggestions = suggestWorkflowNodeTemplatePresets(
    PRESETS,
    DESCRIPTORS,
    "bridge thermal export",
  );
  assert.equal(suggestions[0]?.preset.operatorId, "thermal.bridge.export");
  assert.ok((suggestions[0]?.score ?? 0) > (suggestions[1]?.score ?? 0));
});

test("scoreWorkflowNodeTemplatePresetSearch supports underscore and hyphen tokenization", () => {
  const score = scoreWorkflowNodeTemplatePresetSearch(
    PRESETS[0],
    DESCRIPTORS.get("thermal.bridge.export"),
    "bridge_thermal-export",
  );
  assert.ok((score ?? 0) > 0);
});

test("describeWorkflowNodeTemplatePresetSearchMatches reports useful reasons", () => {
  const matches = describeWorkflowNodeTemplatePresetSearchMatches(
    PRESETS[0],
    DESCRIPTORS.get("thermal.bridge.export"),
    "bridge",
  );
  assert.ok(matches.some((entry) => entry.startsWith("label:")));
  assert.ok(matches.some((entry) => entry.startsWith("operator:") || entry.startsWith("family:") || entry.startsWith("capability:")));
});
