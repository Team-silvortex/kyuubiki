export async function invokeTauri(command, payload = {}) {
  const tauri = window.__TAURI__;
  if (!tauri?.core?.invoke) {
    throw new Error("Tauri runtime is not available. Run this UI inside the desktop shell.");
  }

  return tauri.core.invoke(command, payload);
}

export async function listenTauri(eventName, handler) {
  const tauri = window.__TAURI__;
  if (!tauri?.event?.listen) {
    throw new Error("Tauri event API is not available.");
  }

  return tauri.event.listen(eventName, handler);
}

export async function loadDesktopBrand() {
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

export function normalizeDesktopLanguage(value) {
  return value === "zh" || value === "ja" ? value : "en";
}

export async function loadDesktopLanguagePreference() {
  try {
    const payload = await invokeTauri("get_global_language_preference");
    return normalizeDesktopLanguage(payload?.language);
  } catch (_error) {
    return "en";
  }
}

export async function saveDesktopLanguagePreference(language) {
  const payload = await invokeTauri("set_global_language_preference", {
    payload: { language: normalizeDesktopLanguage(language) },
  });
  return normalizeDesktopLanguage(payload?.language);
}

export function setText(id, value) {
  const element = document.getElementById(id);
  if (element && value) {
    element.textContent = value;
  }
}

const DESKTOP_STATE_VARIANTS = [
  "desktop-shell-state--active",
  "desktop-shell-state--healthy",
  "desktop-shell-state--warning",
  "desktop-shell-state--danger",
  "desktop-shell-state--idle",
];

const DESKTOP_STATE_KEYWORDS = {
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

function includesAnyKeyword(text, keywords) {
  return keywords.some((keyword) => text.includes(keyword));
}

export function classifyDesktopState(value, options = {}) {
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

export function applyDesktopState(element, value, options = {}) {
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
    element.textContent = value;
  }

  element.dataset.desktopStateResolved = state;
  return state;
}

function fallbackStateResult(value, options) {
  return classifyDesktopState(value, options);
}

export function syncDesktopStates(root = document) {
  root.querySelectorAll("[data-desktop-state]").forEach((element) => {
    const kind = element.dataset.desktopState || "auto";
    const value = element.dataset.stateValue || element.textContent || "";
    applyDesktopState(element, value, { kind });
  });
}
