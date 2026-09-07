type FullscreenDocument = {
  fullscreenElement?: Element | null;
  exitFullscreen?: () => void | Promise<void>;
};

export function isWorkbenchViewportFullscreen(
  document: FullscreenDocument,
  target: Element | null | undefined,
): boolean {
  return Boolean(target && document.fullscreenElement === target);
}

export async function exitWorkbenchViewportFullscreen(
  document: FullscreenDocument,
  target: Element | null | undefined,
): Promise<void> {
  if (!isWorkbenchViewportFullscreen(document, target)) return;
  if (typeof document.exitFullscreen !== "function") return;
  await document.exitFullscreen();
}
