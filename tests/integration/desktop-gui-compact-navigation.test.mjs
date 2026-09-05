import assert from "node:assert/strict";
import test from "node:test";
import {
  chromium,
  createDesktopShellRegressionEnvironment,
} from "./desktop-shell-regression.shared.mjs";
import { captureDesktopGuiArtifacts } from "./desktop-gui-artifacts.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

const COMPACT_VIEWPORT = { width: 820, height: 900 };
const MOBILE_VIEWPORT = { width: 430, height: 900 };

async function loadAt(page, url, viewport) {
  await page.setViewportSize(viewport);
  await page.goto(url, { waitUntil: "networkidle", timeout: 60_000 });
}

async function assertHubCompactNavigation(page) {
  await page.waitForSelector(".hub-nav--grouped");
  const layout = await page.evaluate(() => {
    const read = (selector) => {
      const element = document.querySelector(selector);
      const bounds = element?.getBoundingClientRect();
      const style = element ? getComputedStyle(element) : null;
      return bounds && style
        ? {
            bottom: bounds.bottom,
            height: bounds.height,
            left: bounds.left,
            overflowX: style.overflowX,
            right: bounds.right,
            top: bounds.top,
          }
        : null;
    };
    const event = document.querySelector(".hub-event-bar");
    return {
      documentWidth: document.documentElement.scrollWidth,
      event: read(".hub-event-bar"),
      eventGridRow: event ? getComputedStyle(event).gridRowStart : null,
      main: read(".hub-main"),
      nav: read(".hub-nav"),
      rail: read(".hub-rail"),
      viewportHeight: window.innerHeight,
      viewportWidth: window.innerWidth,
    };
  });

  assert.ok(layout.rail && layout.nav && layout.main && layout.event);
  assert.ok(layout.rail.height <= 140, "Hub compact rail must not consume the first screen");
  assert.ok(layout.nav.overflowX === "auto" || layout.nav.overflowX === "scroll");
  assert.equal(layout.eventGridRow, "3", "Hub event feedback must own the bottom grid row");
  assert.ok(layout.main.top >= layout.rail.bottom - 1);
  assert.ok(layout.main.bottom <= layout.event.top + 1, "Hub event bar must not cover main content");
  assert.ok(layout.main.height >= layout.viewportHeight * 0.58, "Hub main task surface must dominate compact windows");
  assert.ok(layout.event.height <= 72, "Hub event feedback should remain a compact status bar");
  assert.ok(layout.documentWidth <= layout.viewportWidth);
}

async function assertInstallerCompactNavigation(page) {
  await page.waitForSelector(".sidebar-tab.active");
  const layout = await page.evaluate(() => {
    const read = (selector) => {
      const element = document.querySelector(selector);
      const bounds = element?.getBoundingClientRect();
      const style = element ? getComputedStyle(element) : null;
      return bounds && style
        ? {
            bottom: bounds.bottom,
            flexDirection: style.flexDirection,
            height: bounds.height,
            left: bounds.left,
            overflowX: style.overflowX,
            right: bounds.right,
            top: bounds.top,
            width: bounds.width,
          }
        : null;
    };
    return {
      content: read(".workspace .content"),
      documentWidth: document.documentElement.scrollWidth,
      hero: read(".hero"),
      sidebar: read(".workspace .sidebar"),
      viewportHeight: window.innerHeight,
      viewportWidth: window.innerWidth,
    };
  });

  assert.ok(layout.hero && layout.sidebar && layout.content);
  assert.equal(layout.sidebar.flexDirection, "row");
  assert.ok(layout.sidebar.overflowX === "auto" || layout.sidebar.overflowX === "scroll");
  assert.ok(layout.sidebar.height <= 82, "Installer navigation must stay a compact task rail");
  assert.ok(layout.content.top >= layout.sidebar.bottom - 1);
  assert.ok(layout.content.top <= layout.viewportHeight * 0.46, "Installer work should begin on the first screen");
  assert.ok(Math.abs(layout.content.width - layout.sidebar.width) <= 2);
  assert.ok(layout.documentWidth <= layout.viewportWidth);
}

async function assertInstallerMobileWidth(page) {
  await page.waitForSelector(".sidebar-tab.active");
  const layout = await page.evaluate(() => {
    const bounds = (selector) => {
      const rect = document.querySelector(selector)?.getBoundingClientRect();
      return rect
        ? { bottom: rect.bottom, left: rect.left, right: rect.right, top: rect.top, width: rect.width }
        : null;
    };
    return {
      content: bounds(".workspace .content"),
      documentWidth: document.documentElement.scrollWidth,
      hero: bounds(".hero"),
      sidebar: bounds(".workspace .sidebar"),
      viewportHeight: window.innerHeight,
      viewportWidth: window.innerWidth,
    };
  });

  assert.ok(layout.hero && layout.sidebar && layout.content);
  assert.ok(layout.documentWidth <= layout.viewportWidth, "Installer must not overflow the mobile viewport");
  for (const surface of [layout.hero, layout.sidebar, layout.content]) {
    assert.ok(surface.left >= 0 && surface.right <= layout.viewportWidth + 1);
  }
  assert.ok(layout.content.top <= layout.viewportHeight * 0.62, "Installer work should remain visible on mobile");
}

test("compact desktop windows keep navigation shallow and work visible", async (t) => {
  const environment = await createDesktopShellRegressionEnvironment();
  let browser;

  try {
    browser = await launchIntegrationBrowser(chromium);
    for (const [shell, url, assertion] of [
      ["hub", environment.hubUrl, assertHubCompactNavigation],
      ["installer", environment.installerUrl, assertInstallerCompactNavigation],
    ]) {
      await t.test(shell, async () => {
        const page = await browser.newPage();
        try {
          await loadAt(page, url, COMPACT_VIEWPORT);
          await assertion(page);
        } catch (error) {
          await captureDesktopGuiArtifacts(page, {
            suite: "desktop-gui-compact-navigation",
            scenario: shell,
            viewport: COMPACT_VIEWPORT,
            error,
          });
          throw error;
        } finally {
          await page.close();
        }
      });
    }


    await t.test("installer mobile", async () => {
      const page = await browser.newPage();
      try {
        await loadAt(page, environment.installerUrl, MOBILE_VIEWPORT);
        await assertInstallerMobileWidth(page);
      } catch (error) {
        await captureDesktopGuiArtifacts(page, {
          suite: "desktop-gui-compact-navigation",
          scenario: "installer-mobile",
          viewport: MOBILE_VIEWPORT,
          error,
        });
        throw error;
      } finally {
        await page.close();
      }
    });
  } finally {
    await browser?.close();
    await environment.cleanup();
  }
}, { timeout: 180_000 });
