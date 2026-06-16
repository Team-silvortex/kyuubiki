import test from "node:test";
import assert from "node:assert/strict";

import { validateCatalogArtifacts } from "../../src/components/workbench/workflow/workbench-workflow-validation-catalog.ts";

test("validateCatalogArtifacts reports missing artifact exposure and suggests a fix", () => {
  const graph = {
    nodes: [
      {
        id: "export_node",
        outputs: [{ id: "json", artifact_type: "artifact/json" }],
      },
    ],
  };
  const issues = validateCatalogArtifacts(
    graph as never,
    [{ node_id: "export_node", artifact_type: "artifact/csv" }] as never,
    "output",
  );

  assert.equal(issues.length, 1);
  assert.equal(issues[0]?.fix?.kind, "set_catalog_artifact_type");
});
