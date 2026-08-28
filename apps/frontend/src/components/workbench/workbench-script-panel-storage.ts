"use client";

export type WorkbenchPanelStorageReadResult = {
  value: { code: string } | null;
  readable: boolean;
};

function readBrowserStorage(
  storage: Storage | undefined,
  key: string,
): WorkbenchPanelStorageReadResult {
  if (!storage) return { value: null, readable: false };
  try {
    const raw = storage.getItem(key);
    if (raw === null) return { value: null, readable: true };
    const parsed = JSON.parse(raw) as unknown;
    if (
      !parsed ||
      typeof parsed !== "object" ||
      Array.isArray(parsed) ||
      typeof (parsed as { code?: unknown }).code !== "string"
    ) {
      return { value: null, readable: false };
    }
    return { value: { code: (parsed as { code: string }).code }, readable: true };
  } catch {
    return { value: null, readable: false };
  }
}

export function safeWorkbenchPanelStorageGetResult(key: string): WorkbenchPanelStorageReadResult {
  if (typeof window === "undefined") return { value: null, readable: false };
  const sessionResult = readBrowserStorage(window.sessionStorage, key);
  if (sessionResult.value) return sessionResult;

  const legacyResult = readBrowserStorage(window.localStorage, key);
  if (!sessionResult.readable) {
    return { value: legacyResult.value, readable: false };
  }
  if (!legacyResult.value) {
    return {
      value: null,
      readable: sessionResult.readable && legacyResult.readable,
    };
  }

  try {
    window.sessionStorage.setItem(key, JSON.stringify(legacyResult.value));
    window.localStorage.removeItem(key);
  } catch {
    return { value: legacyResult.value, readable: false };
  }

  return { value: legacyResult.value, readable: sessionResult.readable };
}

export function safeWorkbenchPanelStorageGet(key: string) {
  return safeWorkbenchPanelStorageGetResult(key).value;
}

export function writeWorkbenchPanelStorage(key: string, code: string): boolean {
  if (typeof window === "undefined") return false;
  try {
    window.sessionStorage.setItem(key, JSON.stringify({ code }));
    window.localStorage.removeItem(key);
    return true;
  } catch {
    return false;
  }
}
