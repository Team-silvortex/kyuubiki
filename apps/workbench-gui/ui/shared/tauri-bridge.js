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

export function setText(id, value) {
  const element = document.getElementById(id);
  if (element && value) {
    element.textContent = value;
  }
}
