import { loadDesktopBrand, setText } from "./shared/tauri-bridge.js";

export function createHubBrandPanel({ state }) {
  async function applyBrand() {
    const brand = await loadDesktopBrand();
    if (!brand) {
      return;
    }

    const releaseVersion = String(brand.releaseVersion || "").replace(/^v/u, "");
    const releaseCodename = String(brand.releaseCodename || "").trim();
    const releaseTag = [releaseCodename, releaseVersion].filter(Boolean).join(" ");

    if (brand.hubName) {
      state.releaseVersion = releaseVersion;
      state.releaseCodename = releaseCodename;
      document.title = releaseTag ? `${brand.hubName} · ${releaseTag}` : brand.hubName;
    }

    setText("brand-hub-title", brand.hubShortName || "Hub");
    setText("brand-hub-role", brand.shellRoleLabel);
    setText("brand-hub-role-chip", brand.shellRoleLabel);
    setText("brand-hub-focus", brand.shellFocusLabel);
    if (releaseTag) {
      setText("brand-hub-version", releaseTag);
    }
  }

  function releaseLabel() {
    const releaseTag = [state.releaseCodename, state.releaseVersion].filter(Boolean).join(" ");
    return releaseTag ? `Kyuubiki Hub · ${releaseTag}` : "Kyuubiki Hub";
  }

  function formatRuntimeReport(value) {
    const body = String(value || "").trim();
    return body ? `${releaseLabel()}\n\n${body}` : releaseLabel();
  }

  return {
    applyBrand,
    formatRuntimeReport,
  };
}
