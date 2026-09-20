"use client";

import { useRef } from "react";

type Page = { id: string; label: string; disabled?: boolean };

export function WorkbenchSubpanelNav({ label, value, pages, onChange, attribute }: {
  label: string;
  value: string;
  pages: Page[];
  onChange: (page: string) => void;
  attribute: `data-${string}`;
}) {
  const root = useRef<HTMLDivElement>(null);
  return <div ref={root} className="panel-tabs workbench-subpanel-nav" role="group" aria-label={label}>
    {pages.map((page) => <button key={page.id} type="button" disabled={page.disabled}
      className={`panel-tab${value === page.id ? " panel-tab--active" : ""}`}
      {...{ [attribute]: page.id }} aria-pressed={value === page.id} onClick={() => onChange(page.id)}
      onKeyDown={(event) => {
        if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
        event.preventDefault();
        const enabled = pages.filter((entry) => !entry.disabled);
        const current = enabled.findIndex((entry) => entry.id === page.id);
        const next = event.key === "Home" ? 0 : event.key === "End" ? enabled.length - 1
          : (current + (event.key === "ArrowRight" ? 1 : -1) + enabled.length) % enabled.length;
        onChange(enabled[next].id);
        const buttons = root.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)");
        buttons?.[next]?.focus();
      }}>{page.label}</button>)}
  </div>;
}

export function WorkbenchListPager({ page, pages, previousLabel, nextLabel, onChange }: {
  page: number; pages: number; previousLabel: string; nextLabel: string; onChange: (page: number) => void;
}) {
  if (pages <= 1) return null;
  return <div className="workbench-list-pager">
    <button className="ghost-button ghost-button--compact" type="button" disabled={page <= 0}
      data-workbench-list-page="previous" onClick={() => onChange(page - 1)}>{previousLabel}</button>
    <output aria-live="polite">{page + 1} / {pages}</output>
    <button className="ghost-button ghost-button--compact" type="button" disabled={page >= pages - 1}
      data-workbench-list-page="next" onClick={() => onChange(page + 1)}>{nextLabel}</button>
  </div>;
}
