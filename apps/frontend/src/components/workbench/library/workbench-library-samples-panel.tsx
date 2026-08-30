"use client";

import { useMemo } from "react";

import type {
  SamplePage,
  WorkbenchLibrarySidebarProps,
} from "@/components/workbench/library/workbench-library-sidebar-types";
import type { StudyDomainKey } from "@/lib/workbench/view-models";

type WorkbenchLibrarySamplesPanelProps = Pick<
  WorkbenchLibrarySidebarProps,
  | "labels"
  | "onImportModel"
  | "onOpenSample"
  | "onRefreshWorkflowCatalog"
  | "onRunWorkflowCatalog"
  | "sampleRows"
  | "workflowCatalogBusy"
  | "workflowCatalogEntries"
> & {
  samplePage: SamplePage;
  selectedSampleDomain: StudyDomainKey;
  setSamplePage: (page: SamplePage) => void;
  setSelectedSampleDomain: (domain: StudyDomainKey) => void;
};

export function WorkbenchLibrarySamplesPanel({
  labels,
  onImportModel,
  onOpenSample,
  onRefreshWorkflowCatalog,
  onRunWorkflowCatalog,
  samplePage,
  sampleRows,
  selectedSampleDomain,
  setSamplePage,
  setSelectedSampleDomain,
  workflowCatalogBusy,
  workflowCatalogEntries,
}: WorkbenchLibrarySamplesPanelProps) {
  const groupedSampleRows = useMemo(() => {
    const groups = new Map<string, { label: string; rows: typeof sampleRows }>();
    for (const sample of sampleRows.filter((entry) => entry.domainKey === selectedSampleDomain)) {
      const existing = groups.get(sample.familyKey);
      if (existing) {
        existing.rows.push(sample);
      } else {
        groups.set(sample.familyKey, { label: sample.familyLabel, rows: [sample] });
      }
    }
    return Array.from(groups.values());
  }, [sampleRows, selectedSampleDomain]);
  const sampleDomainOptions = useMemo(
    () =>
      [
        { key: "mechanical" as const, fallback: "Mechanical" },
        { key: "thermal" as const, fallback: "Thermal" },
        { key: "thermoMechanical" as const, fallback: "Thermo-mechanical" },
      ].map(({ key, fallback }) => ({
        key,
        label: sampleRows.find((entry) => entry.domainKey === key)?.domainLabel ?? fallback,
      })),
    [sampleRows],
  );

  return (
    <section className="sidebar-card">
      <div className="card-head">
        <h2>{labels.sampleLibrary}</h2>
      </div>
      <div className="panel-tabs panel-tabs--wide">
        <button
          className={`panel-tab${samplePage === "catalog" ? " panel-tab--active" : ""}`}
          data-workbench-library-sample-page="catalog"
          onClick={() => setSamplePage("catalog")}
          type="button"
        >
          {labels.sampleCatalogPage}
        </button>
        <button
          className={`panel-tab${samplePage === "import" ? " panel-tab--active" : ""}`}
          data-workbench-library-sample-page="import"
          onClick={() => setSamplePage("import")}
          type="button"
        >
          {labels.sampleImportPage}
        </button>
      </div>
      {samplePage === "catalog" ? (
        <>
          <p className="card-copy">{labels.historyHint}</p>
          <div className="form-grid compact">
            <label>
              <span>{labels.studyDomain}</span>
              <div className="button-row">
                {sampleDomainOptions.map((option) => (
                  <button
                    key={option.key}
                    aria-label={`workbench-sample-domain:${option.key}`}
                    className={`ghost-button ghost-button--compact${selectedSampleDomain === option.key ? " ghost-button--active" : ""}`}
                    onClick={() => setSelectedSampleDomain(option.key)}
                    type="button"
                  >
                    {option.label}
                  </button>
                ))}
              </div>
            </label>
          </div>
          <div className="history-list sample-group-list">
            {groupedSampleRows.length === 0 ? <p className="card-copy">{labels.noDomainStudies}</p> : null}
            {groupedSampleRows.map((group) => (
              <div key={group.label} className="sample-group">
                <div className="sample-group__head">
                  <strong>{group.label}</strong>
                  <span>{group.rows.length}</span>
                </div>
                <div className="sample-group__items">
                  {group.rows.map((sample) => (
                    <button
                      key={sample.id}
                      aria-label={`workbench-sample:${sample.id}`}
                      className="history-item"
                      onClick={() => onOpenSample(sample.href)}
                      type="button"
                    >
                      <strong>{sample.name}</strong>
                      <span>{sample.kindLabel}</span>
                      <small>{sample.summary}</small>
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>
          <div className="sample-group">
            <div className="sample-group__head">
              <strong>{labels.workflowCatalogTitle}</strong>
              <button className="link-button" disabled={workflowCatalogBusy} onClick={onRefreshWorkflowCatalog} type="button">
                {labels.workflowCatalogRefresh}
              </button>
            </div>
            <p className="card-copy">{labels.workflowCatalogHint}</p>
            <div className="sample-group__items">
              {workflowCatalogEntries.length === 0 ? (
                <p className="card-copy">{workflowCatalogBusy ? labels.workflowCatalogRefresh : labels.workflowCatalogEmpty}</p>
              ) : null}
              {workflowCatalogEntries.map((workflow) => (
                <div key={workflow.id} className="history-item">
                  <strong>{workflow.name}</strong>
                  <span>{workflow.version}</span>
                  <small>{workflow.summary}</small>
                  <div className="button-row">
                    <button
                      className="ghost-button ghost-button--compact"
                      disabled={workflowCatalogBusy}
                      onClick={() => onRunWorkflowCatalog(workflow.id)}
                      type="button"
                    >
                      {labels.workflowCatalogRun}
                    </button>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </>
      ) : (
        <label className="import-box">
          <span>{labels.importModel}</span>
          <small>{labels.importHint}</small>
          <input
            type="file"
            accept=".json,application/json"
            onChange={(event) => onImportModel(event.target.files?.[0])}
          />
        </label>
      )}
    </section>
  );
}
