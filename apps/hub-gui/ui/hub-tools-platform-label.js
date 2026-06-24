import {
  desktopPlatformContextLabel,
  normalizeDesktopPlatform,
} from "./shared/platform.js";

export function createToolsPlatformLabelRenderer({ elements, hubCopy, state }) {
  return function renderToolsPlatformLabel() {
    if (!elements.toolsPackagesPlatformLabel) {
      return;
    }

    const baseLabel = hubCopy().panels.tools.packages.platform;
    const selectedPlatform = elements.releasePlatform?.value || state.hostPlatform;
    elements.toolsPackagesPlatformLabel.textContent = desktopPlatformContextLabel(
      baseLabel,
      normalizeDesktopPlatform(selectedPlatform, state.hostPlatform, { allowAll: true }),
      "",
    );
  };
}
