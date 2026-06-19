"use client";

function readBrowserStorage(storage: Storage | undefined, key: string) {
  if (!storage) return null;
  try {
    const raw = storage.getItem(key);
    return raw ? (JSON.parse(raw) as { code?: string }) : null;
  } catch {
    return null;
  }
}

export function safeWorkbenchPanelStorageGet(key: string) {
  if (typeof window === "undefined") return null;
  const sessionValue = readBrowserStorage(window.sessionStorage, key);
  if (sessionValue) return sessionValue;

  const legacyValue = readBrowserStorage(window.localStorage, key);
  if (!legacyValue) return null;

  try {
    window.sessionStorage.setItem(key, JSON.stringify(legacyValue));
    window.localStorage.removeItem(key);
  } catch {
    return legacyValue;
  }

  return legacyValue;
}

export function writeWorkbenchPanelStorage(key: string, code: string) {
  if (typeof window === "undefined") return;
  window.sessionStorage.setItem(key, JSON.stringify({ code }));
  window.localStorage.removeItem(key);
}
