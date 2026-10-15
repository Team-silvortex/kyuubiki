import { advanceWorkbenchAuthorityRevision } from "./authority-revisions";
import { resolveWorkbenchBackendTarget, sanitizeWorkbenchApiBaseUrl, WORKBENCH_API_BASE_URL_STORAGE_KEY } from "./backend-target";

const SETTINGS_KEY = "kyuubiki-workbench-settings";
const MAX_EVENT_VALUE_LENGTH = 65_536;
const trackedWindows = new WeakSet<Window>();

function readSettings(value: string | null) {
  if (value !== null && value.length > MAX_EVENT_VALUE_LENGTH) return null;
  try {
    const parsed: unknown = value ? JSON.parse(value) : {};
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return null;
    const settings = parsed as Record<string, unknown>;
    const mode = settings.frontendRuntimeMode ?? "orchestrated_gui";
    if (mode !== "orchestrated_gui" && mode !== "direct_mesh_gui") return null;
    const base = settings.backendApiBaseUrl;
    if (base != null && typeof base !== "string") return null;
    return { mode, base: sanitizeWorkbenchApiBaseUrl(base) };
  } catch { return null; }
}

function revokeConfiguration() {
  advanceWorkbenchAuthorityRevision("mode");
  advanceWorkbenchAuthorityRevision("backend");
}

function invalidateStoredTarget() {
  // A URL-pinned target is independent of edits to either persisted target field.
  if (resolveWorkbenchBackendTarget().source !== "query") advanceWorkbenchAuthorityRevision("backend");
}

function handleStorageChange(browser: Window, event: StorageEvent) {
  if (event.key !== null && event.key !== SETTINGS_KEY && event.key !== WORKBENCH_API_BASE_URL_STORAGE_KEY) return;
  try {
    if (event.storageArea !== browser.localStorage) return;
  } catch { revokeConfiguration(); return; }
  if (event.key === null) { revokeConfiguration(); return; }

  // Read the event's old/new values, not storage's latest value: queued A -> B -> A
  // notifications must still fence the request that began under the first A.
  if (event.key === SETTINGS_KEY) {
    const before = readSettings(event.oldValue), after = readSettings(event.newValue);
    if (!before || !after) { revokeConfiguration(); return; }
    if (before.mode !== after.mode) advanceWorkbenchAuthorityRevision("mode");
    if (before.base !== after.base) invalidateStoredTarget();
  } else if ([event.oldValue, event.newValue].some(value => value !== null && value.length > MAX_EVENT_VALUE_LENGTH)
    || sanitizeWorkbenchApiBaseUrl(event.oldValue) !== sanitizeWorkbenchApiBaseUrl(event.newValue)) {
    invalidateStoredTarget();
  }
}

export function ensureWorkbenchAuthorityStorageTracking() {
  if (typeof window === "undefined" || typeof window.addEventListener !== "function" || trackedWindows.has(window)) return;
  const browser = window;
  browser.addEventListener("storage", event => handleStorageChange(browser, event));
  trackedWindows.add(browser);
}
