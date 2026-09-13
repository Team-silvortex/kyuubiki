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
      activeAction: window.__kyuubikiInstallerActiveAction ?? null,
      error: error ? String(error?.message || error) : null,
    },
  }));
}

export function bindInstallerActionHandlers(actionHandlers) {
  let activeAction = null;
  document.addEventListener("click", async (event) => {
    const button = event.target?.closest?.("[data-action]");
    if (!button || button.disabled) return;
    const action = button.dataset.action;
    const handler = Object.hasOwn(actionHandlers, action) ? actionHandlers[action] : null;
    if (typeof handler !== "function") {
      publishInstallerActionState(action, "missing");
      return;
    }
    if (activeAction !== null) {
      publishInstallerActionState(action, "blocked", `Installer action already running: ${activeAction}`);
      return;
    }
    // Own the request before notifying automation, which may dispatch another click.
    activeAction = action;
    window.__kyuubikiInstallerActiveAction = action;
    const previousBusy = button.getAttribute("aria-busy");
    let status = "completed";
    let failure;
    try {
      button.setAttribute("aria-busy", "true");
      publishInstallerActionState(action, "running");
      await handler();
    } catch (error) {
      status = "failed";
      failure = error;
    } finally {
      activeAction = null;
      window.__kyuubikiInstallerActiveAction = null;
      if (previousBusy === null) button.removeAttribute("aria-busy");
      else button.setAttribute("aria-busy", previousBusy);
    }
    // Completion listeners can start the next action without the old owner blocking it.
    publishInstallerActionState(action, status, failure);
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
