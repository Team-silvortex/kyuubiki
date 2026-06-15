import test from "node:test";
import assert from "node:assert/strict";

import {
  applyWorkflowDiagnosticsScenario,
  resolveWorkflowDiagnosticsGuardRules,
  resolveWorkflowDiagnosticsScenarioPreview,
  resolveWorkflowDiagnosticsSummaryInfo,
} from "../../src/components/workbench/workflow/workbench-workflow-diagnostics-input-helper.ts";
import {
  buildDiagnosticsGuardNode,
  buildDiagnosticsSummaryPayload,
} from "../support/workflow-diagnostics-fixtures.ts";

test("resolveWorkflowDiagnosticsSummaryInfo reads unified diagnostics contract payloads", () => {
  const summary = resolveWorkflowDiagnosticsSummaryInfo(
    buildDiagnosticsSummaryPayload("thermal"),
  );

  assert.ok(summary);
  assert.equal(summary.domain, "thermal");
  assert.equal(summary.subject, "thermal_result");
  assert.equal(summary.prefix, "thermal");
  assert.deepEqual(summary.metricGroups, ["temperature", "flux"]);
  assert.equal(summary.numericMetrics[0]?.[0], "thermal_temperature_max");
});

test("resolveWorkflowDiagnosticsGuardRules extracts typed guard rules from workflow nodes", () => {
  const rules = resolveWorkflowDiagnosticsGuardRules([
    buildDiagnosticsGuardNode() as never,
  ]);

  assert.equal(rules.length, 3);
  assert.deepEqual(rules.map((rule) => rule.source), [
    "thermal",
    "thermo",
    "electrostatic",
  ]);
});

test("resolveWorkflowDiagnosticsScenarioPreview follows current guard thresholds", () => {
  const rules = resolveWorkflowDiagnosticsGuardRules([
    buildDiagnosticsGuardNode() as never,
  ]);

  const preview = resolveWorkflowDiagnosticsScenarioPreview(
    "thermo_block",
    rules,
  );

  assert.equal(preview.domain, "thermo");
  assert.equal(preview.field, "thermo_peak_stress");
  assert.equal(preview.threshold, 180);
  assert.equal(preview.expectedSeverity, "block");
  assert.ok(preview.targetValue > 180);
});

test("applyWorkflowDiagnosticsScenario pushes payload values past active thresholds", () => {
  const rules = resolveWorkflowDiagnosticsGuardRules([
    buildDiagnosticsGuardNode() as never,
  ]);
  const payload = buildDiagnosticsSummaryPayload("electrostatic");

  const nextPayload = applyWorkflowDiagnosticsScenario(
    payload,
    "electrostatic_warn",
    rules,
  ) as {
    summary: { electrostatic_field_peak_magnitude: number };
  };

  assert.ok(
    nextPayload.summary.electrostatic_field_peak_magnitude > 9,
    "scenario value should move beyond the configured electrostatic threshold",
  );
});
