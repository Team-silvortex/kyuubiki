import { loadDesktopBrand, setText, syncDesktopStates } from "./shared/tauri-bridge.js";

const sectionModel = {
  projects: {
    title: "Projects",
    copy: "Open recent engineering work, import standardized bundles, and jump into Workbench.",
  },
  runtimes: {
    title: "Runtimes",
    copy: "Monitor local and remote runtime targets, control stack lifecycle, and see which compute path is active.",
  },
  deploy: {
    title: "Deploy",
    copy: "Move between local, cloud, distributed, and direct-mesh deployment flows from one desktop shell.",
  },
  observe: {
    title: "Observe",
    copy: "Use the Hub as the runtime overview wall for health, logs, watchdog state, and security-sensitive activity.",
  },
  tools: {
    title: "Tools",
    copy: "Launch diagnostics, validation, packaging, and benchmark workflows without dropping to separate entrypoints.",
  },
};

const title = document.getElementById("section-title");
const copy = document.getElementById("section-copy");
const navItems = Array.from(document.querySelectorAll(".hub-nav__item"));
const panels = Array.from(document.querySelectorAll(".hub-panel"));

async function applyBrand() {
  const brand = await loadDesktopBrand();
  if (!brand) {
    return;
  }

  if (brand.hubName) {
    document.title = brand.hubName;
  }

  setText("brand-hub-title", brand.hubName);
}

function setSection(section) {
  const next = sectionModel[section];
  if (!next) return;

  title.textContent = next.title;
  copy.textContent = next.copy;

  navItems.forEach((item) => {
    item.classList.toggle("hub-nav__item--active", item.dataset.target === section);
  });

  panels.forEach((panel) => {
    panel.classList.toggle("hidden", panel.id !== `${section}-panel`);
  });

  const defaultProjectsPanel = document.getElementById("projects-panel");
  if (defaultProjectsPanel) {
    defaultProjectsPanel.classList.toggle("hidden", section !== "projects");
  }
}

navItems.forEach((item) => {
  item.addEventListener("click", () => setSection(item.dataset.target));
});

await applyBrand();
syncDesktopStates();
setSection("projects");
