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
  entries: SecurityAuditEntryRow[];
};

export const WorkbenchSecurityAuditCard = memo(function WorkbenchSecurityAuditCard({
  title,
  countLabel,
  emptyLabel,
  sessionLabel,
  entries,
}: WorkbenchSecurityAuditCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{countLabel}</span>
      </div>
      <p className="card-copy">{sessionLabel}</p>
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
