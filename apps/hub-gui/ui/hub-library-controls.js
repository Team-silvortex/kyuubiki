export function bindHubLibraryControls(params) {
  const {
    elements,
    state,
    renderHubWorkloadLibrary,
    setWorkloadLibraryOutput,
    renderWorkflowCatalog,
    setWorkflowCatalogOutput,
    localizedWorkflowCatalogLabel,
  } = params;

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
    renderWorkflowCatalog();
  });

  elements.workloadLibrarySearch?.addEventListener("input", () => {
    renderHubWorkloadLibrary();
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
