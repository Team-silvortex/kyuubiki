import test from "node:test";
import assert from "node:assert/strict";

import {
  DIAGNOSTICS_BUNDLE_GUARD_REPORT_TEMPLATE_CHAIN,
  PEAK_DIAGNOSTICS_BUNDLE_REPORT_TEMPLATE_CHAIN,
} from "../../src/components/workbench/workflow/workbench-workflow-template-chain-diagnostics-preset.ts";

test("diagnostics bundle chain stays registered with expected operators and guard defaults", () => {
  const chain = DIAGNOSTICS_BUNDLE_GUARD_REPORT_TEMPLATE_CHAIN;
  assert.equal(chain.label, "diagnostics -> bundle -> guard -> report");
  assert.equal(chain.templates.length, 7);
  assert.deepEqual(
    chain.templates.map((template) => template.operatorId ?? template.kind),
    [
      "extract.electrostatic_result_diagnostics",
      "extract.thermal_result_diagnostics",
      "extract.thermo_result_diagnostics",
      "transform.compose_diagnostics_bundle",
      "transform.evaluate_diagnostics_bundle_guard",
      "transform.compose_diagnostics_report_payload",
      "export.diagnostics_bundle_markdown",
    ],
  );
  assert.deepEqual(chain.connections?.map((connection) => connection.toPort), [
    "electrostatic",
    "thermal",
    "thermo",
    "bundle",
    "bundle",
    "guard",
    "bundle",
  ]);

  const guardTemplate = chain.templates[4];
  assert.ok(guardTemplate?.config);
  assert.deepEqual(guardTemplate.config?.rules, [
    {
      source: "thermal",
      field: "thermal_temperature_max",
      threshold: 120,
      severity: "warn",
      label: "thermal temperature",
    },
    {
      source: "thermo",
      field: "thermo_peak_stress",
      comparison: "gt",
      threshold: 180,
      severity: "block",
      label: "stress ceiling",
    },
    {
      source: "electrostatic",
      field: "electrostatic_field_peak_magnitude",
      comparison: "gt",
      threshold: 9,
      severity: "warn",
      label: "field ceiling",
    },
  ]);
});

test("peak diagnostics chain stays registered with expected operators and guard defaults", () => {
  const chain = PEAK_DIAGNOSTICS_BUNDLE_REPORT_TEMPLATE_CHAIN;
  assert.equal(chain.label, "peak extract -> bundle -> report");
  assert.equal(chain.templates.length, 7);
  assert.deepEqual(
    chain.templates.map((template) => template.operatorId ?? template.kind),
    [
      "extract.electrostatic_peak_field",
      "extract.heat_peak_flux",
      "extract.thermo_peak_response",
      "transform.compose_diagnostics_bundle",
      "transform.evaluate_diagnostics_bundle_guard",
      "transform.compose_diagnostics_report_payload",
      "export.diagnostics_bundle_markdown",
    ],
  );
});
