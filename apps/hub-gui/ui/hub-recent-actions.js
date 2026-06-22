export function bindHubRecentActionControls(params) {
  const {
    elements,
    state,
    renderHubRecents,
    setProjectBundleOutput,
    loadHubRecents,
    saveHubRecents,
    mergeProjectActionHistory,
    formatHubOperatorError,
  } = params;

  function downloadHubJson(filename, payload) {
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
    const url = window.URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    window.URL.revokeObjectURL(url);
  }

  function matchesHistoryFilter(entry) {
    switch (state.historyFilter) {
      case "failed":
        return entry.status === "failed";
      case "inspect":
        return entry.action === "project inspect";
      case "normalize":
        return entry.action === "project normalize";
      case "diff":
        return entry.action === "project diff";
      case "all":
      default:
        return true;
    }
  }

  function currentFilteredHistoryActions(actions = loadHubRecents().actions ?? []) {
    return actions.filter((entry) => matchesHistoryFilter(entry));
  }

  function exportRecentActionHistory() {
    const recents = loadHubRecents();
    const actions = currentFilteredHistoryActions(recents.actions ?? []);
    const payload = {
      exportedAt: new Date().toISOString(),
      filter: state.historyFilter,
      actionCount: actions.length,
      actions,
    };

    downloadHubJson(`kyuubiki-hub-recent-actions-${state.historyFilter}.json`, payload);
    setProjectBundleOutput(`exported ${actions.length} recent actions as JSON`);
  }

  async function importRecentActionHistory(file) {
    if (!file) {
      return;
    }

    const raw = await file.text();
    const parsed = JSON.parse(raw);
    const importedActions = Array.isArray(parsed?.actions) ? parsed.actions : [];
    const recents = loadHubRecents();
    recents.actions = mergeProjectActionHistory(recents.actions ?? [], importedActions);
    saveHubRecents(recents);
    setProjectBundleOutput(`imported ${recents.actions.length} recent actions from JSON`);
  }

  function manageRecentActionHistory(mode) {
    const recents = loadHubRecents();

    switch (mode) {
      case "keep-failed":
        recents.actions = (recents.actions ?? []).filter((entry) => entry.status === "failed");
        saveHubRecents(recents);
        setProjectBundleOutput("kept failed recent actions only");
        return;
      case "import-json":
        elements.historyImportInput?.click();
        return;
      case "clear":
        recents.actions = [];
        saveHubRecents(recents);
        setProjectBundleOutput("cleared recent action history");
        return;
      case "export-json":
        exportRecentActionHistory();
        return;
      default:
        return;
    }
  }

  elements.historyFilterButtons.forEach((button) => {
    button.addEventListener("click", () => {
      state.historyFilter = button.dataset.historyFilter || "all";
      renderHubRecents();
      setProjectBundleOutput(`filtered recent actions: ${state.historyFilter}`);
    });
  });

  elements.historyManageButtons.forEach((button) => {
    button.addEventListener("click", () => {
      manageRecentActionHistory(button.dataset.historyManage || "");
    });
  });

  elements.historyImportInput?.addEventListener("change", async (event) => {
    const input = event.currentTarget;
    const file = input?.files?.[0];

    try {
      await importRecentActionHistory(file);
    } catch (error) {
      setProjectBundleOutput(formatHubOperatorError(error, {
        actionLabel: "Importing recent action history",
      }));
    } finally {
      if (input) {
        input.value = "";
      }
    }
  });
}
