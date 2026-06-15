"use client";

import type { WorkflowGraphNode } from "@/lib/api";

type WorkflowDiagnosticsDomain =
  | "electrostatic"
  | "thermal"
  | "thermo";

type WorkflowDiagnosticsScenario =
  | "electrostatic_warn"
  | "thermal_warn"
  | "thermo_block";

export type WorkflowDiagnosticsGuardRule = {
  source: WorkflowDiagnosticsDomain;
  field: string;
  threshold: number;
  severity: "warn" | "block";
  comparison?: string;
  label?: string;
};

export type WorkflowDiagnosticsScenarioPreview = {
  scenario: WorkflowDiagnosticsScenario;
  domain: WorkflowDiagnosticsDomain;
  field: string;
  threshold: number | null;
  targetValue: number;
  expectedSeverity: "warn" | "block";
  label: string;
};

type WorkflowDiagnosticsSummaryInfo = {
  domain: WorkflowDiagnosticsDomain;
  subject: string;
  prefix: string;
  nodeCount: number | null;
  elementCount: number | null;
  metricGroups: string[];
  numericMetrics: Array<[string, number]>;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asNumber(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function resolveSummaryPayload(
  payload: unknown,
): Record<string, unknown> | null {
  if (!isRecord(payload)) return null;
  const summary = payload.summary;
  if (!isRecord(summary)) return null;
  if (summary.diagnostic_contract !== "kyuubiki.workflow_diagnostics/v1") {
    return null;
  }
  return summary;
}

export function resolveWorkflowDiagnosticsSummaryInfo(
  payload: unknown,
): WorkflowDiagnosticsSummaryInfo | null {
  const summary = resolveSummaryPayload(payload);
  if (!summary) return null;
  const domain = summary?.diagnostic_domain;
  const subject = summary?.diagnostic_subject;
  const prefix = summary?.diagnostic_prefix;
  const metricGroups = Array.isArray(summary?.diagnostic_metric_groups)
    ? summary.diagnostic_metric_groups.filter(
        (entry): entry is string => typeof entry === "string",
      )
    : [];
  if (
    domain !== "electrostatic" &&
    domain !== "thermal" &&
    domain !== "thermo"
  ) {
    return null;
  }
  if (typeof subject !== "string" || typeof prefix !== "string") return null;

  const numericMetrics = Object.entries(summary)
    .filter(
      ([key, value]) =>
        key.startsWith(`${prefix}_`) &&
        typeof value === "number" &&
        Number.isFinite(value),
    )
    .slice(0, 6) as Array<[string, number]>;

  return {
    domain,
    subject,
    prefix,
    nodeCount: asNumber(summary.diagnostic_node_count),
    elementCount: asNumber(summary.diagnostic_element_count),
    metricGroups,
    numericMetrics,
  };
}

function asGuardRule(value: unknown): WorkflowDiagnosticsGuardRule | null {
  if (!isRecord(value)) return null;
  const source = value.source;
  const field = value.field;
  const threshold = value.threshold;
  const severity = value.severity;
  if (
    source !== "electrostatic" &&
    source !== "thermal" &&
    source !== "thermo"
  ) {
    return null;
  }
  if (typeof field !== "string" || typeof threshold !== "number") return null;
  if (severity !== "warn" && severity !== "block") return null;
  return {
    source,
    field,
    threshold,
    severity,
    comparison: typeof value.comparison === "string" ? value.comparison : undefined,
    label: typeof value.label === "string" ? value.label : undefined,
  };
}

export function resolveWorkflowDiagnosticsGuardRules(
  nodes: WorkflowGraphNode[],
) {
  const guardNode = nodes.find(
    (node) => node.operator_id === "transform.evaluate_diagnostics_bundle_guard",
  );
  const rules = Array.isArray(guardNode?.config?.rules)
    ? guardNode.config.rules.map(asGuardRule).filter(Boolean)
    : [];
  return rules as WorkflowDiagnosticsGuardRule[];
}

function resolveScenarioRule(
  scenario: WorkflowDiagnosticsScenario,
  rules: WorkflowDiagnosticsGuardRule[],
) {
  return scenario === "electrostatic_warn"
    ? rules.find((rule) => rule.source === "electrostatic")
    : scenario === "thermal_warn"
      ? rules.find((rule) => rule.source === "thermal")
      : rules.find((rule) => rule.source === "thermo");
}

function resolveScenarioTargetValue(
  scenario: WorkflowDiagnosticsScenario,
  rules: WorkflowDiagnosticsGuardRule[],
) {
  const matchedRule = resolveScenarioRule(scenario, rules);
  if (!matchedRule) {
    return scenario === "electrostatic_warn"
      ? 9.6
      : scenario === "thermal_warn"
        ? 128.0
        : 196.0;
  }
  return matchedRule.severity === "block"
    ? matchedRule.threshold + Math.max(Math.abs(matchedRule.threshold) * 0.08, 1)
    : matchedRule.threshold + Math.max(Math.abs(matchedRule.threshold) * 0.03, 0.5);
}

export function resolveWorkflowDiagnosticsScenarioPreview(
  scenario: WorkflowDiagnosticsScenario,
  rules: WorkflowDiagnosticsGuardRule[],
): WorkflowDiagnosticsScenarioPreview {
  const matchedRule = resolveScenarioRule(scenario, rules);
  const targetValue = resolveScenarioTargetValue(scenario, rules);
  const domain =
    scenario === "electrostatic_warn"
      ? "electrostatic"
      : scenario === "thermal_warn"
        ? "thermal"
        : "thermo";
  return {
    scenario,
    domain,
    field:
      matchedRule?.field ??
      (domain === "electrostatic"
        ? "electrostatic_field_peak_magnitude"
        : domain === "thermal"
          ? "thermal_temperature_max"
          : "thermo_peak_stress"),
    threshold: matchedRule?.threshold ?? null,
    targetValue,
    expectedSeverity: matchedRule?.severity ?? (domain === "thermo" ? "block" : "warn"),
    label:
      matchedRule?.label ??
      (domain === "electrostatic"
        ? "field ceiling"
        : domain === "thermal"
          ? "thermal temperature"
          : "stress ceiling"),
  };
}

export function applyWorkflowDiagnosticsScenario(
  payload: unknown,
  scenario: WorkflowDiagnosticsScenario,
  rules: WorkflowDiagnosticsGuardRule[] = [],
) {
  if (!isRecord(payload)) return payload;
  const summary = resolveSummaryPayload(payload);
  if (!summary) return payload;
  const nextSummary = { ...summary };
  const nextValue = resolveScenarioTargetValue(scenario, rules);

  if (scenario === "electrostatic_warn") {
    nextSummary.electrostatic_field_peak_magnitude = nextValue;
  }
  if (scenario === "thermal_warn") {
    nextSummary.thermal_temperature_max = nextValue;
  }
  if (scenario === "thermo_block") {
    nextSummary.thermo_peak_stress = nextValue;
  }

  return { ...payload, summary: nextSummary };
}
