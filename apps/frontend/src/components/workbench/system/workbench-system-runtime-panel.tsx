"use client";

import { memo, type ReactNode } from "react";

import { WorkbenchProtocolAgentsCard } from "@/components/workbench/system/workbench-protocol-agents-card";
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
  protocolAgentsTitle: string;
  protocolAgentsCountLabel: string;
  protocolAgentsEmptyLabel: string;
  protocolAgents: ProtocolAgentCardRow[];
  watchdogTitle: string;
  watchdogStatus: ReactNode;
  watchdogRows: MetricRow[];
};

export const WorkbenchSystemRuntimePanel = memo(function WorkbenchSystemRuntimePanel({
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
  protocolAgentsTitle,
  protocolAgentsCountLabel,
  protocolAgentsEmptyLabel,
  protocolAgents,
  watchdogTitle,
  watchdogStatus,
  watchdogRows,
}: WorkbenchSystemRuntimePanelProps) {
  return (
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
      <WorkbenchSystemMetricsCard
        title={securityTitle}
        status={securityStatus}
        rows={securityRows}
        footer={securityFooter}
      />
      <WorkbenchProtocolAgentsCard
        title={protocolAgentsTitle}
        countLabel={protocolAgentsCountLabel}
        emptyLabel={protocolAgentsEmptyLabel}
        agents={protocolAgents}
      />
      <WorkbenchSystemMetricsCard title={watchdogTitle} status={watchdogStatus} rows={watchdogRows} />
    </>
  );
});
