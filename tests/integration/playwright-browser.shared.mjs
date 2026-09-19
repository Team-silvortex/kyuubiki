import assert from "node:assert/strict";

function isMissingBundledBrowser(error) {
  const message = String(error?.message || error || "");
  return message.includes("Executable doesn't exist") || message.includes("playwright install");
}

export async function launchIntegrationBrowser(chromium, options = {}) {
  const executablePath = process.env.KYUUBIKI_PLAYWRIGHT_EXECUTABLE_PATH?.trim();
  if (executablePath) {
    return chromium.launch({ headless: true, ...options, executablePath });
  }

  try {
    return await chromium.launch({ headless: true, ...options });
  } catch (error) {
    if (!isMissingBundledBrowser(error)) throw error;

    const channel = process.env.KYUUBIKI_PLAYWRIGHT_CHANNEL?.trim() || "chrome";
    return chromium.launch({ headless: true, ...options, channel });
  }
}

export async function clickIntegrationControl(page, selector, label) {
  const candidates = page.locator(selector);
  const target = candidates.first();
  await waitForVisibleOrPageError(page, target, label);
  assert.equal(await candidates.count(), 1, `${label} should resolve to one visible control`);
  await target.click({ timeout: 15_000 });
  return target;
}

export async function waitForVisibleOrPageError(page, locator, label, timeout = 30_000) {
  let rejectPageError;
  const pageError = new Promise((_, reject) => {
    rejectPageError = (error) => reject(new Error(`${label} aborted after client error: ${error.message}`));
    page.once("pageerror", rejectPageError);
  });
  try {
    await Promise.race([
      locator.waitFor({ state: "visible", timeout }),
      pageError,
    ]);
  } finally {
    page.off("pageerror", rejectPageError);
  }
}

// Chromium rejects Browser.setWindowBounds while a native fullscreen element is active.
// Keep the emulation session alive until its page/context closes: detaching it resets metrics.
const fullscreenViewportSessions = new WeakMap();

export async function resizeFullscreenViewport(page, { width, height }) {
  assert.ok(Number.isInteger(width) && width > 0, "fullscreen viewport width must be positive");
  assert.ok(Number.isInteger(height) && height > 0, "fullscreen viewport height must be positive");
  assert.equal(await page.evaluate(() => !!document.fullscreenElement), true,
    "fullscreen resize must exercise native fullscreen, not the window-local fallback");
  let session = fullscreenViewportSessions.get(page);
  if (!session) {
    session = await page.context().newCDPSession(page);
    fullscreenViewportSessions.set(page, session);
  }
  // Emulate the content viewport without asking Chromium to resize its fullscreen OS window.
  // page.viewportSize() remains Playwright's original bookkeeping; use DOM metrics in this mode.
  await session.send("Emulation.setDeviceMetricsOverride", {
    width, height, screenWidth: width, screenHeight: height,
    deviceScaleFactor: await page.evaluate(() => window.devicePixelRatio),
    mobile: false,
  });
  await page.waitForFunction(([w, h]) => !!document.fullscreenElement &&
    window.innerWidth === w && window.innerHeight === h, [width, height]);
}
