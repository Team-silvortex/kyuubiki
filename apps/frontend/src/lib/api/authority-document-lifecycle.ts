type DocumentLifetime = { active: boolean; generation: symbol };
const lifetimes = new WeakMap<Window, DocumentLifetime>();

export function captureWorkbenchDocumentAuthority() {
  if (typeof window === "undefined") return () => typeof window === "undefined";
  const browser = window;
  // Non-browser callers have no lifecycle events, but still cannot change owners.
  if (typeof browser.addEventListener !== "function") return () => typeof window !== "undefined" && window === browser;
  let lifetime = lifetimes.get(browser);
  if (!lifetime) {
    const state = { active: true, generation: Symbol() };
    browser.addEventListener("pagehide", () => { state.active = false; state.generation = Symbol(); });
    browser.addEventListener("pageshow", () => { state.active = true; });
    lifetimes.set(browser, state);
    lifetime = state;
  }
  // Unlike visibilitychange, pagehide ends the authority of already-started work.
  const { active, generation } = lifetime;
  return () => typeof window !== "undefined" && window === browser && active
    && lifetime.active && lifetime.generation === generation;
}
