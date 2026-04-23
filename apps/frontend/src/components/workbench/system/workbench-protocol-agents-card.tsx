"use client";

import { memo } from "react";

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

type WorkbenchProtocolAgentsCardProps = {
  title: string;
  countLabel: string;
  emptyLabel: string;
  agents: ProtocolAgentCardRow[];
};

export const WorkbenchProtocolAgentsCard = memo(function WorkbenchProtocolAgentsCard({
  title,
  countLabel,
  emptyLabel,
  agents,
}: WorkbenchProtocolAgentsCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{countLabel}</span>
      </div>
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
                <div className="protocol-chip-row">
                  {agent.chips.map((chip) => (
                    <span
                      className={`protocol-chip${chip.tone ? ` protocol-chip--${chip.tone}` : ""}`}
                      key={chip.key}
                      title={chip.title}
                    >
                      {chip.label}
                    </span>
                  ))}
                </div>
              ) : agent.error ? (
                <p className="card-copy">{agent.error}</p>
              ) : null}
            </article>
          ))}
        </div>
      )}
    </section>
  );
});
