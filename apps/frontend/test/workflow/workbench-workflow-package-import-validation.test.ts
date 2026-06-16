import test from "node:test";
import assert from "node:assert/strict";

import { validateImportedWorkflowPackage } from "../../src/components/workbench/workflow/workbench-workflow-package-import-validation.ts";
import {
  buildImportedWorkflowGraph,
  buildImportedWorkflowPackage,
} from "../support/workflow-package-fixtures.ts";

test("validateImportedWorkflowPackage accepts aligned package and graph manifests", () => {
  const diagnostics = validateImportedWorkflowPackage(
    buildImportedWorkflowPackage() as never,
    buildImportedWorkflowGraph() as never,
  );

  assert.deepEqual(diagnostics, []);
});

test("validateImportedWorkflowPackage reports missing operators and dataset ids", () => {
  const importedPackage = buildImportedWorkflowPackage();
  importedPackage.runtime_manifest.required_operator_ids.push("export.diagnostics_bundle_markdown");
  importedPackage.contract_manifest.dataset_value_ids.push("guard_result");

  const diagnostics = validateImportedWorkflowPackage(
    importedPackage as never,
    buildImportedWorkflowGraph() as never,
  );

  assert.ok(
    diagnostics.some((entry) => entry.message.includes("Missing required operator: export.diagnostics_bundle_markdown")),
  );
  assert.ok(
    diagnostics.some((entry) => entry.message.includes("Missing dataset value: guard_result")),
  );
});
