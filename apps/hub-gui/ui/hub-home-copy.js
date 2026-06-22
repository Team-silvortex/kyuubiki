export function renderHubHomeCopy(params) {
  const { elements, copy, setText } = params;

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
}
