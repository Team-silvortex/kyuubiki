"use client";

import type { ReactNode } from "react";

type WorkbenchSystemOverviewCardProps = {
  title: string;
  status?: string;
  className?: string;
  hint?: ReactNode;
  children?: ReactNode;
  actions?: ReactNode;
};

export function WorkbenchSystemOverviewCard({
  title,
  status,
  className,
  hint,
  children,
  actions,
}: WorkbenchSystemOverviewCardProps) {
  return (
    <section className={`sidebar-card sidebar-card--compact${className ? ` ${className}` : ""}`}>
      <div className="card-head">
        <h2>{title}</h2>
        {status ? <span>{status}</span> : null}
      </div>
      {hint ? <div className="card-copy">{hint}</div> : null}
      {children}
      {actions ? <div className="button-row" style={{ flexWrap: "wrap" }}>{actions}</div> : null}
    </section>
  );
}
