export function createHubShellPanel(context) {
  function activateStreamChunk(...ids) {
    context.streamingRuntime?.()?.activateOnly(ids.filter(Boolean), { group: "main" });
  }

  function setSection(section) {
    const next = context.hubCopy().sections[section];
    if (!next) {
      return;
    }

    context.state.activeSection = section;
    context.elements.title.textContent = next.title;
    context.elements.copy.textContent = next.copy;

    context.elements.navItems.forEach((item) => {
      const active = item.dataset.target === section;
      item.classList.toggle("hub-nav__item--active", active);
      item.setAttribute("aria-current", active ? "page" : "false");
    });

    context.elements.panels.forEach((panel) => {
      const hidden = panel.id !== `${section}-panel`;
      panel.classList.toggle("hidden", hidden);
      panel.setAttribute("aria-hidden", String(hidden));
    });

    const defaultProjectsPanel = document.getElementById("projects-panel");
    if (defaultProjectsPanel) {
      defaultProjectsPanel.classList.toggle("hidden", section !== "projects");
    }

    if (section === "projects") {
      renderProjectsPages();
      activateStreamChunk(`section:${section}`, `projects:${context.state.projectsPage}`);
    } else if (section in context.state.panelPages) {
      renderPanelPages(section);
      activateStreamChunk(`section:${section}`, `panel:${section}:${context.state.panelPages[section]}`);
    } else {
      activateStreamChunk(`section:${section}`);
    }

    context.renderAssistantContext();
    context.renderHubAssistantLocalCards();
    context.syncHotRuntimeLogPolling();
    context.syncObserveRuntimeLogPolling();
    if (section === "runtimes") {
      void context.refreshHotRuntimeLog({ silent: true });
    }
    if (section === "observe") {
      void context.refreshObserveRuntimeLog({ silent: true });
    }
  }

  function enhanceHubAccessibility() {
    context.elements.title?.setAttribute("tabindex", "-1");

    context.elements.navItems.forEach((item) => {
      const target = item.dataset.target || "";
      item.setAttribute("aria-controls", `${target}-panel`);
    });

    context.elements.sectionJumpButtons.forEach((button) => {
      const target = button.dataset.targetSection || "";
      button.setAttribute("aria-controls", `${target}-panel`);
    });

    context.elements.projectsPageButtons.forEach((button) => {
      const target = button.dataset.projectsPage || "";
      const pane = context.elements.projectsPanes.find((candidate) => candidate.dataset.projectsPane === target);
      if (!pane) {
        return;
      }

      if (!pane.id) {
        pane.id = `projects-pane-${target}`;
      }
      button.setAttribute("aria-controls", pane.id);
    });

    context.elements.panelPageButtons.forEach((button) => {
      const group = button.dataset.panelPageGroup || "";
      const target = button.dataset.panelPage || "";
      const pane = context.elements.panelPanes.find(
        (candidate) => candidate.dataset.panelPaneGroup === group && candidate.dataset.panelPane === target,
      );
      if (!pane) {
        return;
      }

      if (!pane.id) {
        pane.id = `panel-pane-${group}-${target}`;
      }
      button.setAttribute("aria-controls", pane.id);
    });

    if (context.elements.assistantPanel && !context.elements.assistantPanel.id) {
      context.elements.assistantPanel.id = "hub-assistant-panel";
    }
    if (context.elements.assistantFab && context.elements.assistantPanel) {
      context.elements.assistantFab.setAttribute("aria-controls", context.elements.assistantPanel.id);
    }

    context.elements.densityToggleButtons.forEach((button) => {
      const densityId = button.dataset.densityToggle || "";
      const panel = context.elements.densityPanels.find((candidate) => candidate.dataset.densityPanel === densityId);
      if (!panel) {
        return;
      }

      if (!panel.id) {
        panel.id = `density-panel-${densityId}`;
      }
      button.setAttribute("aria-controls", panel.id);
    });
  }

  function renderProjectsPages() {
    context.elements.projectsPageButtons.forEach((button) => {
      const active = button.dataset.projectsPage === context.state.projectsPage;
      button.classList.toggle("hub-panel-tab--active", active);
      button.setAttribute("aria-pressed", String(active));
    });

    context.elements.projectsPanes.forEach((pane) => {
      const active = pane.dataset.projectsPane === context.state.projectsPage;
      pane.classList.toggle("hidden", !active);
      pane.setAttribute("aria-hidden", String(!active));
    });
  }

  function setProjectsPage(page) {
    context.state.projectsPage =
      page === "library" || page === "bundles" || page === "guides" ? page : "start";
    renderProjectsPages();
    if (context.state.activeSection === "projects") {
      activateStreamChunk("section:projects", `projects:${context.state.projectsPage}`);
    }
    if (
      context.state.projectsPage === "library"
      && !context.state.workflowCatalog.length
      && !context.state.workflowCatalogBusy
    ) {
      void context.fetchWorkflowCatalog({ silent: true });
    }
  }

  function renderPanelPages(group) {
    const activePage = context.state.panelPages[group];
    context.elements.panelPageButtons
      .filter((button) => button.dataset.panelPageGroup === group)
      .forEach((button) => {
        const active = button.dataset.panelPage === activePage;
        button.classList.toggle("hub-panel-tab--active", active);
        button.setAttribute("aria-pressed", String(active));
      });

    context.elements.panelPanes
      .filter((pane) => pane.dataset.panelPaneGroup === group)
      .forEach((pane) => {
        const active = pane.dataset.panelPane === activePage;
        pane.classList.toggle("hidden", !active);
        pane.setAttribute("aria-hidden", String(!active));
      });
  }

  function setPanelPage(group, page) {
    if (!(group in context.state.panelPages)) {
      return;
    }
    context.state.panelPages[group] = page || context.state.panelPages[group];
    renderPanelPages(group);
    if (context.state.activeSection === group) {
      activateStreamChunk(`section:${group}`, `panel:${group}:${context.state.panelPages[group]}`);
    }
  }

  function renderHubDensityToggles() {
    context.elements.densityPanels.forEach((panel) => {
      const densityId = panel.dataset.densityPanel || "";
      const expanded = context.state.density[densityId] !== false;
      panel.classList.toggle("hidden", !expanded);
    });

    context.elements.densityToggleButtons.forEach((button) => {
      const densityId = button.dataset.densityToggle || "";
      const expanded = context.state.density[densityId] !== false;
      button.textContent = expanded ? "Collapse" : "Expand";
      button.setAttribute("aria-expanded", String(expanded));
    });
  }

  function toggleHubDensityPanel(id) {
    if (!(id in context.densityDefaults)) {
      return;
    }

    context.state.density[id] = !(context.state.density[id] !== false);
    context.persistHubDensitySettings();
    renderHubDensityToggles();
  }

  return {
    enhanceHubAccessibility,
    renderHubDensityToggles,
    renderPanelPages,
    renderProjectsPages,
    setPanelPage,
    setProjectsPage,
    setSection,
    toggleHubDensityPanel,
  };
}
