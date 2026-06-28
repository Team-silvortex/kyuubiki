type HubLibraryControlElements = {
  workloadFilterButtons: HTMLElement[];
  workloadFamilyFilterButtons: HTMLElement[];
  workflowCatalogSearch?: HTMLInputElement | null;
  workloadLibrarySearch?: HTMLInputElement | null;
  librarySearchClear?: HTMLElement | null;
  workflowCatalogSearchClear?: HTMLElement | null;
};

type HubLibraryControlState = {
  workloadFilter: string;
  workloadFamilyFilter: string;
};

type HubLibraryControlParams = {
  elements: HubLibraryControlElements;
  state: HubLibraryControlState;
  renderHubWorkloadLibrary: () => void;
  setWorkloadLibraryOutput: (value: string) => void;
  renderWorkflowCatalog: () => void;
  setWorkflowCatalogOutput: (value: string) => void;
  localizedWorkflowCatalogLabel: (key: string) => string;
};

type ScheduledRender = () => void;

export function bindHubLibraryControls(params: HubLibraryControlParams): void {
  const {
    elements,
    state,
    renderHubWorkloadLibrary,
    setWorkloadLibraryOutput,
    renderWorkflowCatalog,
    setWorkflowCatalogOutput,
    localizedWorkflowCatalogLabel,
  } = params;

  const scheduleRender = (render: () => void): ScheduledRender => {
    let frame = 0;
    return () => {
      if (frame) {
        window.cancelAnimationFrame(frame);
      }
      frame = window.requestAnimationFrame(() => {
        frame = 0;
        render();
      });
    };
  };

  const scheduleWorkflowCatalogRender = scheduleRender(renderWorkflowCatalog);
  const scheduleWorkloadLibraryRender = scheduleRender(renderHubWorkloadLibrary);

  elements.workloadFilterButtons.forEach((button) => {
    button.addEventListener("click", () => {
      state.workloadFilter = button.dataset.workloadFilter || "all";
      renderHubWorkloadLibrary();
      setWorkloadLibraryOutput(`filtered workloads: ${state.workloadFilter} / ${state.workloadFamilyFilter}`);
    });
  });

  elements.workloadFamilyFilterButtons.forEach((button) => {
    button.addEventListener("click", () => {
      state.workloadFamilyFilter = button.dataset.workloadFamilyFilter || "all";
      renderHubWorkloadLibrary();
      setWorkloadLibraryOutput(`filtered workloads: ${state.workloadFilter} / ${state.workloadFamilyFilter}`);
    });
  });

  elements.workflowCatalogSearch?.addEventListener("input", () => {
    scheduleWorkflowCatalogRender();
  });

  elements.workloadLibrarySearch?.addEventListener("input", () => {
    scheduleWorkloadLibraryRender();
  });

  elements.librarySearchClear?.addEventListener("click", () => {
    if (elements.workloadLibrarySearch) {
      elements.workloadLibrarySearch.value = "";
    }
    renderHubWorkloadLibrary();
    setWorkloadLibraryOutput(`filtered workloads: ${state.workloadFilter} / ${state.workloadFamilyFilter}`);
  });

  elements.workflowCatalogSearchClear?.addEventListener("click", () => {
    if (elements.workflowCatalogSearch) {
      elements.workflowCatalogSearch.value = "";
    }
    renderWorkflowCatalog();
    setWorkflowCatalogOutput(localizedWorkflowCatalogLabel("workflowCatalogReady"));
  });
}
