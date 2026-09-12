type FullscreenDocument = {
  fullscreenElement?: Element | null;
  exitFullscreen?: () => void | Promise<void>;
  webkitFullscreenElement?: Element | null;
  webkitExitFullscreen?: () => void | Promise<void>;
};

export const workbenchFullscreenEvents = ["fullscreenchange", "webkitfullscreenchange", "workbenchfullscreenchange"];
type FullscreenTarget = HTMLElement & { webkitRequestFullscreen?: () => void | Promise<void> };
const windowSessions = new WeakMap<FullscreenDocument, { target: HTMLElement; close: () => void }>();

export function getWindowImmersiveTarget(document: FullscreenDocument): HTMLElement | null {
  return windowSessions.get(document)?.target ?? null;
}

function enterWindowImmersive(document: Document, target: HTMLElement) {
  const restore: Array<() => void> = [];
  const previousFocus = document.activeElement as HTMLElement | null;
  const notify = () => document.dispatchEvent(new Event("workbenchfullscreenchange"));
  const close = () => {
    if (windowSessions.get(document)?.target !== target) return;
    windowSessions.delete(document);
    target.removeAttribute("data-workbench-window-fullscreen");
    document.removeEventListener("keydown", keyboard, true);
    for (const reset of restore.reverse()) reset();
    previousFocus?.focus?.({ preventScroll: true });
    notify();
  };
  const keyboard = (event: KeyboardEvent) => {
    if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); close(); }
    if (event.key !== "Tab") return;
    const items = [...target.querySelectorAll<HTMLElement>("button, input, select, textarea, a[href], [tabindex]")]
      .filter((item) => item.tabIndex >= 0 && !item.matches(":disabled") && item.getClientRects().length > 0);
    const index = items.indexOf(document.activeElement as HTMLElement);
    if (!items.length) { event.preventDefault(); return; }
    if (index === -1 || (event.shiftKey ? index === 0 : index === items.length - 1)) {
      event.preventDefault();
      items[event.shiftKey ? items.length - 1 : 0].focus();
    }
  };
  // Keep React's tree in place, but isolate the expanded panel from hidden siblings.
  for (let branch: HTMLElement = target; branch.parentElement; branch = branch.parentElement) {
    const parent = branch.parentElement;
    const old = parent.getAttribute("data-workbench-fullscreen-ancestor");
    parent.setAttribute("data-workbench-fullscreen-ancestor", "true");
    restore.push(() => old === null ? parent.removeAttribute("data-workbench-fullscreen-ancestor")
      : parent.setAttribute("data-workbench-fullscreen-ancestor", old));
    for (const sibling of parent.children) {
      if (sibling === branch || sibling.hasAttribute("inert")) continue;
      sibling.setAttribute("inert", "");
      restore.push(() => sibling.removeAttribute("inert"));
    }
  }
  target.setAttribute("data-workbench-window-fullscreen", "true");
  windowSessions.set(document, { target, close });
  document.addEventListener("keydown", keyboard, true);
  notify();
}

export async function requestWorkbenchViewportFullscreen(document: Document, target: FullscreenTarget): Promise<void> {
  if (isWorkbenchViewportFullscreen(document, target)) return;
  const existing = windowSessions.get(document);
  if (existing || document.fullscreenElement || (document as FullscreenDocument).webkitFullscreenElement) {
    throw new Error("viewport:fullscreen_owned_elsewhere");
  }
  if (typeof target.requestFullscreen === "function") await target.requestFullscreen();
  else if (typeof target.webkitRequestFullscreen === "function") await target.webkitRequestFullscreen();
  else enterWindowImmersive(document, target);
}

export function isWorkbenchViewportFullscreen(
  document: FullscreenDocument,
  target: Element | null | undefined,
): boolean {
  return Boolean(target && (document.fullscreenElement === target || document.webkitFullscreenElement === target
    || windowSessions.get(document)?.target === target));
}

export async function exitWorkbenchViewportFullscreen(
  document: FullscreenDocument,
  target: Element | null | undefined,
): Promise<void> {
  if (!isWorkbenchViewportFullscreen(document, target)) return;
  const session = windowSessions.get(document);
  if (session && session.target === target) { session.close(); return; }
  if (document.fullscreenElement === target && typeof document.exitFullscreen === "function") await document.exitFullscreen();
  else if (document.webkitFullscreenElement === target && typeof document.webkitExitFullscreen === "function") {
    await document.webkitExitFullscreen();
  }
}
