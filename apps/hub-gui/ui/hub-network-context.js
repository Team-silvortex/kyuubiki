export function createHubNetworkContext({ elements }) {
  function currentOrchestratorBaseUrl() {
    const text = String(elements.orchestratorUrl?.textContent || "").trim();
    return text || "http://127.0.0.1:4000";
  }

  function currentLocalWorkloadCatalogUrl() {
    return `${currentOrchestratorBaseUrl().replace(/\/+$/u, "")}/api/v1/workloads/catalog`;
  }

  function currentWorkflowCatalogUrl() {
    return `${currentOrchestratorBaseUrl().replace(/\/+$/u, "")}/api/v1/workflows/catalog`;
  }

  function ensureDefaultWorkloadCatalogUrl(force = false) {
    if (!elements.workloadCatalogUrl) {
      return "";
    }

    if (!force && String(elements.workloadCatalogUrl.value || "").trim()) {
      return String(elements.workloadCatalogUrl.value || "").trim();
    }

    const next = currentLocalWorkloadCatalogUrl();
    elements.workloadCatalogUrl.value = next;
    return next;
  }

  return {
    currentLocalWorkloadCatalogUrl,
    currentOrchestratorBaseUrl,
    currentWorkflowCatalogUrl,
    ensureDefaultWorkloadCatalogUrl,
  };
}
