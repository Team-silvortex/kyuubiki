"use client";

import type { ReactNode } from "react";

type MetricRow = {
  label: string;
  value: ReactNode;
};

type WorkbenchSystemMetricsCardProps = {
  title: string;
  status: ReactNode;
  rows: MetricRow[];
  extra?: ReactNode;
  footer?: ReactNode;
};

export function WorkbenchSystemMetricsCard({
  title,
  status,
  rows,
  extra,
  footer,
}: WorkbenchSystemMetricsCardProps) {
  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{title}</h2>
        <span>{status}</span>
      </div>
      <div className="sidebar-list">
        {rows.map((row) => (
          <div key={row.label}>
            <span>{row.label}</span>
            <strong>{row.value}</strong>
          </div>
        ))}
      </div>
      {extra ? extra : null}
      {footer ? footer : null}
    </section>
  );
}
