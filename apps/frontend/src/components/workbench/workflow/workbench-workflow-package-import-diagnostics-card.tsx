"use client";

import { useMemo, useState } from "react";
import type { WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";
import { scoreWorkflowPackageImportDiagnosticSearch } from "@/components/workbench/workflow/workbench-workflow-package-diagnostics-search";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

export function WorkbenchWorkflowPackageImportDiagnosticsCard(props: {
  activeFilter?: "all" | "node" | "dataset" | "package";
  diagnostics: WorkflowPackageImportDiagnostic[];
  labels: WorkflowSidebarLabels;
  onLocateDiagnostic?: (diagnostic: WorkflowPackageImportDiagnostic) => void;
}) {
  const { activeFilter = "all", diagnostics, labels, onLocateDiagnostic } = props;
  const [query, setQuery] = useState("");
  const scopeFilteredDiagnostics =
    activeFilter === "node"
      ? diagnostics.filter((diagnostic) => diagnostic.locate?.kind === "node")
      : activeFilter === "dataset"
        ? diagnostics.filter((diagnostic) => diagnostic.locate?.kind === "dataset")
        : activeFilter === "package"
          ? diagnostics.filter((diagnostic) => diagnostic.locate?.kind === "package")
          : diagnostics;
  const filteredDiagnostics = useMemo(() => {
    const normalized = query.trim();
    if (!normalized) return scopeFilteredDiagnostics;
    return scopeFilteredDiagnostics
      .flatMap((diagnostic) => {
        const score = scoreWorkflowPackageImportDiagnosticSearch(diagnostic, normalized);
        return score == null ? [] : [{ diagnostic, score }];
      })
      .sort((left, right) => right.score - left.score)
      .map((entry) => entry.diagnostic);
  }, [query, scopeFilteredDiagnostics]);
  if (diagnostics.length === 0) return null;

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>Package import diagnostics</h2>
        <span className="status-pill status-pill--watch">{filteredDiagnostics.length}</span>
      </div>
      <label style={{ display: "grid", gap: "0.35rem", marginBottom: "0.75rem" }}>
        <span className="card-copy">{labels.packageDiagnosticsSearchLabel}</span>
        <input
          onChange={(event) => setQuery(event.target.value)}
          placeholder={labels.packageDiagnosticsSearchPlaceholder}
          type="search"
          value={query}
        />
      </label>
      {filteredDiagnostics.length === 0 ? <p className="card-copy">{labels.packageDiagnosticsSearchEmptyLabel}</p> : null}
      <div style={{ display: "grid", gap: "0.35rem" }}>
        {filteredDiagnostics.map((diagnostic, index) => (
          <div key={`${index}:${diagnostic.message}`} style={{ display: "grid", gap: "0.15rem" }}>
            <strong style={{ fontSize: "0.9rem" }}>issue {index + 1}</strong>
            <span className="card-copy">{diagnostic.message}</span>
            {diagnostic.locate ? <div className="button-row"><button onClick={() => onLocateDiagnostic?.(diagnostic)} type="button">locate</button></div> : null}
          </div>
        ))}
      </div>
    </section>
  );
}
