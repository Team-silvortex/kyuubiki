function topLevelBlock(root, selector) {
  let node = root.querySelector(selector);
  while (node?.parentElement && node.parentElement !== root) {
    node = node.parentElement;
  }
  return node?.parentElement === root ? node : null;
}

function activateSection(root, name) {
  root.querySelectorAll("[data-installer-section-target]").forEach((button) => {
    const active = button.dataset.installerSectionTarget === name;
    button.classList.toggle("installer-section-tab--active", active);
    button.setAttribute("aria-selected", String(active));
    button.tabIndex = active ? 0 : -1;
  });
  root.querySelectorAll("[data-installer-section]").forEach((section) => {
    const active = section.dataset.installerSection === name;
    section.classList.toggle("installer-panel-section--active", active);
    section.hidden = !active;
    if (active) {
      section.scrollTop = 0;
    }
  });
  root.dataset.activeInstallerSection = name;
}

export function mountInstallerPanelSections(root, config) {
  if (!root || root.dataset.installerSectionsMounted === "true") {
    return;
  }

  const nav = document.createElement("div");
  nav.className = "installer-section-nav";
  nav.dataset.installerSectionNav = "";
  nav.setAttribute("role", "tablist");
  nav.setAttribute("aria-label", config.label);

  config.sections.forEach((definition, index) => {
    const button = document.createElement("button");
    button.className = "installer-section-tab";
    button.type = "button";
    button.dataset.installerSectionTarget = definition.name;
    button.id = `${root.id}-${definition.name}-tab`;
    button.textContent = definition.label;
    button.setAttribute("role", "tab");
    button.setAttribute("aria-controls", `${root.id}-${definition.name}-section`);
    button.addEventListener("click", () => activateSection(root, definition.name));
    nav.appendChild(button);

    const section = document.createElement("section");
    section.className = "installer-panel-section";
    section.dataset.installerSection = definition.name;
    section.id = `${root.id}-${definition.name}-section`;
    section.setAttribute("role", "tabpanel");
    section.setAttribute("aria-labelledby", button.id);

    const blocks = new Set(
      definition.anchors.map((selector) => topLevelBlock(root, selector)).filter(Boolean),
    );
    blocks.forEach((block) => section.appendChild(block));
    root.appendChild(section);

    if (index === 0) {
      button.classList.add("installer-section-tab--active");
    }
  });

  nav.addEventListener("keydown", (event) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
      return;
    }
    const tabs = Array.from(nav.querySelectorAll("[data-installer-section-target]"));
    const current = Math.max(0, tabs.indexOf(document.activeElement));
    const nextIndex = event.key === "Home"
      ? 0
      : event.key === "End"
        ? tabs.length - 1
        : (current + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length;
    const next = tabs[nextIndex];
    event.preventDefault();
    next?.focus();
    if (next?.dataset.installerSectionTarget) {
      activateSection(root, next.dataset.installerSectionTarget);
    }
  });

  const header = root.querySelector(":scope > .section-header");
  header?.insertAdjacentElement("afterend", nav);
  root.classList.add("panel--sectioned");
  root.dataset.installerSectionsMounted = "true";
  activateSection(root, config.defaultSection || config.sections[0]?.name);
}
