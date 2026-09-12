import { resolveImmersiveDockLayout, resizeImmersiveDock, type ImmersiveDockPreferences,
  type ImmersiveDockGeometry } from "./workbench-immersive-dock-layout";

export function installImmersiveDockResize(layout: HTMLElement, handle: HTMLElement,
  preferences: { current: ImmersiveDockPreferences }) {
  let frame = 0;
  let drag: { pointer: number; origin: number; size: number; before: ImmersiveDockPreferences;
    geometry: ImmersiveDockGeometry; windowWidth: number; windowHeight: number } | null = null;
  const chrome = layout.querySelector<HTMLElement>(".canvas-layout__chrome");
  const main = layout.querySelector<HTMLElement>(".canvas-layout__main");
  const stage = layout.querySelector<HTMLElement>('[data-workbench-viewport="stage"]');
  const geometry = (): ImmersiveDockGeometry => {
    const stacked = window.innerWidth <= 700, style = getComputedStyle(layout);
    const pixels = (value: string) => Number.parseFloat(value) || 0;
    return {
      width: layout.clientWidth, height: layout.clientHeight, stacked,
      gutter: stacked ? pixels(style.paddingTop) + pixels(style.paddingBottom) + handle.offsetHeight + 2 * pixels(style.rowGap)
        : pixels(style.paddingLeft) + pixels(style.paddingRight) + handle.offsetWidth + 2 * pixels(style.columnGap),
      // Include actual chrome, borders and spacing instead of treating the whole main panel as canvas.
      minMainSize: stacked ? innerHeight * 0.4 + Math.max(0,
        (main?.getBoundingClientRect().height ?? 0) - (stage?.getBoundingClientRect().height ?? 0)) + 2 : 0,
    };
  };
  function paint() {
    frame = 0;
    const bounds = geometry(), result = resolveImmersiveDockLayout(preferences.current, bounds);
    layout.style.setProperty("--workbench-immersive-dock-size", `${result.size}px`);
    layout.toggleAttribute("data-immersive-compact", bounds.stacked && result.size < 360);
    handle.setAttribute("aria-orientation", bounds.stacked ? "horizontal" : "vertical");
    handle.setAttribute("aria-valuemin", String(Math.floor(result.min)));
    handle.setAttribute("aria-valuemax", String(Math.ceil(result.max)));
    handle.setAttribute("aria-valuenow", String(Math.round(result.size)));
  }
  function schedule() { if (!frame) frame = requestAnimationFrame(paint); }
  function stop(commit: boolean) {
    if (!drag) return;
    const previous = drag;
    drag = null;
    if (!commit) preferences.current = previous.before;
    if (handle.hasPointerCapture(previous.pointer)) handle.releasePointerCapture(previous.pointer);
    layout.removeAttribute("data-immersive-resizing");
    if (frame) cancelAnimationFrame(frame);
    paint();
  }
  function pointerDown(event: PointerEvent) {
    if (event.button !== 0 || !event.isPrimary || drag) return;
    event.preventDefault();
    handle.focus({ preventScroll: true });
    handle.setPointerCapture(event.pointerId);
    const bounds = geometry();
    drag = { pointer: event.pointerId, origin: bounds.stacked ? event.clientY : event.clientX,
      size: resolveImmersiveDockLayout(preferences.current, bounds).size, before: preferences.current,
      geometry: bounds, windowWidth: innerWidth, windowHeight: innerHeight };
    layout.dataset.immersiveResizing = "true";
  }
  function pointerMove(event: PointerEvent) {
    if (!drag || drag.pointer !== event.pointerId) return;
    const bounds = geometry();
    if (innerWidth !== drag.windowWidth || innerHeight !== drag.windowHeight ||
      bounds.width !== drag.geometry.width || bounds.height !== drag.geometry.height) {
      stop(false);
      return;
    }
    const coordinate = bounds.stacked ? event.clientY : event.clientX;
    preferences.current = resizeImmersiveDock(drag.before, bounds, drag.size + coordinate - drag.origin);
    schedule();
  }
  function pointerUp(event: PointerEvent) {
    if (drag?.pointer !== event.pointerId) return;
    pointerMove(event);
    stop(true);
  }
  function pointerCancel(event: PointerEvent) { if (drag?.pointer === event.pointerId) stop(false); }
  function cancel() { stop(false); }
  function resize() { cancel(); schedule(); }
  function keyDown(event: KeyboardEvent) {
    if (drag) {
      if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); cancel(); }
      return;
    }
    const bounds = geometry(), current = resolveImmersiveDockLayout(preferences.current, bounds);
    const delta = event.shiftKey ? 40 : 10;
    let size = current.size;
    if (event.key === "Home") size = current.min;
    else if (event.key === "End") size = current.max;
    else if (event.key === (bounds.stacked ? "ArrowDown" : "ArrowRight")) size += delta;
    else if (event.key === (bounds.stacked ? "ArrowUp" : "ArrowLeft")) size -= delta;
    else return;
    event.preventDefault();
    event.stopPropagation();
    preferences.current = resizeImmersiveDock(preferences.current, bounds, size);
    paint();
  }
  function reset() {
    cancel();
    const { key } = resolveImmersiveDockLayout(preferences.current, geometry());
    preferences.current = { ...preferences.current };
    delete preferences.current[key];
    paint();
  }
  const observer = typeof ResizeObserver === "function" ? new ResizeObserver(schedule) : null;
  observer?.observe(layout);
  if (chrome) observer?.observe(chrome);
  handle.addEventListener("pointerdown", pointerDown);
  handle.addEventListener("pointermove", pointerMove);
  handle.addEventListener("pointerup", pointerUp);
  handle.addEventListener("pointercancel", pointerCancel);
  handle.addEventListener("lostpointercapture", pointerCancel);
  handle.addEventListener("keydown", keyDown);
  handle.addEventListener("dblclick", reset);
  window.addEventListener("blur", cancel);
  window.addEventListener("resize", resize);
  document.addEventListener("fullscreenchange", resize);
  paint();
  return () => {
    cancel();
    if (frame) cancelAnimationFrame(frame);
    observer?.disconnect();
    handle.removeEventListener("pointerdown", pointerDown);
    handle.removeEventListener("pointermove", pointerMove);
    handle.removeEventListener("pointerup", pointerUp);
    handle.removeEventListener("pointercancel", pointerCancel);
    handle.removeEventListener("lostpointercapture", pointerCancel);
    handle.removeEventListener("keydown", keyDown);
    handle.removeEventListener("dblclick", reset);
    window.removeEventListener("blur", cancel);
    window.removeEventListener("resize", resize);
    document.removeEventListener("fullscreenchange", resize);
    layout.style.removeProperty("--workbench-immersive-dock-size");
    layout.removeAttribute("data-immersive-compact");
  };
}
