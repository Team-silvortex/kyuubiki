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

test("validateImportedWorkflowPackage rejects manifests that omit graph contracts", () => {
  const importedPackage = buildImportedWorkflowPackage();
  importedPackage.runtime_manifest.required_operator_ids.pop();
  importedPackage.runtime_manifest.operator_fetch_plan = [];
  importedPackage.contract_manifest.dataset_value_ids.pop();
  importedPackage.contract_manifest.output_contracts = [];

  const diagnostics = validateImportedWorkflowPackage(
    importedPackage as never,
    buildImportedWorkflowGraph() as never,
  );

  assert.ok(
    diagnostics.some((entry) => entry.message.includes("Undeclared workflow operator")),
  );
  assert.ok(
    diagnostics.some((entry) => entry.message.includes("Missing operator fetch plan")),
  );
  assert.ok(
    diagnostics.some((entry) => entry.message.includes("Undeclared dataset value")),
  );
  assert.ok(
    diagnostics.some((entry) => entry.message.includes("Missing output contract declaration")),
  );
});

test("validateImportedWorkflowPackage rejects workflow identity drift", () => {
  const importedPackage = buildImportedWorkflowPackage();
  importedPackage.workflow.id = "workflow.other";

  const diagnostics = validateImportedWorkflowPackage(
    importedPackage as never,
    buildImportedWorkflowGraph() as never,
  );

  assert.ok(diagnostics.some((entry) => entry.message.includes("Workflow id mismatch")));
});
