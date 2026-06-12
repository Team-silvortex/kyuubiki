"use client";

import type { WorkflowCatalogEntry } from "@/lib/api";
import type {
  WorkflowPackage,
  WorkflowPackageContractEntry,
} from "@/components/workbench/workflow/workbench-workflow-package";
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

function formatContractEntries(entries: WorkflowPackageContractEntry[]) {
  if (entries.length === 0) return "--";
  return entries
    .map((entry) =>
      [
        entry.artifact_type,
        entry.dataset_value ? `dataset:${entry.dataset_value}` : null,
        entry.schema_ref,
      ]
        .filter(Boolean)
        .join(" -> "),
    )
    .join(", ");
}

function formatBridgeSeedSummaries(
  values: Array<{
    operator_id: string;
    node_count: number;
    element_count: number;
    contract_version?: string;
  }>,
) {
  if (values.length === 0) return "--";
  return values
    .map((value) =>
      [
        value.operator_id,
        `${value.node_count}n/${value.element_count}e`,
        value.contract_version,
      ]
        .filter(Boolean)
        .join(" -> "),
    )
    .join(", ");
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
  const contractManifest =
    importedPackage?.contract_manifest ??
    (workflow.graph
      ? {
          dataset_schema: workflow.graph.dataset_contract?.schema_version,
          dataset_contract_id: workflow.graph.dataset_contract?.id,
          dataset_contract_version: workflow.graph.dataset_contract?.version,
          dataset_value_ids: workflow.graph.dataset_contract?.values.map((value) => value.id) ?? [],
          entry_contracts: [],
          output_contracts: [],
        }
      : null);
  const runtimeManifest =
    importedPackage?.runtime_manifest ??
    (workflow.graph
      ? {
          required_operator_ids:
            workflow.graph.nodes
              .map((node) => node.operator_id)
              .filter((value): value is string => typeof value === "string") ?? [],
          sample_input_node_ids: workflow.entry_inputs.map((entry) => entry.node_id),
          included_input_text_node_ids: Object.keys(
            importedPackage?.workflow.input_artifact_texts ??
              workflow.local?.input_artifact_texts ??
              {},
          ),
          bridge_seed_summaries: [],
        }
      : null);

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
          <span>{labels.datasetSchemaLabel}</span>
          <strong>
            {contractManifest?.dataset_contract_id
              ? `${contractManifest.dataset_contract_id}@${contractManifest.dataset_contract_version ?? "1"}`
              : contractManifest?.dataset_schema ?? "--"}
          </strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageManifestCapabilitiesLabel}</span>
          <strong>{formatList(mountedCapabilities)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageManifestOperatorsLabel}</span>
          <strong>{formatList(runtimeManifest?.required_operator_ids ?? mountedOperators)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageManifestEntryArtifactsLabel}</span>
          <strong>{formatList(mountedEntryArtifacts)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.packageManifestOutputArtifactsLabel}</span>
          <strong>{formatList(mountedOutputArtifacts)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.operatorInputSchemaLabel}</span>
          <strong>{formatContractEntries(contractManifest?.entry_contracts ?? [])}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.operatorOutputSchemaLabel}</span>
          <strong>{formatContractEntries(contractManifest?.output_contracts ?? [])}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.inputArtifactsTitle}</span>
          <strong>{formatList(runtimeManifest?.included_input_text_node_ids)}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.bridgeContractTitle}</span>
          <strong>{formatBridgeSeedSummaries(runtimeManifest?.bridge_seed_summaries ?? [])}</strong>
        </div>
      </div>
    </section>
  );
}
