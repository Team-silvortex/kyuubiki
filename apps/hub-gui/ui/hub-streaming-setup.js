import { createHubStreamingRuntime } from "./hub-streaming-runtime.js";

export function setupHubStreamingRuntime({
  elements,
  fetchWorkflowCatalog,
  refreshHotRuntimeLog,
  refreshObserveRuntimeLog,
  setEventMessage,
  state,
}) {
  const runtime = createHubStreamingRuntime({ setEventMessage });

  elements.panels.forEach((panel) => {
    const section = panel.id?.replace(/-panel$/, "");
    if (!section) {
      return;
    }
    runtime.registerChunk(`section:${section}`, {
      nodes: panel,
      retainMs: section === "projects" ? 90000 : 30000,
    });
  });

  elements.projectsPanes.forEach((pane) => {
    const page = pane.dataset.projectsPane || "start";
    runtime.registerChunk(`projects:${page}`, {
      nodes: pane,
      retainMs: page === "start" ? 90000 : 30000,
      onHydrate: () => {
        if (page === "library" && !state.workflowCatalog.length && !state.workflowCatalogBusy) {
          void fetchWorkflowCatalog({ silent: true });
        }
      },
      onRelease: () => {
        if (page === "library" && elements.workflowCatalogOutput) {
          elements.workflowCatalogOutput.textContent = "";
        }
      },
    });
  });

  elements.panelPanes.forEach((pane) => {
    const group = pane.dataset.panelPaneGroup || "";
    const page = pane.dataset.panelPane || "";
    if (!group || !page) {
      return;
    }
    runtime.registerChunk(`panel:${group}:${page}`, {
      nodes: pane,
      retainMs: group === "observe" || group === "runtimes" ? 20000 : 30000,
      onHydrate: () => {
        if (group === "runtimes" && page === "hot") {
          void refreshHotRuntimeLog({ silent: true });
        }
        if (group === "observe" && page === "stack") {
          void refreshObserveRuntimeLog({ silent: true });
        }
      },
    });
  });

  runtime.registerChunk("overlay:assistant", {
    nodes: elements.assistantPanel,
    group: "overlay",
    retainMs: 15000,
  });

  return runtime;
}
