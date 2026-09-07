const VIEWBOX_WIDTH = 980;
const VIEWBOX_HEIGHT = 460;

export function fitWorkbenchViewport(width: number, height: number) {
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    return { width: 0, height: 0 };
  }
  const scale = Math.min(width / VIEWBOX_WIDTH, height / VIEWBOX_HEIGHT);
  return { width: VIEWBOX_WIDTH * scale, height: VIEWBOX_HEIGHT * scale };
}

// Size the surface itself, not just its SVG viewBox, so picking and the WebGL overlay stay aligned.
export function observeWorkbenchViewportFit(stage: HTMLElement) {
  let frame: number | null = null;
  let disposed = false;
  const measure = () => {
    frame = null;
    if (disposed) return;
    const size = fitWorkbenchViewport(stage.clientWidth, stage.clientHeight);
    stage.style.setProperty("--workbench-fit-width", `${size.width}px`);
    stage.style.setProperty("--workbench-fit-height", `${size.height}px`);
  };
  const schedule = () => {
    if (!disposed && frame === null) frame = window.requestAnimationFrame(measure);
  };
  const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(schedule);
  observer?.observe(stage);
  window.addEventListener("resize", schedule);
  measure();
  return () => {
    disposed = true;
    observer?.disconnect();
    window.removeEventListener("resize", schedule);
    if (frame !== null) window.cancelAnimationFrame(frame);
    stage.style.removeProperty("--workbench-fit-width");
    stage.style.removeProperty("--workbench-fit-height");
  };
}
