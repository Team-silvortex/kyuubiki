"use client";

import type { WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";

export function WorkbenchWorkflowPackageImportDiagnosticsCard(props: {
  diagnostics: WorkflowPackageImportDiagnostic[];
  onLocateDiagnostic?: (diagnostic: WorkflowPackageImportDiagnostic) => void;
}) {
  const { diagnostics, onLocateDiagnostic } = props;
  if (diagnostics.length === 0) return null;

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>Package import diagnostics</h2>
        <span className="status-pill status-pill--watch">{diagnostics.length}</span>
      </div>
      <div style={{ display: "grid", gap: "0.35rem" }}>
        {diagnostics.map((diagnostic, index) => (
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
