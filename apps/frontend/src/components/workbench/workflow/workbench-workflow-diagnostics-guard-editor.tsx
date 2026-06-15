"use client";

import type { WorkflowGraphNode } from "@/lib/api";

type WorkbenchWorkflowDiagnosticsGuardEditorProps = {
  node: WorkflowGraphNode;
  onUpdateNode: (
    nodeId: string,
    updater: (node: WorkflowGraphNode) => WorkflowGraphNode,
  ) => void;
};

type DiagnosticsGuardRule = {
  source: "electrostatic" | "thermal" | "thermo";
  field: string;
  threshold: number;
  severity: "warn" | "block";
  comparison?: string;
  label?: string;
};

const COMPARISONS = ["gt", "gte", "lt", "lte"] as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asGuardRule(value: unknown): DiagnosticsGuardRule | null {
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
    comparison: typeof value.comparison === "string" ? value.comparison : "gt",
    label: typeof value.label === "string" ? value.label : "",
  };
}

function resolveRules(node: WorkflowGraphNode) {
  return Array.isArray(node.config?.rules)
    ? node.config.rules.map(asGuardRule).filter(Boolean)
    : [];
}

function updateRules(
  node: WorkflowGraphNode,
  onUpdateNode: WorkbenchWorkflowDiagnosticsGuardEditorProps["onUpdateNode"],
  updater: (rules: DiagnosticsGuardRule[]) => DiagnosticsGuardRule[],
) {
  const rules = resolveRules(node) as DiagnosticsGuardRule[];
  const nextRules = updater([...rules]);
  onUpdateNode(node.id, (current) => ({
    ...current,
    config: {
      ...(typeof current.config === "object" && current.config !== null
        ? current.config
        : {}),
      rules: nextRules,
    },
  }));
}

export function WorkbenchWorkflowDiagnosticsGuardEditor({
  node,
  onUpdateNode,
}: WorkbenchWorkflowDiagnosticsGuardEditorProps) {
  if (node.operator_id !== "transform.evaluate_diagnostics_bundle_guard") return null;
  const rules = resolveRules(node) as DiagnosticsGuardRule[];

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h3>Diagnostics Guard</h3>
        <button
          onClick={() =>
            updateRules(node, onUpdateNode, (current) => [
              ...current,
              {
                source: "thermal",
                field: "thermal_temperature_max",
                threshold: 120,
                severity: "warn",
                comparison: "gt",
                label: "thermal temperature",
              },
            ])
          }
          type="button"
        >
          Add rule
        </button>
      </div>
      <div className="sidebar-stack">
        {rules.map((rule, index) => (
          <div className="form-grid compact" key={`${node.id}:guard:${index}`}>
            <label>
              <span>Source</span>
              <select
                onChange={(event) =>
                  updateRules(node, onUpdateNode, (current) =>
                    current.map((entry, entryIndex) =>
                      entryIndex === index
                        ? { ...entry, source: event.target.value as DiagnosticsGuardRule["source"] }
                        : entry,
                    ),
                  )
                }
                value={rule.source}
              >
                <option value="electrostatic">electrostatic</option>
                <option value="thermal">thermal</option>
                <option value="thermo">thermo</option>
              </select>
            </label>
            <label>
              <span>Field</span>
              <input
                onChange={(event) =>
                  updateRules(node, onUpdateNode, (current) =>
                    current.map((entry, entryIndex) =>
                      entryIndex === index ? { ...entry, field: event.target.value } : entry,
                    ),
                  )
                }
                value={rule.field}
              />
            </label>
            <label>
              <span>Threshold</span>
              <input
                onChange={(event) =>
                  updateRules(node, onUpdateNode, (current) =>
                    current.map((entry, entryIndex) =>
                      entryIndex === index
                        ? { ...entry, threshold: Number(event.target.value || 0) }
                        : entry,
                    ),
                  )
                }
                type="number"
                value={rule.threshold}
              />
            </label>
            <label>
              <span>Severity</span>
              <select
                onChange={(event) =>
                  updateRules(node, onUpdateNode, (current) =>
                    current.map((entry, entryIndex) =>
                      entryIndex === index
                        ? { ...entry, severity: event.target.value as DiagnosticsGuardRule["severity"] }
                        : entry,
                    ),
                  )
                }
                value={rule.severity}
              >
                <option value="warn">warn</option>
                <option value="block">block</option>
              </select>
            </label>
            <label>
              <span>Comparison</span>
              <select
                onChange={(event) =>
                  updateRules(node, onUpdateNode, (current) =>
                    current.map((entry, entryIndex) =>
                      entryIndex === index ? { ...entry, comparison: event.target.value } : entry,
                    ),
                  )
                }
                value={rule.comparison ?? "gt"}
              >
                {COMPARISONS.map((comparison) => (
                  <option key={comparison} value={comparison}>
                    {comparison}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>Label</span>
              <input
                onChange={(event) =>
                  updateRules(node, onUpdateNode, (current) =>
                    current.map((entry, entryIndex) =>
                      entryIndex === index ? { ...entry, label: event.target.value } : entry,
                    ),
                  )
                }
                value={rule.label ?? ""}
              />
            </label>
            <button
              onClick={() =>
                updateRules(node, onUpdateNode, (current) =>
                  current.filter((_, entryIndex) => entryIndex !== index),
                )
              }
              type="button"
            >
              Remove rule
            </button>
          </div>
        ))}
        {rules.length === 0 ? <p className="card-copy">No diagnostics guard rules configured.</p> : null}
      </div>
    </section>
  );
}
