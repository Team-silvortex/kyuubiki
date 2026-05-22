"use client";

import { memo, useState, type ReactNode } from "react";

import { WorkbenchProtocolAgentsCard } from "@/components/workbench/system/workbench-protocol-agents-card";
import { WorkbenchSecurityAuditCard } from "@/components/workbench/system/workbench-security-audit-card";
import { WorkbenchSystemMetricsCard } from "@/components/workbench/system/workbench-system-metrics-card";

type MetricRow = {
  label: string;
  value: ReactNode;
};

type ProtocolAgentMetric = {
  label: string;
  value: string | number;
  tone?: string;
};

type ProtocolAgentChip = {
  key: string;
  label: string;
  tone?: string;
  title?: string;
};

type ProtocolAgentCardRow = {
  id: string;
  endpoint: string;
  metrics: ProtocolAgentMetric[];
  chips: ProtocolAgentChip[];
  error?: string;
};

type WorkbenchSystemRuntimePanelProps = {
  overviewTabLabel: string;
  stackTabLabel: string;
  securityTabLabel: string;
  agentsTabLabel: string;
  auditTabLabel: string;
  watchdogTabLabel: string;
  backendTitle: string;
  backendStatus: ReactNode;
  backendRows: MetricRow[];
  protocolsTitle: string;
  protocolsStatus: ReactNode;
  protocolRows: MetricRow[];
  protocolMethods?: string[];
  securityTitle: string;
  securityStatus: ReactNode;
  securityRows: MetricRow[];
  securityFooter: ReactNode;
  auditTitle: string;
  auditCountLabel: string;
  auditEmptyLabel: string;
  auditSessionLabel: string;
  auditWindowLabel: string;
  auditSourceLabel: string;
  auditRiskLabel: string;
  auditStatusLabel: string;
  auditActionLabel: string;
  auditSummaryTitle: string;
  auditSummaryRows: Array<{ label: string; value: string }>;
  auditTrendTitle: string;
  auditTrendEmptyLabel: string;
  auditTrendBars: Array<{ key: string; label: string; value: string; ratio: number }>;
  auditSourceStatusTitle: string;
  auditSourceStatusFacets: Array<{ key: string; label: string; value: string }>;
  auditStudyFacetTitle: string;
  auditProjectFacetTitle: string;
  auditModelVersionFacetTitle: string;
  auditFacetEmptyLabel: string;
  auditStudyFacets: Array<{ key: string; label: string; value: string }>;
  auditProjectFacets: Array<{ key: string; label: string; value: string }>;
  auditModelVersionFacets: Array<{ key: string; label: string; value: string }>;
  auditRefreshLabel: string;
  auditExportLabel: string;
  auditExportCsvLabel: string;
  auditWindowValue: string;
  auditSourceValue: string;
  auditRiskValue: string;
  auditStatusValue: string;
  auditActionValue: string;
  auditWindowOptions: Array<{ value: string; label: string }>;
  auditSourceOptions: Array<{ value: string; label: string }>;
  auditRiskOptions: Array<{ value: string; label: string }>;
  auditStatusOptions: Array<{ value: string; label: string }>;
  onAuditWindowChange: (value: string) => void;
  onAuditSourceChange: (value: string) => void;
  onAuditRiskChange: (value: string) => void;
  onAuditStatusChange: (value: string) => void;
  onAuditActionChange: (value: string) => void;
  onAuditRefresh: () => void;
  onAuditExport: () => void;
  onAuditExportCsv: () => void;
  auditEntries: Array<{
    id: string;
    at: string;
    action: string;
    source: string;
    risk: string;
    status: string;
    note: string;
  }>;
  protocolAgentsTitle: string;
  protocolAgentsCountLabel: string;
  protocolAgentsEmptyLabel: string;
  protocolAgents: ProtocolAgentCardRow[];
  watchdogTitle: string;
  watchdogStatus: ReactNode;
  watchdogRows: MetricRow[];
};

export const WorkbenchSystemRuntimePanel = memo(function WorkbenchSystemRuntimePanel({
  overviewTabLabel,
  stackTabLabel,
  securityTabLabel,
  agentsTabLabel,
  auditTabLabel,
  watchdogTabLabel,
  backendTitle,
  backendStatus,
  backendRows,
  protocolsTitle,
  protocolsStatus,
  protocolRows,
  protocolMethods,
  securityTitle,
  securityStatus,
  securityRows,
  securityFooter,
  auditTitle,
  auditCountLabel,
  auditEmptyLabel,
  auditSessionLabel,
  auditWindowLabel,
  auditSourceLabel,
  auditRiskLabel,
  auditStatusLabel,
  auditActionLabel,
  auditSummaryTitle,
  auditSummaryRows,
  auditTrendTitle,
  auditTrendEmptyLabel,
  auditTrendBars,
  auditSourceStatusTitle,
  auditSourceStatusFacets,
  auditStudyFacetTitle,
  auditProjectFacetTitle,
  auditModelVersionFacetTitle,
  auditFacetEmptyLabel,
  auditStudyFacets,
  auditProjectFacets,
  auditModelVersionFacets,
  auditRefreshLabel,
  auditExportLabel,
  auditExportCsvLabel,
  auditWindowValue,
  auditSourceValue,
  auditRiskValue,
  auditStatusValue,
  auditActionValue,
  auditWindowOptions,
  auditSourceOptions,
  auditRiskOptions,
  auditStatusOptions,
  onAuditWindowChange,
  onAuditSourceChange,
  onAuditRiskChange,
  onAuditStatusChange,
  onAuditActionChange,
  onAuditRefresh,
  onAuditExport,
  onAuditExportCsv,
  auditEntries,
  protocolAgentsTitle,
  protocolAgentsCountLabel,
  protocolAgentsEmptyLabel,
  protocolAgents,
  watchdogTitle,
  watchdogStatus,
  watchdogRows,
}: WorkbenchSystemRuntimePanelProps) {
  const [page, setPage] = useState<"overview" | "stack" | "security" | "agents" | "audit" | "watchdog">("overview");

  return (
    <>
      <div className="panel-tabs panel-tabs--wide">
        <button className={`panel-tab${page === "overview" ? " panel-tab--active" : ""}`} onClick={() => setPage("overview")} type="button">
          {overviewTabLabel}
        </button>
        <button className={`panel-tab${page === "stack" ? " panel-tab--active" : ""}`} onClick={() => setPage("stack")} type="button">
          {stackTabLabel}
        </button>
        <button className={`panel-tab${page === "security" ? " panel-tab--active" : ""}`} onClick={() => setPage("security")} type="button">
          {securityTabLabel}
        </button>
        <button className={`panel-tab${page === "agents" ? " panel-tab--active" : ""}`} onClick={() => setPage("agents")} type="button">
          {agentsTabLabel}
        </button>
        <button className={`panel-tab${page === "audit" ? " panel-tab--active" : ""}`} onClick={() => setPage("audit")} type="button">
          {auditTabLabel}
        </button>
        <button className={`panel-tab${page === "watchdog" ? " panel-tab--active" : ""}`} onClick={() => setPage("watchdog")} type="button">
          {watchdogTabLabel}
        </button>
      </div>
      {page === "overview" ? (
        <div className="runtime-overview-grid">
          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{stackTabLabel}</h2>
              <span>{backendStatus}</span>
            </div>
            <div className="sidebar-list sidebar-list--metrics">
              {backendRows.slice(0, 3).map((row) => (
                <div className="sidebar-list__row" key={`backend-${row.label}`}>
                  <span>{row.label}</span>
                  <strong>{row.value}</strong>
                </div>
              ))}
            </div>
            <div className="button-row">
              <button onClick={() => setPage("stack")} type="button">
                {stackTabLabel}
              </button>
            </div>
          </section>

          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{securityTabLabel}</h2>
              <span>{securityStatus}</span>
            </div>
            <div className="sidebar-list sidebar-list--metrics">
              {securityRows.slice(0, 3).map((row) => (
                <div className="sidebar-list__row" key={`security-${row.label}`}>
                  <span>{row.label}</span>
                  <strong>{row.value}</strong>
                </div>
              ))}
            </div>
            <div className="button-row">
              <button onClick={() => setPage("security")} type="button">
                {securityTabLabel}
              </button>
            </div>
          </section>

          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{agentsTabLabel}</h2>
              <span>{protocolAgentsCountLabel}</span>
            </div>
            {protocolAgents.length > 0 ? (
              <div className="sidebar-list sidebar-list--metrics">
                {protocolAgents.slice(0, 2).map((agent) => (
                  <div className="sidebar-list__row" key={`agent-${agent.id}`}>
                    <span>{agent.id}</span>
                    <strong>{agent.endpoint}</strong>
                  </div>
                ))}
              </div>
            ) : (
              <p className="card-copy">{protocolAgentsEmptyLabel}</p>
            )}
            <div className="button-row">
              <button onClick={() => setPage("agents")} type="button">
                {agentsTabLabel}
              </button>
            </div>
          </section>

          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{auditTabLabel}</h2>
              <span>{auditCountLabel}</span>
            </div>
            <div className="sidebar-list sidebar-list--metrics">
              {auditSummaryRows.slice(0, 3).map((row) => (
                <div className="sidebar-list__row" key={`audit-${row.label}`}>
                  <span>{row.label}</span>
                  <strong>{row.value}</strong>
                </div>
              ))}
            </div>
            <div className="button-row">
              <button onClick={() => setPage("audit")} type="button">
                {auditTabLabel}
              </button>
            </div>
          </section>

          <section className="sidebar-card sidebar-card--compact runtime-overview-card">
            <div className="card-head">
              <h2>{watchdogTabLabel}</h2>
              <span>{watchdogStatus}</span>
            </div>
            <div className="sidebar-list sidebar-list--metrics">
              {watchdogRows.slice(0, 3).map((row) => (
                <div className="sidebar-list__row" key={`watchdog-${row.label}`}>
                  <span>{row.label}</span>
                  <strong>{row.value}</strong>
                </div>
              ))}
            </div>
            <div className="button-row">
              <button onClick={() => setPage("watchdog")} type="button">
                {watchdogTabLabel}
              </button>
            </div>
          </section>
        </div>
      ) : null}
      {page === "stack" ? (
        <>
          <WorkbenchSystemMetricsCard title={backendTitle} status={backendStatus} rows={backendRows} />
          <WorkbenchSystemMetricsCard
            title={protocolsTitle}
            status={protocolsStatus}
            rows={protocolRows}
            extra={
              protocolMethods?.length ? (
                <div className="protocol-chip-row">
                  {protocolMethods.map((method) => (
                    <span className="protocol-chip" key={method}>
                      {method}
                    </span>
                  ))}
                </div>
              ) : null
            }
          />
        </>
      ) : null}
      {page === "security" ? (
        <WorkbenchSystemMetricsCard
          title={securityTitle}
          status={securityStatus}
          rows={securityRows}
          footer={securityFooter}
        />
      ) : null}
      {page === "audit" ? (
        <WorkbenchSecurityAuditCard
          title={auditTitle}
          countLabel={auditCountLabel}
          emptyLabel={auditEmptyLabel}
          sessionLabel={auditSessionLabel}
          windowLabel={auditWindowLabel}
          sourceLabel={auditSourceLabel}
          riskLabel={auditRiskLabel}
          statusLabel={auditStatusLabel}
          actionLabel={auditActionLabel}
          summaryTitle={auditSummaryTitle}
          summaryRows={auditSummaryRows}
          trendTitle={auditTrendTitle}
          trendEmptyLabel={auditTrendEmptyLabel}
          trendBars={auditTrendBars}
          sourceStatusTitle={auditSourceStatusTitle}
          sourceStatusFacets={auditSourceStatusFacets}
          studyFacetTitle={auditStudyFacetTitle}
          projectFacetTitle={auditProjectFacetTitle}
          modelVersionFacetTitle={auditModelVersionFacetTitle}
          facetEmptyLabel={auditFacetEmptyLabel}
          studyFacets={auditStudyFacets}
          projectFacets={auditProjectFacets}
          modelVersionFacets={auditModelVersionFacets}
          refreshLabel={auditRefreshLabel}
          exportLabel={auditExportLabel}
          exportCsvLabel={auditExportCsvLabel}
          windowValue={auditWindowValue}
          sourceValue={auditSourceValue}
          riskValue={auditRiskValue}
          statusValue={auditStatusValue}
          actionValue={auditActionValue}
          windowOptions={auditWindowOptions}
          sourceOptions={auditSourceOptions}
          riskOptions={auditRiskOptions}
          statusOptions={auditStatusOptions}
          onWindowChange={onAuditWindowChange}
          onSourceChange={onAuditSourceChange}
          onRiskChange={onAuditRiskChange}
          onStatusChange={onAuditStatusChange}
          onActionChange={onAuditActionChange}
          onRefresh={onAuditRefresh}
          onExport={onAuditExport}
          onExportCsv={onAuditExportCsv}
          entries={auditEntries}
        />
      ) : null}
      {page === "agents" ? (
        <WorkbenchProtocolAgentsCard
          title={protocolAgentsTitle}
          countLabel={protocolAgentsCountLabel}
          emptyLabel={protocolAgentsEmptyLabel}
          agents={protocolAgents}
        />
      ) : null}
      {page === "watchdog" ? <WorkbenchSystemMetricsCard title={watchdogTitle} status={watchdogStatus} rows={watchdogRows} /> : null}
    </>
  );
});
