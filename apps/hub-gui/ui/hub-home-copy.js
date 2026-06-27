export function renderHubHomeCopy(params) {
  const { elements, copy, setText } = params;
  ensureMainlineEntry();

  if (elements.projectsTabStart) {
    elements.projectsTabStart.textContent = copy.home.tabs.start;
  }
  if (elements.projectsTabLibrary) {
    elements.projectsTabLibrary.textContent = copy.home.tabs.library;
  }
  if (elements.projectsTabBundles) {
    elements.projectsTabBundles.textContent = copy.home.tabs.bundles;
  }
  if (elements.projectsTabGuides) {
    elements.projectsTabGuides.textContent = copy.home.tabs.guides;
  }
  setText(elements.homeStep1Label, copy.home.steps.step1Label);
  setText(elements.homeStep1Title, copy.home.steps.step1Title);
  setText(elements.homeStep1Copy, copy.home.steps.step1Copy);
  setText(elements.homeStep2Label, copy.home.steps.step2Label);
  setText(elements.homeStep2Title, copy.home.steps.step2Title);
  setText(elements.homeStep2Copy, copy.home.steps.step2Copy);
  setText(elements.homeStep3Label, copy.home.steps.step3Label);
  setText(elements.homeStep3Title, copy.home.steps.step3Title);
  setText(elements.homeStep3Copy, copy.home.steps.step3Copy);
  setText(elements.homePathLabel, copy.home.path.label);
  setText(elements.homePathTitle, copy.home.path.title);
  setText(elements.homePathCopy, copy.home.path.copy);
  setText(elements.homeFlow1Title, copy.home.flow.title1);
  setText(elements.homeFlow1Copy, copy.home.flow.copy1);
  setText(elements.homeFlow2Title, copy.home.flow.title2);
  setText(elements.homeFlow2Copy, copy.home.flow.copy2);
  setText(elements.homeFlow3Title, copy.home.flow.title3);
  setText(elements.homeFlow3Copy, copy.home.flow.copy3);
  setText(elements.homeQuickLabel, copy.home.quick.label);
  setText(elements.homeQuickTitle, copy.home.quick.title);
  setText(elements.homeQuickCopy, copy.home.quick.copy);
  setText(elements.homeClusterLibraryTitle, copy.home.quick.libraryTitle);
  setText(elements.homeClusterLibraryCopy, copy.home.quick.libraryCopy);
  setText(elements.homeClusterBundlesTitle, copy.home.quick.bundlesTitle);
  setText(elements.homeClusterBundlesCopy, copy.home.quick.bundlesCopy);
  setText(elements.homeClusterGuidesTitle, copy.home.quick.guidesTitle);
  setText(elements.homeClusterGuidesCopy, copy.home.quick.guidesCopy);
  setText(elements.homeClusterInstallerTitle, copy.home.quick.installerTitle);
  setText(elements.homeClusterInstallerCopy, copy.home.quick.installerCopy);
  setText(elements.homeClusterRuntimesTitle, copy.home.quick.runtimesTitle);
  setText(elements.homeClusterRuntimesCopy, copy.home.quick.runtimesCopy);
  setText(elements.homeActionStart, copy.home.actions.start);
  setText(elements.homeActionSync, copy.home.actions.sync);
  setText(elements.homeActionOpen, copy.home.actions.open);
  renderMainlineEntry(copy, setText);
}

function ensureMainlineEntry() {
  const startPane = document.querySelector('[data-projects-pane="start"]');
  if (!startPane || document.getElementById("home-mainline-entry")) {
    return;
  }

  const mainline = document.createElement("article");
  mainline.className = "hub-card desktop-shell-surface-card hub-mainline-entry";
  mainline.id = "home-mainline-entry";
  mainline.innerHTML = `
    <div class="hub-card__intro">
      <div>
        <div class="hub-card__eyebrow" id="home-mainline-label">Mainline workflow</div>
        <h2 id="home-mainline-title">Follow the next best step</h2>
      </div>
      <p class="desktop-shell-note" id="home-mainline-copy">Use this as the primary route through Hub.</p>
    </div>
    <div class="hub-mainline-track" aria-label="Mainline workflow">
      ${[1, 2, 3, 4].map((step) => `
        <button class="hub-mainline-step" data-mainline-step="${step}" type="button">
          <span class="hub-mainline-step__index" id="home-mainline-step${step}-index">${step}</span>
          <strong id="home-mainline-step${step}-title"></strong>
          <span class="desktop-shell-note" id="home-mainline-step${step}-copy"></span>
        </button>
      `).join("")}
    </div>
  `;
  startPane.prepend(mainline);
}

function renderMainlineEntry(copy, setText) {
  const mainline = copy.home.mainline;
  if (!mainline) {
    return;
  }

  setText("home-mainline-label", mainline.label);
  setText("home-mainline-title", mainline.title);
  setText("home-mainline-copy", mainline.copy);
  mainline.steps.forEach((step, index) => {
    const id = index + 1;
    setText(`home-mainline-step${id}-index`, step.index);
    setText(`home-mainline-step${id}-title`, step.title);
    setText(`home-mainline-step${id}-copy`, step.copy);
    document.querySelector(`[data-mainline-step="${id}"]`)?.setAttribute("data-mainline-action", step.action);
  });
}
