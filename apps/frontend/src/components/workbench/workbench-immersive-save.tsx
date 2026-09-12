"use client";

import { useEffect, useRef, useState } from "react";
import type { WorkbenchCopy } from "./workbench-copy";
import type { WorkbenchScriptInvoker } from "./workbench-script-commit-boundary";

export type ImmersiveModelStorage = {
  projects: Array<{ project_id: string; name: string }>;
  selectedProjectId: string | null;
  selectedModelId: string | null;
  selectedVersionId: string | null;
  loadedModelName: string;
  invoke: WorkbenchScriptInvoker;
};

// The request lock belongs to the viewport, so closing a tool tab cannot submit a second save.
export function useWorkbenchImmersiveSave(storage: ImmersiveModelStorage) {
  const [name, setName] = useState(storage.loadedModelName);
  const [busy, setBusy] = useState(false);
  const [feedback, setFeedback] = useState<{ error?: string; receipt?: Record<string, unknown> } | null>(null);
  const lock = useRef(false), mounted = useRef(true);
  useEffect(() => {
    setName(storage.loadedModelName);
    setFeedback(null);
  }, [storage.loadedModelName, storage.selectedProjectId, storage.selectedModelId]);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  const run = async (action: string, payload: Record<string, unknown>) => {
    if (lock.current) return;
    lock.current = true;
    setBusy(true);
    setFeedback(null);
    try {
      const result = await storage.invoke(action, payload, "");
      if (mounted.current) setFeedback({ receipt: result as Record<string, unknown> });
    } catch (error) {
      if (mounted.current) setFeedback({ error: error instanceof Error ? error.message : "model:save_failed" });
    } finally {
      lock.current = false;
      if (mounted.current) setBusy(false);
    }
  };
  return { ...storage, name, setName, busy, feedback,
    save: (saveAs: boolean) => run(saveAs ? "model/saveAs" : "model/save", { name }),
    selectProject: (projectId: string) => run("project/select", { projectId }),
  };
}

export function WorkbenchImmersiveSave({ controller, t }: {
  controller: ReturnType<typeof useWorkbenchImmersiveSave>; t: WorkbenchCopy;
}) {
  const c = controller;
  return <div className="sidebar-stack immersive-save" data-workbench-immersive-save="panel" aria-busy={c.busy}>
    <div className="form-grid compact">
      <label><span>{t.tabs.projects}</span>
        <select value={c.selectedProjectId ?? ""} disabled={c.busy} data-workbench-immersive-save="project"
          onChange={(event) => void c.selectProject(event.target.value)}>
          <option value="" disabled>{t.projectRequired}</option>
          {c.projects.map((project) => <option key={project.project_id} value={project.project_id}>{project.name}</option>)}
        </select>
      </label>
      <label><span>{t.modelName}</span><input value={c.name} maxLength={256} disabled={c.busy}
        data-workbench-immersive-save="name" onChange={(event) => c.setName(event.target.value)} /></label>
    </div>
    {!c.selectedProjectId ? <p className="card-copy">{t.projectRequired}</p> : null}
    <div className="button-row">
      <button className="ghost-button" data-workbench-immersive-save="save" type="button"
        disabled={c.busy || !c.selectedProjectId || !c.name.trim()} onClick={() => void c.save(false)}>{t.save}</button>
      <button className="ghost-button" data-workbench-immersive-save="save-as" type="button"
        disabled={c.busy || !c.selectedProjectId || !c.name.trim()} onClick={() => void c.save(true)}>{t.saveAs}</button>
    </div>
    <p className="card-copy" role="status" data-workbench-immersive-save="status">
      {c.busy ? t.busy : c.feedback?.error ? `${t.auditStatusOptions.failed}: ${c.feedback.error}`
        : c.feedback?.receipt ? `${t.auditStatusOptions.completed}: ${JSON.stringify(c.feedback.receipt)}` : t.ready}
    </p>
    <dl className="immersive-save__context">
      <dt>{t.modelName}</dt><dd>{c.selectedModelId ?? t.none}</dd>
      <dt>{t.versions}</dt><dd>{c.selectedVersionId ?? t.none}</dd>
    </dl>
  </div>;
}
