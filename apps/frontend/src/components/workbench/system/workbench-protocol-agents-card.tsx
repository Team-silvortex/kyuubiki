"use client";

import { memo, useState } from "react";
import { WorkbenchSystemOverviewCard } from "@/components/workbench/system/workbench-system-overview-card";

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
  chipPreviewLimit?: number;
  showMoreLabel: string;
  showLessLabel: string;
  error?: string;
};

type WorkbenchProtocolAgentsCardProps = {
  title: string;
  countLabel: string;
  emptyLabel: string;
  agents: ProtocolAgentCardRow[];
};

function ProtocolAgentChips({ agent }: { agent: ProtocolAgentCardRow }) {
  const [expanded, setExpanded] = useState(false);
  const previewLimit = agent.chipPreviewLimit ?? 10;
  const visibleChips = expanded ? agent.chips : agent.chips.slice(0, previewLimit);
  const hiddenCount = Math.max(0, agent.chips.length - previewLimit);

  return (
    <div className="protocol-chip-row">
      {visibleChips.map((chip) => (
        <span
          className={`protocol-chip${chip.tone ? ` protocol-chip--${chip.tone}` : ""}`}
          key={chip.key}
          title={chip.title}
        >
          {chip.label}
        </span>
      ))}
      {hiddenCount > 0 ? (
        <button
          aria-expanded={expanded}
          className="protocol-agent-card__chip-toggle"
          onClick={() => setExpanded((current) => !current)}
          type="button"
        >
          {expanded ? agent.showLessLabel : `${agent.showMoreLabel} +${hiddenCount}`}
        </button>
      ) : null}
    </div>
  );
}

export const WorkbenchProtocolAgentsCard = memo(function WorkbenchProtocolAgentsCard({
  title,
  countLabel,
  emptyLabel,
  agents,
}: WorkbenchProtocolAgentsCardProps) {
  return (
    <WorkbenchSystemOverviewCard title={title} status={countLabel}>
      {agents.length === 0 ? (
        <p className="card-copy">{emptyLabel}</p>
      ) : (
        <div className="protocol-agent-list">
          {agents.map((agent) => (
            <article className="protocol-agent-card" key={agent.id}>
              <div className="protocol-agent-card__head">
                <strong>{agent.id}</strong>
                <span>{agent.endpoint}</span>
              </div>
              <div className="sidebar-list">
                {agent.metrics.map((metric) => (
                  <div key={`${agent.id}-${metric.label}`}>
                    <span>{metric.label}</span>
                    <strong>
                      {metric.tone ? (
                        <span className={`status-chip status-chip--${metric.tone}`}>{metric.value}</span>
                      ) : (
                        metric.value
                      )}
                    </strong>
                  </div>
                ))}
              </div>
              {agent.chips.length > 0 ? (
                <ProtocolAgentChips agent={agent} />
              ) : agent.error ? (
                <p className="card-copy">{agent.error}</p>
              ) : null}
            </article>
          ))}
        </div>
      )}
    </WorkbenchSystemOverviewCard>
  );
});
