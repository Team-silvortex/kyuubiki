"use client";

import { useEffect, useId, useRef, useState } from "react";
import type { WorkbenchCopy } from "./workbench-copy";
import { WorkbenchImmersiveSave, type useWorkbenchImmersiveSave } from "./workbench-immersive-save";

type Props = {
  t: WorkbenchCopy;
  save: ReturnType<typeof useWorkbenchImmersiveSave>;
  execution: ReturnType<typeof useWorkbenchResearchExecution>;
  activePage: string;
  jobActive: boolean;
  hasResult: boolean;
  onStudy: () => void;
  onModel: () => void;
  onResult: () => void | Promise<void>;
};

// Keep in-flight state on the viewport owner, not on the conditionally mounted toolbar.
export function useWorkbenchResearchExecution({ save, pending, jobActive }: {
  save: Props["save"]; pending: boolean; jobActive: boolean;
}) {
  const [dispatching, setDispatching] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const locks = useRef({ "job/run": false, "job/cancel": false });
  const feedbackSequence = useRef(0);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  const busy = pending || dispatching || cancelling || save.busy;
  const run = async (action: "job/run" | "job/cancel") => {
    // A run observes the job through completion; cancellation must not share its lock.
    if (locks.current[action] || save.busy ||
      (action === "job/run" ? busy || jobActive : !jobActive)) return;
    locks.current[action] = true;
    const sequence = ++feedbackSequence.current;
    const setActive = action === "job/run" ? setDispatching : setCancelling;
    setActive(true);
    setError(null);
    try {
      await save.invoke(action, {}, "");
    } catch (failure) {
      if (mounted.current && sequence === feedbackSequence.current) {
        setError(failure instanceof Error ? failure.message : String(failure));
      }
    } finally {
      locks.current[action] = false;
      if (mounted.current) setActive(false);
    }
  };
  return { busy, dispatching, cancelling, error, run, pending };
}

export function WorkbenchResearchActions({ t, save, execution, activePage,
  jobActive, hasResult, onStudy, onModel, onResult }: Props) {
  const [saveOpen, setSaveOpen] = useState(false);
  const panelId = useId();
  const { busy, dispatching, cancelling, error, run, pending } = execution;
  return <section className="research-actions" data-workbench-research="toolbar" aria-label={t.actions}>
    <div className="research-actions__row">
      <div className="research-actions__steps" role="group" aria-label={t.immersiveStudy}>
        <button type="button" data-workbench-research-action="study" aria-pressed={activePage === "study"}
          onClick={() => { setSaveOpen(false); onStudy(); }}>{t.immersiveStudy}</button>
        <button type="button" data-workbench-research-action="model" aria-pressed={activePage === "studio"}
          onClick={() => { setSaveOpen(false); onModel(); }}>{t.immersiveModel}</button>
        <button type="button" data-workbench-research-action="save" aria-expanded={saveOpen}
          aria-controls={panelId} onClick={() => setSaveOpen((open) => !open)}>{t.save}</button>
      </div>
      <div className="research-actions__steps" role="group" aria-label={t.run}>
        <button type="button" className="research-actions__run" data-workbench-research-action="run"
          disabled={busy || jobActive} aria-busy={pending || dispatching}
          onClick={() => void run("job/run")}>{pending || dispatching || jobActive ? t.running : t.run}</button>
        {jobActive ? <button type="button" data-workbench-research-action="cancel" disabled={cancelling || save.busy}
          aria-busy={cancelling}
          onClick={() => void run("job/cancel")}>{t.cancelJob}</button> : null}
        <button type="button" data-workbench-research-action="result" disabled={!hasResult || busy || jobActive}
          onClick={() => void onResult()}>{t.result}</button>
      </div>
    </div>
    {error ? <p className="research-actions__feedback" role="alert">{error}</p> : null}
    {saveOpen ? <div id={panelId} className="research-actions__save"
      data-workbench-research="save" onKeyDown={(event) => {
        if (event.key === "Escape") {
          setSaveOpen(false);
          event.currentTarget.closest(".research-actions")?.querySelector<HTMLButtonElement>(
            '[data-workbench-research-action="save"]')?.focus();
        }
      }}><WorkbenchImmersiveSave controller={save} t={t} compact /></div> : null}
  </section>;
}
