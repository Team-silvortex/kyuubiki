type TauriInvoke = (command: string, payload?: Record<string, unknown>) => Promise<unknown>;
type TauriListen = (
  eventName: string,
  handler: (event: unknown) => void,
) => Promise<() => void>;

type TauriRuntime = {
  core?: {
    invoke?: TauriInvoke;
  };
  event?: {
    listen?: TauriListen;
  };
};

type DesktopLanguagePayload = {
  language?: unknown;
};

type DesktopStateKind = "activity" | "health" | "auto";
type DesktopState = "active" | "healthy" | "warning" | "danger" | "idle";

type DesktopStateOptions = {
  fallback?: DesktopState;
  kind?: DesktopStateKind;
};

type DesktopLanguageWatcherOptions = {
  getCurrentLanguage?: () => unknown;
  onChange?: (language: string) => void | Promise<void>;
  intervalMs?: number;
};

type DesktopStateKeywordMap = Record<DesktopState, string[]>;

declare global {
  interface Window {
    __TAURI__?: TauriRuntime;
  }
}

export async function invokeTauri(
  command: string,
  payload: Record<string, unknown> = {},
): Promise<unknown> {
  const tauri = window.__TAURI__;
  if (!tauri?.core?.invoke) {
    throw new Error("Tauri runtime is not available. Run this UI inside the desktop shell.");
  }

  return tauri.core.invoke(command, payload);
}

export async function listenTauri(
  eventName: string,
  handler: (event: unknown) => void,
): Promise<() => void> {
  const tauri = window.__TAURI__;
  if (!tauri?.event?.listen) {
    throw new Error("Tauri event API is not available.");
  }

  return tauri.event.listen(eventName, handler);
}

export async function loadDesktopBrand(): Promise<unknown | null> {
  try {
    const response = await fetch("./assets/brand.json");
    if (!response.ok) {
      return null;
    }

    return response.json();
  } catch (_error) {
    return null;
  }
}

export function normalizeDesktopLanguage(value: unknown): string {
  return typeof value === "string" && value.trim() ? value.trim() : "en";
}

function readLanguagePayload(payload: unknown): string {
  return normalizeDesktopLanguage((payload as DesktopLanguagePayload | null)?.language);
}

export async function loadDesktopLanguagePreference(): Promise<string> {
  try {
    const payload = await invokeTauri("get_global_language_preference");
    return readLanguagePayload(payload);
  } catch (_error) {
    return "en";
  }
}

export async function saveDesktopLanguagePreference(language: unknown): Promise<string> {
  const payload = await invokeTauri("set_global_language_preference", {
    payload: { language: normalizeDesktopLanguage(language) },
  });
  return readLanguagePayload(payload);
}

export function watchDesktopLanguagePreference({
  getCurrentLanguage,
  onChange,
  intervalMs = 2000,
}: DesktopLanguageWatcherOptions = {}): () => void {
  if (typeof onChange !== "function") {
    return () => {};
  }

  let disposed = false;
  let pending = false;
  const readCurrentLanguage =
    typeof getCurrentLanguage === "function" ? getCurrentLanguage : () => undefined;

  const checkLanguage = async () => {
    if (disposed || pending) {
      return;
    }

    pending = true;
    try {
      const nextLanguage = await loadDesktopLanguagePreference();
      if (nextLanguage && nextLanguage !== normalizeDesktopLanguage(readCurrentLanguage())) {
        await onChange(nextLanguage);
      }
    } finally {
      pending = false;
    }
  };

  const onVisibilityChange = () => {
    if (document.visibilityState !== "hidden") {
      void checkLanguage();
    }
  };

  window.addEventListener("focus", checkLanguage);
  window.addEventListener("pageshow", checkLanguage);
  document.addEventListener("visibilitychange", onVisibilityChange);
  const timer = window.setInterval(() => {
    if (document.visibilityState !== "hidden") {
      void checkLanguage();
    }
  }, Math.max(500, intervalMs));

  return () => {
    disposed = true;
    window.clearInterval(timer);
    window.removeEventListener("focus", checkLanguage);
    window.removeEventListener("pageshow", checkLanguage);
    document.removeEventListener("visibilitychange", onVisibilityChange);
  };
}

export function setText(target: string | Element | null, value: unknown): void {
  const element = typeof target === "string" ? document.getElementById(target) : target;
  if (element && value !== undefined && value !== null) {
    element.textContent = String(value);
  }
}

const DESKTOP_STATE_VARIANTS = [
  "desktop-shell-state--active",
  "desktop-shell-state--healthy",
  "desktop-shell-state--warning",
  "desktop-shell-state--danger",
  "desktop-shell-state--idle",
];

const DESKTOP_STATE_KEYWORDS: DesktopStateKeywordMap = {
  danger: [
    "error",
    "failed",
    "failure",
    "down",
    "stopped",
    "unhealthy",
    "invalid",
    "denied",
    "panic",
    "refused",
    "crash",
    "dead",
  ],
  warning: [
    "warning",
    "missing",
    "pending",
    "starting",
    "restarting",
    "degraded",
    "partial",
    "timeout",
    "attention",
    "queued",
    "recent",
  ],
  healthy: ["ready", "healthy", "running", "stable", "connected", "ok", "passed", "online"],
  active: [
    "active",
    "enabled",
    "selected",
    "tauri",
    "local",
    "cloud",
    "distributed",
    "direct mesh",
    "direct-mesh",
    "orchestrated_gui",
    "sqlite",
    "postgres",
    "frontend",
  ],
  idle: ["idle", "unknown", "none", "--", "n/a"],
};

function includesAnyKeyword(text: string, keywords: string[]): boolean {
  return keywords.some((keyword) => text.includes(keyword));
}

export function classifyDesktopState(
  value: unknown,
  options: DesktopStateOptions = {},
): DesktopState {
  const fallback = options.fallback || "idle";
  const kind = options.kind || "auto";
  const text = String(value || "")
    .trim()
    .toLowerCase();

  if (!text) {
    return fallback;
  }

  if (kind === "activity") {
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.danger)) return "danger";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.warning)) return "warning";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.active)) return "active";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.healthy)) return "healthy";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.idle)) return "idle";
    return fallback;
  }

  if (kind === "health") {
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.danger)) return "danger";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.warning)) return "warning";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.healthy)) return "healthy";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.active)) return "active";
    if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.idle)) return "idle";
    return fallback;
  }

  if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.danger)) return "danger";
  if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.warning)) return "warning";
  if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.healthy)) return "healthy";
  if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.active)) return "active";
  if (includesAnyKeyword(text, DESKTOP_STATE_KEYWORDS.idle)) return "idle";
  return fallback;
}

export function applyDesktopState(
  element: Element | null,
  value: unknown,
  options: DesktopStateOptions = {},
): DesktopState {
  if (!element) {
    return fallbackStateResult(value, options);
  }

  const resolvedValue = value ?? element.textContent ?? "";
  const state = classifyDesktopState(resolvedValue, options);
  element.classList.add("desktop-shell-state");
  DESKTOP_STATE_VARIANTS.forEach((className) => {
    element.classList.remove(className);
  });
  element.classList.add(`desktop-shell-state--${state}`);

  if (value !== undefined) {
    element.textContent = String(value);
  }

  if (element instanceof HTMLElement) {
    element.dataset.desktopStateResolved = state;
  }
  return state;
}

function fallbackStateResult(value: unknown, options: DesktopStateOptions): DesktopState {
  return classifyDesktopState(value, options);
}

export function syncDesktopStates(root: ParentNode = document): void {
  root.querySelectorAll("[data-desktop-state]").forEach((element) => {
    const kind =
      element instanceof HTMLElement ? element.dataset.desktopState || "auto" : "auto";
    const value =
      element instanceof HTMLElement
        ? element.dataset.stateValue || element.textContent || ""
        : element.textContent || "";
    applyDesktopState(element, value, { kind: kind as DesktopStateKind });
  });
}
