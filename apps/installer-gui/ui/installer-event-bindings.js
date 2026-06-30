export function bindInstallerActionHandlers(actionHandlers) {
  document.addEventListener("click", async (event) => {
    const button = event.target?.closest?.("[data-action]");
    if (!button) return;
    const handler = actionHandlers[button.dataset.action];
    if (handler) await handler();
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
