export type WorkbenchProjectSelection = {
  projectId: string | null;
  modelId: string | null;
  versionId: string | null;
};

export type WorkbenchProjectRefresh = (
  bootstrap?: boolean,
  preferredProjectId?: string | null,
  options?: { preserveSelection?: boolean },
) => Promise<void>;

export function createWorkbenchProjectContext(initial: WorkbenchProjectSelection) {
  let selection = { ...initial };
  let revision = 0;
  let mounted = true;
  const capture = () => {
    const captured = mounted ? revision : -1;
    return () => mounted && captured === revision;
  };

  return {
    capture,
    current: () => ({ ...selection }),
    hasModel: (modelId: string) => mounted && selection.modelId === modelId,
    detachDeleted: (kind: keyof WorkbenchProjectSelection, id: string) => {
      if (!mounted || selection[kind] !== id) return null;
      selection = {
        projectId: kind === "projectId" ? null : selection.projectId,
        modelId: kind === "versionId" ? selection.modelId : null,
        versionId: null,
      };
      // Invalidate reads still in flight so they cannot reattach a deleted record.
      revision += 1;
      return { ...selection };
    },
    begin: () => {
      // The latest load/save intent wins even before its selection has committed.
      revision += 1;
      return capture();
    },
    update: (next: WorkbenchProjectSelection) => {
      if (selection.projectId !== next.projectId || selection.modelId !== next.modelId ||
          selection.versionId !== next.versionId) revision += 1;
      selection = { ...next };
    },
    mount: () => { mounted = true; },
    dispose: () => { mounted = false; revision += 1; },
  };
}

export type WorkbenchProjectContext = ReturnType<typeof createWorkbenchProjectContext>;

export function workbenchProjectContextChangedError() {
  return new Error("WORKBENCH_CONTEXT_CHANGED: the workspace changed while the operation was pending; inspect its result before continuing.");
}
