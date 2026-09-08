import {
  resolveWorkbenchPanelLayout, resizeWorkbenchPanel,
  type WorkbenchPanelGeometry, type WorkbenchPanelPreferences, type WorkbenchPanelSize,
} from "./workbench-panel-layout";
import { createWorkbenchPanelPersistence } from "./workbench-panel-persistence";
import { createWorkbenchPanelLayoutClient } from "../../../../desktop-shared/src/workbench-panel-layout-bridge";

const handleSelector = "[data-workbench-resize]";

export function installWorkbenchPanelLayout(root: HTMLElement) {
  let preferences: WorkbenchPanelPreferences = {};
  let frame = 0;
  let drag: {
    handle: HTMLElement; panel: WorkbenchPanelSize; pointer: number; origin: number;
    size: number; before: WorkbenchPanelPreferences; geometry: WorkbenchPanelGeometry;
    windowWidth: number; windowHeight: number;
  } | null = null;

  const persistence = createWorkbenchPanelPersistence({
    storage: () => window.localStorage,
    client: createWorkbenchPanelLayoutClient(),
    restore: sizes => { preferences = sizes; schedulePaint(); },
    status: value => { root.dataset.workbenchLayoutStorage = value; },
  });
  preferences = persistence.initial;

  function geometry(): WorkbenchPanelGeometry {
    const style = getComputedStyle(root);
    return {
      width: root.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight),
      height: root.querySelector(".workspace-main")?.getBoundingClientRect().height ?? root.clientHeight,
      gap: parseFloat(style.columnGap) || 0,
      workflow: root.dataset.workbenchSection === "workflow",
    };
  }

  function paint() {
    frame = 0;
    const layout = resolveWorkbenchPanelLayout(preferences, geometry());
    for (const panel of ["sidebar", "inspector", "report"] as const) {
      const property = `--workbench-${panel}-size`;
      const value = `${layout.sizes[panel]}px`;
      if (root.style.getPropertyValue(property) !== value) root.style.setProperty(property, value);
      const handle = root.querySelector<HTMLElement>(`[data-workbench-resize="${panel}"]`);
      handle?.setAttribute("aria-valuenow", String(Math.round(layout.sizes[panel])));
      handle?.setAttribute("aria-valuemin", String(Math.floor(layout.limits[panel].min)));
      handle?.setAttribute("aria-valuemax", String(Math.ceil(layout.limits[panel].max)));
    }
  }

  function schedulePaint() {
    if (!frame) frame = requestAnimationFrame(paint);
  }

  function save() {
    persistence.save(preferences);
  }

  function stop(commit: boolean) {
    if (!drag) return;
    const previous = drag;
    drag = null;
    if (!commit) preferences = previous.before;
    root.removeAttribute("data-workbench-resizing");
    if (previous.handle.hasPointerCapture(previous.pointer)) previous.handle.releasePointerCapture(previous.pointer);
    if (frame) cancelAnimationFrame(frame);
    paint();
    if (commit) save();
  }

  function handleFrom(event: Event): HTMLElement | null {
    return event.target instanceof Element ? event.target.closest<HTMLElement>(handleSelector) : null;
  }

  function panelFrom(handle: HTMLElement): WorkbenchPanelSize | null {
    const panel = handle.dataset.workbenchResize;
    return panel === "sidebar" || panel === "inspector" || panel === "report" ? panel : null;
  }

  function enabled(handle: HTMLElement) {
    return root.dataset.workbenchStackPanels !== "true" && handle.getBoundingClientRect().height > 0;
  }

  function pointerDown(event: PointerEvent) {
    const handle = handleFrom(event);
    const panel = handle && panelFrom(handle);
    if (!handle || !panel || !enabled(handle) || drag || event.button !== 0 || !event.isPrimary) return;
    event.preventDefault();
    const bounds = geometry();
    persistence.changed();
    handle.focus({ preventScroll: true });
    handle.setPointerCapture(event.pointerId);
    drag = {
      handle, panel, pointer: event.pointerId, before: preferences, geometry: bounds,
      windowWidth: window.innerWidth, windowHeight: window.innerHeight,
      origin: panel === "report" ? event.clientY : event.clientX,
      size: resolveWorkbenchPanelLayout(preferences, bounds).sizes[panel],
    };
    root.dataset.workbenchResizing = panel;
  }

  function pointerMove(event: PointerEvent) {
    if (!drag || drag.pointer !== event.pointerId) return;
    // WebViews can deliver pointerup before the pending window resize event.
    if (window.innerWidth !== drag.windowWidth || window.innerHeight !== drag.windowHeight) {
      stop(false);
      return;
    }
    const coordinate = drag.panel === "report" ? event.clientY : event.clientX;
    const sign = drag.panel === "sidebar" ? 1 : -1;
    preferences = resizeWorkbenchPanel(drag.before, drag.geometry, drag.panel, drag.size + (coordinate - drag.origin) * sign);
    schedulePaint();
  }

  function pointerUp(event: PointerEvent) {
    if (drag?.pointer !== event.pointerId) return;
    pointerMove(event);
    stop(true);
  }

  function pointerCancel(event: PointerEvent) {
    if (drag?.pointer === event.pointerId) stop(false);
  }

  function keyDown(event: KeyboardEvent) {
    if (drag) {
      if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); stop(false); }
      return;
    }
    const handle = handleFrom(event);
    const panel = handle && panelFrom(handle);
    if (!handle || !panel || !enabled(handle)) return;
    const bounds = geometry();
    const layout = resolveWorkbenchPanelLayout(preferences, bounds);
    let size = layout.sizes[panel];
    const delta = event.shiftKey ? 40 : 10;
    if (event.key === "Home") size = layout.limits[panel].min;
    else if (event.key === "End") size = layout.limits[panel].max;
    else if (panel === "report" && event.key === "ArrowUp") size += delta;
    else if (panel === "report" && event.key === "ArrowDown") size -= delta;
    else if (panel !== "report" && event.key === "ArrowRight") size += panel === "sidebar" ? delta : -delta;
    else if (panel !== "report" && event.key === "ArrowLeft") size += panel === "sidebar" ? -delta : delta;
    else return;
    event.preventDefault();
    event.stopPropagation();
    preferences = resizeWorkbenchPanel(preferences, bounds, panel, size);
    paint();
    save();
  }

  function doubleClick(event: MouseEvent) {
    const handle = handleFrom(event);
    const panel = handle && panelFrom(handle);
    if (!handle || !panel || !enabled(handle)) return;
    stop(false);
    preferences = { ...preferences };
    delete preferences[panel];
    paint();
    save();
  }

  function click(event: MouseEvent) {
    if (!(event.target instanceof Element) || !event.target.closest("[data-workbench-layout-reset]")) return;
    stop(false);
    preferences = {};
    paint();
    save();
  }

  function cancel() { stop(false); }
  function resize() { cancel(); schedulePaint(); }
  function reconcileNavigation() {
    cancel();
    if (frame) cancelAnimationFrame(frame);
    // Navigation changes the panel's default width; do not leave new content in the old layout until a later frame.
    paint();
  }
  root.dataset.workbenchResizableLayout = "true";
  paint();
  const observer = typeof ResizeObserver === "function" ? new ResizeObserver(schedulePaint) : null;
  const attributes = new MutationObserver(reconcileNavigation);
  attributes.observe(root, { attributes: true, attributeFilter: ["data-workbench-section", "data-workbench-stack-panels"] });
  observer?.observe(root);
  const main = root.querySelector(".workspace-main");
  if (main) observer?.observe(main);
  root.addEventListener("pointerdown", pointerDown);
  root.addEventListener("pointermove", pointerMove);
  root.addEventListener("pointerup", pointerUp);
  root.addEventListener("pointercancel", pointerCancel);
  root.addEventListener("lostpointercapture", pointerCancel);
  root.addEventListener("keydown", keyDown, true);
  root.addEventListener("dblclick", doubleClick);
  root.addEventListener("click", click);
  root.addEventListener("workbench-layout-mounted", schedulePaint);
  window.addEventListener("blur", cancel);
  window.addEventListener("resize", resize);
  document.addEventListener("fullscreenchange", resize);
  return () => {
    persistence.dispose();
    cancel();
    if (frame) cancelAnimationFrame(frame);
    observer?.disconnect();
    attributes.disconnect();
    root.removeEventListener("pointerdown", pointerDown);
    root.removeEventListener("pointermove", pointerMove);
    root.removeEventListener("pointerup", pointerUp);
    root.removeEventListener("pointercancel", pointerCancel);
    root.removeEventListener("lostpointercapture", pointerCancel);
    root.removeEventListener("keydown", keyDown, true);
    root.removeEventListener("dblclick", doubleClick);
    root.removeEventListener("click", click);
    root.removeEventListener("workbench-layout-mounted", schedulePaint);
    window.removeEventListener("blur", cancel);
    window.removeEventListener("resize", resize);
    document.removeEventListener("fullscreenchange", resize);
    root.removeAttribute("data-workbench-resizable-layout");
    for (const panel of ["sidebar", "inspector", "report"]) root.style.removeProperty(`--workbench-${panel}-size`);
  };
}
