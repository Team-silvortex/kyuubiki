"use client";

import { memo, useEffect, useState } from "react";
import { WorkbenchListPager, WorkbenchSubpanelNav } from "../workbench-compact-panels";
import { getWorkbenchCompactPanelCopy } from "../workbench-compact-panel-copy";
import type { WorkbenchCopy } from "../workbench-copy";

type SecurityAuditEntryRow = {
  id: string;
  at: string;
  action: string;
  source: string;
  risk: string;
  status: string;
  note: string;
};

type WorkbenchSecurityAuditCardProps = {
  language: string;
  ui: Pick<WorkbenchCopy, "previousPage" | "nextPage" | "clearFilters" | "exportData">;
  title: string;
  countLabel: string;
  emptyLabel: string;
  sessionLabel: string;
  windowLabel: string;
  sourceLabel: string;
  riskLabel: string;
  statusLabel: string;
  actionLabel: string;
  summaryTitle: string;
  summaryRows: Array<{ label: string; value: string }>;
  trendTitle: string;
  trendEmptyLabel: string;
  trendBars: Array<{ key: string; label: string; value: string; ratio: number }>;
  sourceStatusTitle: string;
  sourceStatusFacets: Array<{ key: string; label: string; value: string }>;
  studyFacetTitle: string;
  projectFacetTitle: string;
  modelVersionFacetTitle: string;
  facetEmptyLabel: string;
  studyFacets: Array<{ key: string; label: string; value: string }>;
  projectFacets: Array<{ key: string; label: string; value: string }>;
  modelVersionFacets: Array<{ key: string; label: string; value: string }>;
  refreshLabel: string;
  exportLabel: string;
  exportCsvLabel: string;
  windowValue: string;
  sourceValue: string;
  riskValue: string;
  statusValue: string;
  actionValue: string;
  windowOptions: Array<{ value: string; label: string }>;
  sourceOptions: Array<{ value: string; label: string }>;
  riskOptions: Array<{ value: string; label: string }>;
  statusOptions: Array<{ value: string; label: string }>;
  onWindowChange: (value: string) => void;
  onSourceChange: (value: string) => void;
  onRiskChange: (value: string) => void;
  onStatusChange: (value: string) => void;
  onActionChange: (value: string) => void;
  onRefresh: () => void;
  onExport: () => void;
  onExportCsv: () => void;
  entries: SecurityAuditEntryRow[];
};

export const WorkbenchSecurityAuditCard = memo(function WorkbenchSecurityAuditCard({
  language,
  ui,
  title,
  countLabel,
  emptyLabel,
  sessionLabel,
  windowLabel,
  sourceLabel,
  riskLabel,
  statusLabel,
  actionLabel,
  summaryTitle,
  summaryRows,
  trendTitle,
  trendEmptyLabel,
  trendBars,
  sourceStatusTitle,
  sourceStatusFacets,
  studyFacetTitle,
  projectFacetTitle,
  modelVersionFacetTitle,
  facetEmptyLabel,
  studyFacets,
  projectFacets,
  modelVersionFacets,
  refreshLabel,
  exportLabel,
  exportCsvLabel,
  windowValue,
  sourceValue,
  riskValue,
  statusValue,
  actionValue,
  windowOptions,
  sourceOptions,
  riskOptions,
  statusOptions,
  onWindowChange,
  onSourceChange,
  onRiskChange,
  onStatusChange,
  onActionChange,
  onRefresh,
  onExport,
  onExportCsv,
  entries,
}: WorkbenchSecurityAuditCardProps) {
  const [page, setPage] = useState("events");
  const [listPage, setListPage] = useState(0);
  const compact = getWorkbenchCompactPanelCopy(language);
  const pages = Math.max(1, Math.ceil(entries.length / 10));
  const visiblePage = Math.min(listPage, pages - 1);
  useEffect(() => setListPage(0), [windowValue, sourceValue, riskValue, statusValue, actionValue]);
  const activeFilters = [
    { label: windowLabel, value: windowValue, options: windowOptions },
    { label: sourceLabel, value: sourceValue, options: sourceOptions },
    { label: riskLabel, value: riskValue, options: riskOptions },
    { label: statusLabel, value: statusValue, options: statusOptions },
    { label: actionLabel, value: actionValue, options: [] },
  ].filter((filter) => filter.value);
  return (
    <section className="sidebar-card sidebar-card--compact workbench-audit-card" data-workbench-audit="panel">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{countLabel}</span>
      </div>
      <div className="audit-primary-actions">
        <button className="ghost-button ghost-button--compact" data-workbench-audit-action="refresh" onClick={onRefresh} type="button">{refreshLabel}</button>
        <details className="audit-export-menu">
          <summary className="ghost-button ghost-button--compact" data-workbench-audit-action="exports-toggle">{ui.exportData}</summary>
          <div className="button-row">
            <button className="ghost-button ghost-button--compact" data-workbench-audit-action="export-json" onClick={onExport} type="button">{exportLabel}</button>
            <button className="ghost-button ghost-button--compact" data-workbench-audit-action="export-csv" onClick={onExportCsv} type="button">{exportCsvLabel}</button>
          </div>
        </details>
      </div>
      <WorkbenchSubpanelNav label={title} value={page} onChange={setPage} attribute="data-workbench-audit-page"
        pages={[{ id: "events", label: compact.events }, { id: "filters", label: `${compact.filters} (${activeFilters.length})` },
          { id: "summary", label: summaryTitle }, { id: "facets", label: compact.facets }]} />
      {activeFilters.length ? <div className="workbench-audit-filter-summary" data-workbench-audit="active-filters">
        {activeFilters.map((filter) => <span className="protocol-chip" key={filter.label}>
          {filter.label}: {filter.options.find((option) => option.value === filter.value)?.label ?? filter.value}
        </span>)}
      </div> : null}
      {page === "filters" ? <div data-workbench-audit-content="filters">
      <div className="form-grid compact">
        <label>
          <span>{windowLabel}</span>
          <select data-workbench-audit-filter="window" value={windowValue} onChange={(event) => onWindowChange(event.target.value)}>
            {windowOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{sourceLabel}</span>
          <select data-workbench-audit-filter="source" value={sourceValue} onChange={(event) => onSourceChange(event.target.value)}>
            {sourceOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{riskLabel}</span>
          <select data-workbench-audit-filter="risk" value={riskValue} onChange={(event) => onRiskChange(event.target.value)}>
            {riskOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{statusLabel}</span>
          <select data-workbench-audit-filter="status" value={statusValue} onChange={(event) => onStatusChange(event.target.value)}>
            {statusOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{actionLabel}</span>
          <input data-workbench-audit-filter="action" value={actionValue} onChange={(event) => onActionChange(event.target.value)} />
        </label>
      </div>
      <button className="ghost-button ghost-button--compact" data-workbench-audit-action="clear-filters" type="button"
        disabled={!activeFilters.length} onClick={() => {
          onWindowChange(""); onSourceChange(""); onRiskChange(""); onStatusChange(""); onActionChange("");
        }}>{ui.clearFilters}</button>
      </div> : null}
      {page === "summary" ? <div data-workbench-audit-content="summary">
      <div className="card-section">
        <div className="card-head">
          <h3>{summaryTitle}</h3>
        </div>
        <div className="form-grid compact">
          {summaryRows.map((row) => (
            <label key={row.label}>
              <span>{row.label}</span>
              <strong>{row.value}</strong>
            </label>
          ))}
        </div>
      </div>
      <div className="card-section">
        <div className="card-head">
          <h3>{trendTitle}</h3>
        </div>
        {trendBars.length > 0 ? (
          <div className="audit-trend-list">
            {trendBars.map((bar) => (
              <div className="audit-trend-row" key={bar.key}>
                <div className="audit-trend-meta">
                  <span>{bar.label}</span>
                  <strong>{bar.value}</strong>
                </div>
                <div className="audit-trend-track" aria-hidden="true">
                  <span className="audit-trend-fill" style={{ width: `${Math.max(bar.ratio * 100, 6)}%` }} />
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="card-copy">{trendEmptyLabel}</p>
        )}
      </div>
      </div> : null}
      {page === "facets" ? <div data-workbench-audit-content="facets">
        {[
          { id: "source", title: sourceStatusTitle, facets: sourceStatusFacets },
          { id: "study", title: studyFacetTitle, facets: studyFacets },
          { id: "project", title: projectFacetTitle, facets: projectFacets },
          { id: "version", title: modelVersionFacetTitle, facets: modelVersionFacets },
        ].map((group) => <section className="card-section" key={group.id}>
          <div className="card-head"><h3>{group.title}</h3></div>
          {group.facets.length ? <div className="protocol-chip-row">
            {group.facets.map((facet) => <span className="protocol-chip" key={facet.key}>
              {`${facet.label} · ${facet.value}`}
            </span>)}
          </div> : <p className="card-copy">{facetEmptyLabel}</p>}
        </section>)}
      </div> : null}
      {page === "events" ? <div data-workbench-audit-content="events">
      <p className="card-copy">{sessionLabel}</p>
      <WorkbenchListPager page={visiblePage} pages={pages} previousLabel={ui.previousPage} nextLabel={ui.nextPage} onChange={setListPage} />
      {entries.length === 0 ? (
        <p className="card-copy">{emptyLabel}</p>
      ) : (
        <div className="script-panel__catalog">
          {entries.slice(visiblePage * 10, (visiblePage + 1) * 10).map((entry) => (
            <article className="script-panel__action" key={entry.id} data-workbench-audit-event={entry.id}>
              <details>
                <summary className="audit-event-summary"><strong>{entry.action}</strong><small>{entry.status} · {entry.at}</small></summary>
                <p className="card-copy">{entry.note}</p>
                <div className="script-panel__payload"><span>{entry.source}</span><code>{entry.risk}</code></div>
              </details>
            </article>
          ))}
        </div>
      )}
      </div> : null}
    </section>
  );
});
