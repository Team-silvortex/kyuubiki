import assert from "node:assert/strict";
import test from "node:test";
import {
  assertNoPageErrors,
  chromium,
  createDesktopShellRegressionEnvironment,
} from "./desktop-shell-regression.shared.mjs";
import {
  createWorkbenchRegressionEnvironment,
} from "./workbench-shell-regression.shared.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

async function assertVisibleSurface(page, selector, label) {
  const surface = await page.locator(selector).evaluate((element) => {
    const rect = element.getBoundingClientRect();
    const interactiveCount = element.querySelectorAll(
      "button, input, select, textarea, iframe, [role=button]",
    ).length;
    const semanticCount = element.querySelectorAll(
      "h1, h2, h3, p, pre, strong, code",
    ).length;
    return {
      display: getComputedStyle(element).display,
      height: rect.height,
      interactiveCount,
      semanticCount,
      textLength: element.textContent.trim().length,
      width: rect.width,
    };
  });

  assert.notEqual(surface.display, "none", `${label} must not be display:none`);
  assert.ok(surface.width > 0 && surface.height > 0, `${label} must occupy visible space`);
  assert.ok(
    surface.interactiveCount > 0 || surface.semanticCount > 0,
    `${label} must contain semantic or interactive content`,
  );
  assert.ok(
    surface.interactiveCount > 0 || surface.textLength > 20,
    `${label} must not be an empty navigation shell`,
  );
}

async function verifyHubNavigation(page) {
  const targets = await page.locator(".hub-nav__item[data-target]").evaluateAll((items) =>
    items.map((item) => item.dataset.target),
  );
  assert.ok(targets.length >= 5, "Hub should expose its primary navigation");

  for (const target of targets) {
    await page.locator(`.hub-nav__item[data-target="${target}"]`).click();
    await page.waitForFunction(
      (currentTarget) =>
        document.querySelector(`.hub-nav__item[data-target="${currentTarget}"]`)
          ?.classList.contains("hub-nav__item--active"),
      target,
    );
    await assertVisibleSurface(page, "#hub-main", `Hub ${target}`);
    assert.ok(
      (await page.locator("#section-title").textContent())?.trim(),
      `Hub ${target} should publish a section title`,
    );
  }

  await page.locator(`.hub-nav__item[data-target="${targets[0]}"]`).click();
}

async function verifyInstallerNavigation(page) {
  const tabs = await page.locator(".sidebar-tab[data-tab]").evaluateAll((items) =>
    items.map((item) => item.dataset.tab),
  );
  assert.ok(tabs.length >= 8, "Installer should expose its primary navigation");

  for (const tab of tabs) {
    await page.locator(`.sidebar-tab[data-tab="${tab}"]`).click();
    const panel = `[data-panel="${tab}"]`;
    await page.waitForFunction(
      (selector) => document.querySelector(selector)?.classList.contains("panel-visible"),
      panel,
    );
    await assertVisibleSurface(page, panel, `Installer ${tab}`);
  }

  await page.locator(`.sidebar-tab[data-tab="${tabs[0]}"]`).click();
}

async function verifyWorkbenchNavigation(page) {
  const pages = await page.locator("[data-shell-page]").evaluateAll((items) =>
    items.map((item) => item.dataset.shellPage),
  );
  assert.deepEqual(pages.sort(), ["control", "workbench"]);

  for (const shellPage of pages) {
    if (shellPage === "control" && await page.locator('[data-shell-target="control"]:visible').count()) {
      await page.locator('[data-shell-target="control"]:visible').click();
    } else {
      await page.locator(`[data-shell-page="${shellPage}"]`).click();
    }
    const pane = `[data-shell-pane="${shellPage}"]`;
    await page.waitForFunction(
      (selector) => !document.querySelector(selector)?.classList.contains("hidden"),
      pane,
    );
    await assertVisibleSurface(page, pane, `Workbench ${shellPage}`);
  }

  await page.locator('[data-shell-target="control"]:visible').click();
}

test("desktop primary navigation forms a reversible non-empty loop", async () => {
  const desktop = await createDesktopShellRegressionEnvironment();
  const workbench = await createWorkbenchRegressionEnvironment();
  let browser;

  try {
    browser = await launchIntegrationBrowser(chromium);
    for (const [name, url, verify] of [
      ["Hub", desktop.hubUrl, verifyHubNavigation],
      ["Installer", desktop.installerUrl, verifyInstallerNavigation],
      ["Workbench", workbench.workbenchUrl, verifyWorkbenchNavigation],
    ]) {
      const page = await browser.newPage();
      try {
        await page.goto(url, { waitUntil: "networkidle", timeout: 60_000 });
        await verify(page);
        await assertNoPageErrors(page);
      } finally {
        await page.close();
      }
      assert.ok(name);
    }
  } finally {
    await browser?.close();
    await Promise.all([desktop.cleanup(), workbench.cleanup()]);
  }
}, { timeout: 180_000 });
