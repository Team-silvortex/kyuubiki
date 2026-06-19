"use client";

import type { WorkflowNodeTemplateSelection } from "@/components/workbench/workflow/workbench-workflow-node-templates";

export type WorkflowTemplateChainPresetConnection = {
  from: number;
  to: number;
  fromPort?: string;
  toPort?: string;
};

export type WorkflowTemplateChainPreset = {
  id: string;
  label: string;
  templates: WorkflowNodeTemplateSelection[];
  connections?: WorkflowTemplateChainPresetConnection[];
  summary?: string;
  version?: string;
  tags?: string[];
  updatedAt?: string;
  source: "built-in" | "imported";
};

export const DIAGNOSTICS_BUNDLE_GUARD_REPORT_TEMPLATE_CHAIN: WorkflowTemplateChainPreset =
  {
    id: "diagnostics_bundle_guard_report",
    label: "diagnostics -> bundle -> guard -> report",
    source: "built-in",
    summary:
      "Bundle electrostatic, thermal, and thermo diagnostics, evaluate a unified guard, and export a markdown report.",
    tags: [
      "diagnostics",
      "bundle",
      "guard",
      "report",
      "markdown",
      "headless",
    ],
    templates: [
      {
        kind: "extract",
        operatorId: "extract.electrostatic_result_diagnostics",
      },
      {
        kind: "extract",
        operatorId: "extract.thermal_result_diagnostics",
      },
      {
        kind: "extract",
        operatorId: "extract.thermo_result_diagnostics",
      },
      {
        kind: "transform",
        operatorId: "transform.compose_diagnostics_bundle",
      },
      {
        kind: "transform",
        operatorId: "transform.evaluate_diagnostics_bundle_guard",
        config: {
          rules: [
            {
              source: "thermal",
              field: "thermal_temperature_max",
              threshold: 120.0,
              severity: "warn",
              label: "thermal temperature",
            },
            {
              source: "thermo",
              field: "thermo_peak_stress",
              comparison: "gt",
              threshold: 180.0,
              severity: "block",
              label: "stress ceiling",
            },
            {
              source: "electrostatic",
              field: "electrostatic_field_peak_magnitude",
              comparison: "gt",
              threshold: 9.0,
              severity: "warn",
              label: "field ceiling",
            },
          ],
        },
      },
      {
        kind: "transform",
        operatorId: "transform.compose_diagnostics_report_payload",
      },
      {
        kind: "export",
        operatorId: "export.diagnostics_bundle_markdown",
        config: { title: "Diagnostics Bundle Report" },
      },
    ],
    connections: [
      { from: 0, to: 3, toPort: "electrostatic" },
      { from: 1, to: 3, toPort: "thermal" },
      { from: 2, to: 3, toPort: "thermo" },
      { from: 3, to: 4, toPort: "bundle" },
      { from: 3, to: 5, toPort: "bundle" },
      { from: 4, to: 5, toPort: "guard" },
      { from: 5, to: 6, toPort: "bundle" },
    ],
  };

export const PEAK_DIAGNOSTICS_BUNDLE_REPORT_TEMPLATE_CHAIN: WorkflowTemplateChainPreset = {
  id: "peak_diagnostics_bundle_report",
  label: "peak extract -> bundle -> report",
  source: "built-in",
  summary:
    "Bundle electrostatic, thermal, and thermo peak-response extracts, evaluate a unified guard, and export a markdown report.",
  tags: ["peak", "diagnostics", "bundle", "guard", "report", "headless"],
  templates: [
    { kind: "extract", operatorId: "extract.electrostatic_peak_field" },
    { kind: "extract", operatorId: "extract.heat_peak_flux" },
    { kind: "extract", operatorId: "extract.thermo_peak_response" },
    { kind: "transform", operatorId: "transform.compose_diagnostics_bundle" },
    {
      kind: "transform",
      operatorId: "transform.evaluate_diagnostics_bundle_guard",
      config: {
        rules: [
          {
            source: "thermal",
            field: "thermal_flux_peak_magnitude",
            threshold: 25.0,
            severity: "warn",
            label: "thermal flux peak",
          },
          {
            source: "thermo",
            field: "thermo_peak_stress",
            comparison: "gt",
            threshold: 180.0,
            severity: "block",
            label: "thermo stress peak",
          },
          {
            source: "electrostatic",
            field: "electrostatic_field_peak_magnitude",
            comparison: "gt",
            threshold: 9.0,
            severity: "warn",
            label: "field ceiling",
          },
        ],
      },
    },
    { kind: "transform", operatorId: "transform.compose_diagnostics_report_payload" },
    { kind: "export", operatorId: "export.diagnostics_bundle_markdown", config: { title: "Peak Diagnostics Bundle Report" } },
  ],
  connections: [
    { from: 0, to: 3, toPort: "electrostatic" },
    { from: 1, to: 3, toPort: "thermal" },
    { from: 2, to: 3, toPort: "thermo" },
    { from: 3, to: 4, toPort: "bundle" },
    { from: 3, to: 5, toPort: "bundle" },
    { from: 4, to: 5, toPort: "guard" },
    { from: 5, to: 6, toPort: "bundle" },
  ],
};
