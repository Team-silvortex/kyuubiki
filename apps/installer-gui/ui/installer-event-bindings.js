function publishInstallerActionState(action, status, error) {
  const now = Date.now();
  window.__kyuubikiInstallerLastAction = action;
  window.__kyuubikiInstallerActionStatus = status;
  if (status === "running") {
    window.__kyuubikiInstallerActionStartedAt = now;
  } else {
    window.__kyuubikiInstallerActionSettledAt = now;
  }
  if (status === "completed") {
    window.__kyuubikiInstallerActionCompletedAt = now;
    window.__kyuubikiInstallerLastCompletedAction = action;
  }
  document.dispatchEvent(new CustomEvent("kyuubiki:installer-action", {
    detail: {
      action,
      status,
      error: error ? String(error?.message || error) : null,
    },
  }));
}

export function bindInstallerActionHandlers(actionHandlers) {
  document.addEventListener("click", async (event) => {
    const button = event.target?.closest?.("[data-action]");
    if (!button || button.disabled) return;
    const action = button.dataset.action;
    const handler = actionHandlers[action];
    if (!handler) {
      publishInstallerActionState(action, "missing");
      return;
    }
    publishInstallerActionState(action, "running");
    try {
      await handler();
      publishInstallerActionState(action, "completed");
    } catch (error) {
      publishInstallerActionState(action, "failed", error);
    }
  });
}

export function bindInstallerSidebarTabs() {
  document.querySelectorAll(".sidebar-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      document.querySelectorAll(".sidebar-tab").forEach((item) => item.classList.remove("active"));
      document.querySelectorAll(".panel").forEach((panel) => panel.classList.remove("panel-visible"));
      tab.classList.add("active");
      document.querySelector(`[data-panel="${tab.dataset.tab}"]`)?.classList.add("panel-visible");
    });
  });
}

export function bindInstallerSensitiveFields(ids, fieldIds) {
  fieldIds.forEach((id) => {
    ids(id)?.addEventListener("input", () => {
      ids(id).dataset.configured = "false";
    });
  });
}
