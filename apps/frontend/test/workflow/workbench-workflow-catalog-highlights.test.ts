import test from "node:test";
import assert from "node:assert/strict";

import {
  PINNED_WORKFLOW_IDS,
  deriveWorkflowCatalogHighlights,
} from "../../src/components/workbench/workflow/workbench-workflow-catalog-highlights.ts";

test("pinned workflow ids include peak diagnostics report workflow", () => {
  assert.ok(
    PINNED_WORKFLOW_IDS.includes("workflow.peak-diagnostics-bundle-report-markdown"),
  );
});

test("deriveWorkflowCatalogHighlights marks peak diagnostics chains distinctly", () => {
  const highlights = deriveWorkflowCatalogHighlights({
    id: "workflow.peak-diagnostics-bundle-report-markdown",
    name: "Peak diagnostics bundle report markdown",
    summary: "Peak diagnostics workflow",
    version: "1.0.0",
    capability_tags: ["peak", "diagnostics", "guard", "report", "markdown"],
    graph: {
      nodes: [
        { id: "extract_peak_e", kind: "extract" },
        { id: "extract_peak_t", kind: "extract" },
        { id: "extract_peak_tm", kind: "extract" },
        { id: "bundle", kind: "transform" },
        { id: "guard", kind: "transform" },
        { id: "report", kind: "transform" },
      ],
    },
    local: null,
  } as never);

  assert.equal(highlights[0]?.label, "chain");
  assert.equal(highlights[0]?.value, "peak -> guard -> report");
  assert.ok(
    highlights.some(
      (entry) => entry.label === "stages" && entry.value === "0 solve / 3 transform / 3 extract",
    ),
  );
});
