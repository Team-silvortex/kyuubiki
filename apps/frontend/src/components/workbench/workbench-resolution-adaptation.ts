export type WorkbenchWindowMode = "standard" | "compact" | "narrow" | "ultranarrow";

export type WorkbenchViewportProfile = "desktop" | "compact" | "tablet" | "phone";

export type WorkbenchResolutionAdaptation = {
  bottomSafeAreaPx: number;
  minTouchTargetPx: number;
  profile: WorkbenchViewportProfile;
  shouldCompactChrome: boolean;
  shouldStackPanels: boolean;
  shouldUseScrollableShell: boolean;
  windowMode: WorkbenchWindowMode;
};

export type WorkbenchResolutionInput = {
  width: number;
  height: number;
};

export function resolveWorkbenchWindowMode(width: number): WorkbenchWindowMode {
  if (width <= 680) return "ultranarrow";
  if (width <= 980) return "narrow";
  if (width <= 1440) return "compact";
  return "standard";
}

export function resolveWorkbenchViewportProfile(width: number): WorkbenchViewportProfile {
  if (width <= 680) return "phone";
  if (width <= 980) return "tablet";
  if (width <= 1440) return "compact";
  return "desktop";
}

export function resolveWorkbenchResolutionAdaptation(
  input: WorkbenchResolutionInput,
): WorkbenchResolutionAdaptation {
  const width = Math.max(0, Math.floor(input.width));
  const height = Math.max(0, Math.floor(input.height));
  const profile = resolveWorkbenchViewportProfile(width);
  const shortWindow = height > 0 && height < 720;
  const phone = profile === "phone";
  const tablet = profile === "tablet";

  return {
    bottomSafeAreaPx: phone ? 84 : shortWindow ? 72 : 56,
    minTouchTargetPx: phone || tablet ? 44 : 36,
    profile,
    shouldCompactChrome: profile !== "desktop" || shortWindow,
    shouldStackPanels: phone || width <= 860,
    shouldUseScrollableShell: phone || shortWindow,
    windowMode: resolveWorkbenchWindowMode(width),
  };
}

export function buildWorkbenchResolutionStyleVars(
  adaptation: WorkbenchResolutionAdaptation,
): Record<string, string> {
  return {
    "--workbench-bottom-safe-area": `${adaptation.bottomSafeAreaPx}px`,
    "--workbench-min-touch-target": `${adaptation.minTouchTargetPx}px`,
  };
}
