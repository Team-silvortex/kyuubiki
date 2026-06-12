"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import type { WorkflowPackage } from "@/components/workbench/workflow/workbench-workflow-package";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowPackageManifestCardProps = {
  labels: WorkflowSidebarLabels;
  workflow: WorkflowCatalogEntry;
  importedPackage: WorkflowPackage | null;
};

function formatList(values?: string[]) {
  const normalized = values?.filter(Boolean) ?? [];
  return normalized.length > 0 ? normalized.join(", ") : "--";
}

function formatDateTime(value?: string) {
  if (!value) return "--";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export function WorkbenchWorkflowPackageManifestCard({
  labels,
  workflow,
  importedPackage,
}: WorkbenchWorkflowPackageManifestCardProps) {
  const mountedPackageId = importedPackage?.package_id ?? workflow.local?.imported_from_package_id ?? null;
  const mountedPackageVersion =
    importedPackage?.package_version ?? workflow.local?.imported_from_package_version ?? null;
  const mountedPackageTags = importedPackage?.tags ?? workflow.local?.tags ?? workflow.capability_tags ?? [];
  const mountedDomains = importedPackage?.search_index.domains ?? workflow.domains ?? [];
  const mountedCapabilities =
    importedPackage?.search_index.capability_tags ?? workflow.capability_tags ?? [];
  const mountedOperators =
    importedPackage?.search_index.operator_ids ??
    workflow.graph?.nodes
      .map((node) => node.operator_id)
      .filter((value): value is string => typeof value === "string") ??
    [];
  const mountedEntryArtifacts =
    importedPackage?.search_index.entry_artifacts ??
    workflow.entry_inputs.map((entry) => entry.artifact_type);
  const mountedOutputArtifacts =
    importedPackage?.search_index.output_artifacts ??
    workflow.output_artifacts.map((entry) => entry.artifact_type);

  if (!mountedPackageId) {
    return (
      <section className="sidebar-card sidebar-card--compact" data-workflow-package-card="card">
        <div className="card-head">
          <h2>{labels.packageManifestTitle}</h2>
          <span className="status-pill status-pill--watch">{workflow.local ? labels.localWorkflowBadgeLabel : workflow.version}</span>
        </div>
        <p className="card-copy">{labels.packageManifestNoneLabel}</p>
      </section>
    );
  }

  return (
    <section className="sidebar-card sidebar-card--compact" data-workflow-package-card="card">
      <div className="card-head">
        <h2>{labels.packageManifestTitle}</h2>
        <span className={`status-pill status-pill--${importedPackage ? "good" : "watch"}`}>
          {importedPackage ? labels.packageManifestDraftLabel : labels.packageManifestMountedLabel}
        </span>
      </div>
      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.localWorkflowPackageIdLabel}</span>
          <strong>{mountedPackageId}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.localWorkflowPackageVersionLabel}</span>
          <strong>{mountedPackageVersion ?? "--"}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.localWorkflowTagsLabel}</span>
          <strong>{formatList(mountedPackageTags)}</strong>
        </div>
        {importedPackage ? (
          <div className="sidebar-list__row">
            <span>{labels.packageManifestExportedAtLabel}</span>
            <strong>{formatDateTime(importedPackage.exported_at)}</strong>
          </div>
        ) : null}
        <div className="sidebar-list__row">
          <span>{labels.packageManifestDomainsLabel}</span>
          <strong>{formatList(mountedDomains)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageManifestCapabilitiesLabel}</span>
          <strong>{formatList(mountedCapabilities)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageManifestOperatorsLabel}</span>
          <strong>{formatList(mountedOperators)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageManifestEntryArtifactsLabel}</span>
          <strong>{formatList(mountedEntryArtifacts)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageManifestOutputArtifactsLabel}</span>
          <strong>{formatList(mountedOutputArtifacts)}</strong>
        </div>
      </div>
    </section>
  );
}
