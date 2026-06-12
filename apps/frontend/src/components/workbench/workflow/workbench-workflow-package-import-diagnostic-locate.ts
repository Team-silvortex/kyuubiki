"use client";

import type { WorkflowPackageImportDiagnostic } from "@/components/workbench/workflow/workbench-workflow-package-adapter";

export function locateWorkflowPackageImportDiagnostic(
  root: HTMLElement | null,
  diagnostic: WorkflowPackageImportDiagnostic,
  setSelectedDatasetValueId: (value: string | null) => void,
) {
  const locate = diagnostic.locate;
  if (!locate) return;
  if (locate.kind === "node") {
    return queueMicrotask(() =>
      root?.querySelector<HTMLElement>(`[data-workflow-node-id="${locate.nodeId}"]`)?.scrollIntoView({ block: "nearest", behavior: "smooth" }),
    );
  }
  if (locate.kind === "dataset") {
    return queueMicrotask(() => {
      if (locate.datasetValueId) setSelectedDatasetValueId(locate.datasetValueId);
      root
        ?.querySelector<HTMLElement>(
          locate.datasetValueId
            ? `[data-workflow-dataset-value-id="${locate.datasetValueId}"], [data-workflow-dataset-editor="editor"]`
            : '[data-workflow-dataset-editor="editor"]',
        )
        ?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
  }
  queueMicrotask(() =>
    root
      ?.querySelector<HTMLElement>('[data-workflow-package-card="card"], [data-workflow-package-policy-card="card"]')
      ?.scrollIntoView({ block: "nearest", behavior: "smooth" }),
  );
}
