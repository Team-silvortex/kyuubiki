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
