"use client";

export function safeWorkbenchPanelStorageGet(key: string) {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as { code?: string }) : null;
  } catch {
    return null;
  }
}

export function writeWorkbenchPanelStorage(key: string, code: string) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(key, JSON.stringify({ code }));
}
