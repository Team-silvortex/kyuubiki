import type {
  WorkflowResolvedDiagnosticsReport,
} from "@/components/workbench/workflow/workbench-workflow-diagnostics-report-contract";
import type { WorkflowSummaryArtifactFieldValue } from "@/lib/api";

export type WorkflowDiagnosticsFocusContext = Record<string, WorkflowSummaryArtifactFieldValue>;
export type WorkflowDiagnosticsFocusCardSummary = {
  anchorLine: string | null;
  companionLine: string | null;
  domain: "electrostatic" | "thermal" | "thermo" | "diagnostics";
  domainLabel: string;
  sections: Array<{ label: string; lines: string[] }>;
  vectorLines: string[];
};

export function resolveWorkflowDiagnosticsReportMode(
  report: WorkflowResolvedDiagnosticsReport | null | undefined,
) {
  if (!report) return "diagnostics" as const;
  const focusMetricKeys = Object.keys(report.payload.report_focus_metrics);
  return focusMetricKeys.some(
    (key) =>
      key === "electrostatic.field_peak" ||
      key === "thermal.flux_peak" ||
      key === "thermo.stress_peak" ||
      key === "thermo.displacement_peak",
  )
    ? ("peak" as const)
    : ("diagnostics" as const);
}

export function resolveWorkflowDiagnosticsReportTitle(
  report: WorkflowResolvedDiagnosticsReport | null | undefined,
) {
  return resolveWorkflowDiagnosticsReportMode(report) === "peak"
    ? "Peak diagnostics focus"
    : "Diagnostics focus";
}

const PEAK_HIGHLIGHT_PRIORITY = [
  "electrostatic.field_peak",
  "thermal.flux_peak",
  "thermo.displacement_peak",
  "thermo.stress_peak",
] as const;

function priorityForPeakHighlight(id: string) {
  const index = PEAK_HIGHLIGHT_PRIORITY.indexOf(
    id as (typeof PEAK_HIGHLIGHT_PRIORITY)[number],
  );
  return index >= 0 ? index : PEAK_HIGHLIGHT_PRIORITY.length;
}

export function orderWorkflowDiagnosticsHighlights(
  report: WorkflowResolvedDiagnosticsReport | null | undefined,
) {
  const highlights = report?.payload.report_highlights ?? [];
  if (resolveWorkflowDiagnosticsReportMode(report) !== "peak") return highlights;
  return [...highlights].sort((left, right) => {
    const priorityDelta = priorityForPeakHighlight(left.id) - priorityForPeakHighlight(right.id);
    if (priorityDelta !== 0) return priorityDelta;
    if (left.attention !== right.attention) return left.attention ? -1 : 1;
    return left.label.localeCompare(right.label);
  });
}

export function orderWorkflowDiagnosticsFocusMetrics(
  report: WorkflowResolvedDiagnosticsReport | null | undefined,
) {
  const entries = Object.entries(report?.payload.report_focus_metrics ?? {});
  if (resolveWorkflowDiagnosticsReportMode(report) !== "peak") return entries;
  return [...entries].sort(([leftKey], [rightKey]) => {
    const priorityDelta = priorityForPeakHighlight(leftKey) - priorityForPeakHighlight(rightKey);
    if (priorityDelta !== 0) return priorityDelta;
    return leftKey.localeCompare(rightKey);
  });
}

export function formatWorkflowDiagnosticsMetricValue(
  value: WorkflowSummaryArtifactFieldValue,
  digits = 3,
) {
  return typeof value === "number" ? value.toExponential(digits) : String(value);
}

export function resolveWorkflowDiagnosticsFocusContext(
  report: WorkflowResolvedDiagnosticsReport | null | undefined,
  key: string,
) {
  const context = report?.payload.report_focus_context?.[key];
  return context && Object.keys(context).length > 0 ? context : null;
}

export function listWorkflowDiagnosticsFocusContextEntries(
  context: WorkflowDiagnosticsFocusContext | null | undefined,
) {
  if (!context) return [] as Array<[string, WorkflowSummaryArtifactFieldValue]>;
  const priority = [
    "source",
    "value_field",
    "peak_node_id",
    "peak_element_id",
    "peak_displacement_x",
    "peak_displacement_y",
    "peak_node_temperature_delta",
    "peak_electric_field_x",
    "peak_electric_field_y",
    "peak_flux_density_x",
    "peak_flux_density_y",
    "peak_heat_flux_x",
    "peak_heat_flux_y",
    "peak_temperature_gradient_x",
    "peak_temperature_gradient_y",
    "peak_stress_x",
    "peak_stress_y",
    "peak_tau_xy",
    "peak_element_temperature_delta",
    "thermo_peak_thermal_strain_id",
  ];
  return Object.entries(context).sort(([left], [right]) => {
    const leftIndex = priority.indexOf(left);
    const rightIndex = priority.indexOf(right);
    if (leftIndex !== rightIndex) {
      if (leftIndex < 0) return 1;
      if (rightIndex < 0) return -1;
      return leftIndex - rightIndex;
    }
    return left.localeCompare(right);
  });
}

export function formatWorkflowDiagnosticsContextLabel(key: string) {
  return key.replaceAll("_", " ");
}

export function summarizeWorkflowDiagnosticsFocusContext(
  context: WorkflowDiagnosticsFocusContext | null | undefined,
  digits = 3,
) {
  const entries = listWorkflowDiagnosticsFocusContextEntries(context);
  if (entries.length === 0) return [] as string[];
  const lines: string[] = [];
  const source = context?.source;
  const valueField = context?.value_field;
  const anchor = context?.peak_node_id ?? context?.peak_element_id ?? context?.thermo_peak_thermal_strain_id;
  if (source || valueField || anchor) {
    lines.push(
      [source ? `source ${String(source)}` : null, valueField ? `field ${String(valueField)}` : null, anchor ? `anchor ${String(anchor)}` : null]
        .filter(Boolean)
        .join(" · "),
    );
  }
  const vectors = [
    ["field", context?.peak_electric_field_x, context?.peak_electric_field_y],
    ["flux density", context?.peak_flux_density_x, context?.peak_flux_density_y],
    ["heat flux", context?.peak_heat_flux_x, context?.peak_heat_flux_y],
    ["gradient", context?.peak_temperature_gradient_x, context?.peak_temperature_gradient_y],
    ["displacement", context?.peak_displacement_x, context?.peak_displacement_y],
    ["stress", context?.peak_stress_x, context?.peak_stress_y],
  ] as const;
  vectors.forEach(([label, x, y]) => {
    if (x == null && y == null) return;
    const components = [
      x != null ? `x=${formatWorkflowDiagnosticsMetricValue(x, digits)}` : null,
      y != null ? `y=${formatWorkflowDiagnosticsMetricValue(y, digits)}` : null,
    ].filter(Boolean);
    lines.push(
      `${label}: ${components.join(" · ")}`,
    );
  });
  const companionFields = entries.filter(([key]) =>
    key === "peak_node_temperature_delta" ||
    key === "peak_element_temperature_delta" ||
    key === "peak_tau_xy",
  );
  if (companionFields.length > 0) {
    lines.push(
      companionFields
        .map(
          ([key, value]) =>
            `${formatWorkflowDiagnosticsContextLabel(key)}=${formatWorkflowDiagnosticsMetricValue(value, digits)}`,
        )
        .join(" · "),
    );
  }
  return lines;
}

export function buildWorkflowDiagnosticsFocusCardSummary(
  key: string,
  context: WorkflowDiagnosticsFocusContext | null | undefined,
  digits = 3,
): WorkflowDiagnosticsFocusCardSummary {
  const lines = summarizeWorkflowDiagnosticsFocusContext(context, digits);
  const domain = key.startsWith("electrostatic.")
    ? "electrostatic"
    : key.startsWith("thermal.")
      ? "thermal"
      : key.startsWith("thermo.")
        ? "thermo"
        : "diagnostics";
  const domainLabel =
    domain === "electrostatic"
      ? "electrostatic field"
      : domain === "thermal"
        ? "thermal transport"
        : domain === "thermo"
          ? "thermo-mechanical"
          : "diagnostics";
  const vectorLines = lines.filter(
    (line) =>
      line.startsWith("field:") ||
      line.startsWith("flux density:") ||
      line.startsWith("heat flux:") ||
      line.startsWith("gradient:") ||
      line.startsWith("displacement:") ||
      line.startsWith("stress:"),
  );
  const companionLine =
    lines.find((line) => line.includes("temperature delta")) ??
    lines.find((line) => line.includes("tau xy")) ??
    null;
  const electrostaticVectors = vectorLines.filter(
    (line) => line.startsWith("field:") || line.startsWith("flux density:"),
  );
  const thermalVectors = vectorLines.filter(
    (line) => line.startsWith("heat flux:") || line.startsWith("gradient:"),
  );
  const thermoVectors = vectorLines.filter(
    (line) => line.startsWith("displacement:") || line.startsWith("stress:"),
  );
  const sections =
    domain === "electrostatic"
      ? [
          { label: "sample", lines: anchorLineList(lines[0] ?? null) },
          { label: "field vector", lines: electrostaticVectors },
        ]
      : domain === "thermal"
        ? [
            { label: "sample", lines: anchorLineList(lines[0] ?? null) },
            { label: "transport", lines: thermalVectors },
          ]
        : domain === "thermo"
          ? [
              { label: "sample", lines: anchorLineList(lines[0] ?? null) },
              { label: "response", lines: thermoVectors },
              { label: "coupling", lines: companionLine ? [companionLine] : [] },
            ]
          : [
              { label: "context", lines: lines.slice(0, 3) },
            ];
  return {
    anchorLine: lines[0] ?? null,
    companionLine,
    domain,
    domainLabel,
    sections: sections.filter((section) => section.lines.length > 0),
    vectorLines,
  };
}

function anchorLineList(line: string | null) {
  return line ? [line] : [];
}

export function summarizeWorkflowDiagnosticsReport(
  report: WorkflowResolvedDiagnosticsReport | null | undefined,
  options?: {
    maxHighlights?: number;
    maxFocusMetrics?: number;
    digits?: number;
  },
) {
  if (!report) return null;
  const mode = resolveWorkflowDiagnosticsReportMode(report);
  const maxHighlights = options?.maxHighlights ?? 2;
  const maxFocusMetrics = options?.maxFocusMetrics ?? 3;
  const digits = options?.digits ?? 3;
  const highlightPreview = orderWorkflowDiagnosticsHighlights(report)
    .slice(0, maxHighlights)
    .map((entry) => `${entry.label}=${formatWorkflowDiagnosticsMetricValue(entry.value, digits)}`)
    .join(", ");
  if (highlightPreview) return mode === "peak" ? `peak review: ${highlightPreview}` : highlightPreview;
  const focusPreview = orderWorkflowDiagnosticsFocusMetrics(report)
    .slice(0, maxFocusMetrics)
    .map(([key, value]) => `${key}=${formatWorkflowDiagnosticsMetricValue(value, digits)}`)
    .join(", ");
  if (!focusPreview) return null;
  return mode === "peak" ? `peak review: ${focusPreview}` : focusPreview;
}
