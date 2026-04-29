"use client";

import { memo } from "react";

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
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{countLabel}</span>
      </div>
      <p className="card-copy">{sessionLabel}</p>
      <div className="form-grid compact">
        <label>
          <span>{windowLabel}</span>
          <select value={windowValue} onChange={(event) => onWindowChange(event.target.value)}>
            {windowOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{sourceLabel}</span>
          <select value={sourceValue} onChange={(event) => onSourceChange(event.target.value)}>
            {sourceOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{riskLabel}</span>
          <select value={riskValue} onChange={(event) => onRiskChange(event.target.value)}>
            {riskOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{statusLabel}</span>
          <select value={statusValue} onChange={(event) => onStatusChange(event.target.value)}>
            {statusOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </label>
        <label>
          <span>{actionLabel}</span>
          <input value={actionValue} onChange={(event) => onActionChange(event.target.value)} />
        </label>
      </div>
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
      <div className="card-section">
        <div className="card-head">
          <h3>{sourceStatusTitle}</h3>
        </div>
        {sourceStatusFacets.length > 0 ? (
          <div className="protocol-chip-row">
            {sourceStatusFacets.map((facet) => (
              <span className="protocol-chip" key={facet.key}>
                {`${facet.label} · ${facet.value}`}
              </span>
            ))}
          </div>
        ) : (
          <p className="card-copy">{facetEmptyLabel}</p>
        )}
      </div>
      <div className="card-section">
        <div className="card-head">
          <h3>{studyFacetTitle}</h3>
        </div>
        {studyFacets.length > 0 ? (
          <div className="protocol-chip-row">
            {studyFacets.map((facet) => (
              <span className="protocol-chip" key={facet.key}>
                {`${facet.label} · ${facet.value}`}
              </span>
            ))}
          </div>
        ) : (
          <p className="card-copy">{facetEmptyLabel}</p>
        )}
      </div>
      <div className="card-section">
        <div className="card-head">
          <h3>{projectFacetTitle}</h3>
        </div>
        {projectFacets.length > 0 ? (
          <div className="protocol-chip-row">
            {projectFacets.map((facet) => (
              <span className="protocol-chip" key={facet.key}>
                {`${facet.label} · ${facet.value}`}
              </span>
            ))}
          </div>
        ) : (
          <p className="card-copy">{facetEmptyLabel}</p>
        )}
      </div>
      <div className="card-section">
        <div className="card-head">
          <h3>{modelVersionFacetTitle}</h3>
        </div>
        {modelVersionFacets.length > 0 ? (
          <div className="protocol-chip-row">
            {modelVersionFacets.map((facet) => (
              <span className="protocol-chip" key={facet.key}>
                {`${facet.label} · ${facet.value}`}
              </span>
            ))}
          </div>
        ) : (
          <p className="card-copy">{facetEmptyLabel}</p>
        )}
      </div>
      <div className="button-row">
        <button className="ghost-button ghost-button--compact" onClick={onRefresh} type="button">
          {refreshLabel}
        </button>
        <button className="ghost-button ghost-button--compact" onClick={onExport} type="button">
          {exportLabel}
        </button>
        <button className="ghost-button ghost-button--compact" onClick={onExportCsv} type="button">
          {exportCsvLabel}
        </button>
      </div>
      {entries.length === 0 ? (
        <p className="card-copy">{emptyLabel}</p>
      ) : (
        <div className="script-panel__catalog">
          {entries.map((entry) => (
            <article className="script-panel__action" key={entry.id}>
              <div className="script-panel__action-head">
                <strong>{entry.action}</strong>
                <span>{entry.status}</span>
              </div>
              <p className="card-copy">{entry.note}</p>
              <div className="script-panel__payload">
                <span>{entry.source}</span>
                <code>{`${entry.risk} · ${entry.at}`}</code>
              </div>
            </article>
          ))}
        </div>
      )}
    </section>
  );
});
