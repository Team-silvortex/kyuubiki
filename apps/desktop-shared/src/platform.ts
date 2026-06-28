export const DESKTOP_PLATFORMS = ["macos", "linux", "windows"] as const;

export type DesktopPlatform = (typeof DESKTOP_PLATFORMS)[number];
export type DesktopPlatformFilter = DesktopPlatform | "all";

export const DESKTOP_PLATFORM_LABELS: Record<DesktopPlatformFilter, string> = {
  all: "All platforms",
  macos: "macOS",
  linux: "Linux",
  windows: "Windows",
};

type PlatformOptions = {
  allowAll?: boolean;
  includeAll?: boolean;
  fallback?: DesktopPlatform;
};

type DesktopPlatformSelect = {
  value: string;
  innerHTML: string;
  appendChild: (element: HTMLOptionElement) => void;
};

type DesktopReleaseInput = {
  value: string;
  placeholder: string;
};

export function detectDesktopPlatform(): DesktopPlatform {
  const navigatorWithUaData = globalThis.navigator as Navigator & {
    userAgentData?: { platform?: string };
  };
  const platformText = [
    navigatorWithUaData?.userAgentData?.platform,
    navigatorWithUaData?.platform,
    navigatorWithUaData?.userAgent,
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();

  if (platformText.includes("mac")) return "macos";
  if (platformText.includes("win")) return "windows";
  return "linux";
}

export function normalizeDesktopPlatform(
  value: unknown,
  fallback: DesktopPlatform = detectDesktopPlatform(),
  options: PlatformOptions = {},
): DesktopPlatformFilter {
  if (options.allowAll && value === "all") return "all";
  return isDesktopPlatform(value) ? value : fallback;
}

export function desktopPlatformLabel(value: unknown): string {
  return (
    DESKTOP_PLATFORM_LABELS[normalizeDesktopPlatform(value, "linux", { allowAll: true })] ||
    DESKTOP_PLATFORM_LABELS.linux
  );
}

export function desktopReleaseRoot(value: unknown): string {
  return `dist/${normalizeDesktopPlatform(value)}`;
}

export function desktopReleaseRootPattern(): string {
  return "dist/{platform}";
}

export function desktopPlatformContextLabel(
  label: string,
  value: unknown,
  context = "host",
): string {
  const suffix = String(context || "").trim();
  return suffix
    ? `${label} · ${desktopPlatformLabel(value)} ${suffix}`
    : `${label} · ${desktopPlatformLabel(value)}`;
}

export function desktopPlatformOptions(options: PlatformOptions = {}) {
  const entries: DesktopPlatformFilter[] = options.includeAll
    ? ["all", ...DESKTOP_PLATFORMS]
    : [...DESKTOP_PLATFORMS];
  return entries.map((value) => ({
    value,
    label: DESKTOP_PLATFORM_LABELS[value],
  }));
}

export function populateDesktopPlatformSelect(
  select: DesktopPlatformSelect | null | undefined,
  options: PlatformOptions = {},
): void {
  if (!select) return;
  const currentValue = String(select.value || "").trim();
  const normalizedCurrent = normalizeDesktopPlatform(
    currentValue,
    options.fallback || detectDesktopPlatform(),
    { allowAll: options.includeAll === true },
  );
  select.innerHTML = "";
  for (const option of desktopPlatformOptions({ includeAll: options.includeAll === true })) {
    const element = document.createElement("option");
    element.value = option.value;
    element.textContent = option.label;
    if (option.value === normalizedCurrent) {
      element.selected = true;
    }
    select.appendChild(element);
  }
}

export function syncDesktopReleaseTargetInput(
  input: DesktopReleaseInput | null | undefined,
  platform: unknown,
): void {
  if (!input) return;
  const nextValue = desktopReleaseRoot(platform);
  const currentValue = String(input.value || "").trim();
  const placeholder = String(input.placeholder || "").trim();

  input.placeholder = nextValue;

  if (
    !currentValue ||
    currentValue === placeholder ||
    currentValue.startsWith("dist/")
  ) {
    input.value = nextValue;
  }
}

function isDesktopPlatform(value: unknown): value is DesktopPlatform {
  return DESKTOP_PLATFORMS.includes(value as DesktopPlatform);
}
