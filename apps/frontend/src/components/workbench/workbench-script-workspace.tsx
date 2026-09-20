"use client";

import { useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { exitWorkbenchViewportFullscreen, isWorkbenchViewportFullscreen, requestWorkbenchViewportFullscreen, workbenchFullscreenEvents } from "./workbench-fullscreen";
import { getWorkbenchScriptWorkspaceCopy } from "./workbench-script-workspace-copy";

export type WorkbenchScriptPage = "script" | "dsl" | "record" | "inspect" | "catalog" | "headless";
const pageOrder: WorkbenchScriptPage[] = ["script", "dsl", "record", "inspect", "catalog", "headless"];

type Props = {
  language: string;
  activePage: WorkbenchScriptPage;
  onPageChange: (page: WorkbenchScriptPage) => void;
  pages: Record<WorkbenchScriptPage, ReactNode>;
  help: ReactNode;
  notice: ReactNode;
};

export function WorkbenchScriptWorkspace({ language, activePage, onPageChange, pages, help, notice }: Props) {
  const rootRef = useRef<HTMLElement>(null);
  const lastFocus = useRef<HTMLElement | null>(null);
  const id = useId();
  const [expanded, setExpanded] = useState(false);
  const [focusError, setFocusError] = useState<string | null>(null);
  const [visited, setVisited] = useState<WorkbenchScriptPage[]>([activePage]);
  const copy = getWorkbenchScriptWorkspaceCopy(language);
  const labels: Record<WorkbenchScriptPage, string> = { script: "Python", dsl: "DSL", record: copy.record, inspect: copy.inspect, catalog: copy.catalog, headless: copy.headless };

  useEffect(() => {
    setVisited((current) => current.includes(activePage) ? current : [...current, activePage]);
  }, [activePage]);
  useLayoutEffect(() => {
    const previous = lastFocus.current;
    const previousPage = previous?.closest<HTMLElement>("[data-workbench-pwdt-content]");
    if (!previousPage?.hidden || (document.activeElement !== previous && document.activeElement !== document.body)) return;
    const page = rootRef.current?.querySelector<HTMLElement>(`[data-workbench-pwdt-content="${activePage}"]`);
    const next = page?.querySelector<HTMLElement>("textarea, input")
      ?? page?.querySelector<HTMLElement>("button:not(:disabled)")
      ?? rootRef.current?.querySelector<HTMLElement>(`[data-workbench-pwdt-page="${activePage}"]`);
    next?.focus({ preventScroll: true });
  }, [activePage]);
  useEffect(() => {
    const target = rootRef.current;
    const sync = () => setExpanded(isWorkbenchViewportFullscreen(document, target));
    for (const event of workbenchFullscreenEvents) document.addEventListener(event, sync);
    return () => {
      for (const event of workbenchFullscreenEvents) document.removeEventListener(event, sync);
      void exitWorkbenchViewportFullscreen(document, target).catch(() => undefined);
    };
  }, []);
  const toggleExpanded = async () => {
    const target = rootRef.current;
    if (!target) return;
    try {
      setFocusError(null);
      if (isWorkbenchViewportFullscreen(document, target)) await exitWorkbenchViewportFullscreen(document, target);
      else await requestWorkbenchViewportFullscreen(document, target);
      setExpanded(isWorkbenchViewportFullscreen(document, target));
    } catch (error) {
      setFocusError(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <section ref={rootRef} className="pwdt-workspace" data-workbench-pwdt="workspace" data-expanded={expanded} aria-label="PWDT"
      onFocusCapture={(event) => { lastFocus.current = event.target; }}>
      <div className="pwdt-workspace__head">
        <strong>PWDT</strong>
        <button className="ghost-button ghost-button--compact" data-workbench-pwdt-expand="true" aria-expanded={expanded}
          onClick={() => void toggleExpanded()} type="button">{expanded ? copy.restore : copy.expand}</button>
      </div>
      <details className="pwdt-workspace__help"><summary>{copy.help}</summary>{help}</details>
      <div className="pwdt-workspace__notice">{notice}</div>
      {focusError ? <p className="card-copy" role="alert">{focusError}</p> : null}
      <div className="panel-tabs pwdt-workspace__tabs" role="tablist" aria-label="PWDT">
        {pageOrder.map((page, index) => (
          <button key={page} type="button" role="tab" id={`${id}-tab-${page}`} aria-controls={`${id}-page-${page}`}
            aria-selected={activePage === page} tabIndex={activePage === page ? 0 : -1}
            className={`panel-tab${activePage === page ? " panel-tab--active" : ""}`} data-workbench-pwdt-page={page}
            onClick={() => onPageChange(page)} onKeyDown={(event) => {
              const target = event.key === "ArrowRight" ? (index + 1) % pageOrder.length
                : event.key === "ArrowLeft" ? (index + pageOrder.length - 1) % pageOrder.length
                  : event.key === "Home" ? 0 : event.key === "End" ? pageOrder.length - 1 : null;
              if (target === null) return;
              event.preventDefault();
              onPageChange(pageOrder[target]);
              rootRef.current?.querySelector<HTMLButtonElement>(`[data-workbench-pwdt-page="${pageOrder[target]}"]`)?.focus();
            }}>{labels[page]}</button>
        ))}
      </div>
      {pageOrder.map((page) => (
        <div key={page} id={`${id}-page-${page}`} role="tabpanel" aria-labelledby={`${id}-tab-${page}`}
          className="pwdt-workspace__page" data-workbench-pwdt-content={page} hidden={activePage !== page}>
          {activePage === page || visited.includes(page) ? pages[page] : null}
        </div>
      ))}
    </section>
  );
}
