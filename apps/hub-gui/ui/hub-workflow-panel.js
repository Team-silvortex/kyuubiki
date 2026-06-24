import {
  fetchWorkflowCatalog as fetchWorkflowCatalogPanel,
  renderWorkflowCatalog as renderWorkflowCatalogPanel,
  runWorkflowCatalogSample as runWorkflowCatalogSamplePanel,
  waitForWorkflowJob as waitForWorkflowCatalogJob,
} from "./hub-workflow-catalog.js";

export function createHubWorkflowPanel(context) {
  async function waitForWorkflowJob(jobId, options = {}) {
    return waitForWorkflowCatalogJob(jobId, {
      ...options,
      currentOrchestratorBaseUrl: context.currentOrchestratorBaseUrl,
      hubMessage: context.hubMessage,
      localizedWorkflowCatalogLabel: context.localizedWorkflowCatalogLabel,
      setWorkflowCatalogOutput: context.setWorkflowCatalogOutput,
    });
  }

  async function runWorkflowCatalogSample(entry) {
    return runWorkflowCatalogSamplePanel(entry, {
      state: context.state,
      applyDesktopState: context.applyDesktopState,
      actionState: context.elements.actionState,
      currentWorkflowCatalogUrl: context.currentWorkflowCatalogUrl,
      currentOrchestratorBaseUrl: context.currentOrchestratorBaseUrl,
      hubMessage: context.hubMessage,
      localizedWorkflowCatalogLabel: context.localizedWorkflowCatalogLabel,
      setWorkflowCatalogOutput: context.setWorkflowCatalogOutput,
      setOperationOutput: context.setOperationOutput,
      formatHubOperatorError: context.formatHubOperatorError,
      renderWorkflowCatalog,
    });
  }

  async function fetchWorkflowCatalog(options = {}) {
    return fetchWorkflowCatalogPanel({
      ...options,
      state: context.state,
      renderWorkflowCatalog,
      setWorkflowCatalogOutput: context.setWorkflowCatalogOutput,
      setOperationOutput: context.setOperationOutput,
      applyDesktopState: context.applyDesktopState,
      actionState: context.elements.actionState,
      currentWorkflowCatalogUrl: context.currentWorkflowCatalogUrl,
      hubMessage: context.hubMessage,
      localizedWorkflowCatalogLabel: context.localizedWorkflowCatalogLabel,
      formatHubOperatorError: context.formatHubOperatorError,
    });
  }

  function renderWorkflowCatalog(entries = context.state.workflowCatalog) {
    return renderWorkflowCatalogPanel(entries, {
      workflowCatalogList: context.elements.workflowCatalogList,
      workflowCatalogBusy: context.state.workflowCatalogBusy,
      workflowCatalogQuery: context.elements.workflowCatalogSearch?.value || "",
      renderEmptyHistoryState: context.renderEmptyHistoryState,
      localizedWorkflowCatalogLabel: context.localizedWorkflowCatalogLabel,
      appendTextElement: context.appendTextElement,
      hubMessage: context.hubMessage,
      runWorkflowCatalogSample,
    });
  }

  return {
    fetchWorkflowCatalog,
    renderWorkflowCatalog,
    runWorkflowCatalogSample,
    waitForWorkflowJob,
  };
}
