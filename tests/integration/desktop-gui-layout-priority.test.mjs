import assert from "node:assert/strict";
import test from "node:test";
import {
  chromium,
  createDesktopShellRegressionEnvironment,
} from "./desktop-shell-regression.shared.mjs";
import { captureDesktopGuiArtifacts } from "./desktop-gui-artifacts.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";

async function installerLayout(page, viewport) {
  await page.setViewportSize(viewport);
  await page.goto(page.url(), { waitUntil: "networkidle", timeout: 60_000 });
  await page.waitForSelector(".workspace .content");
  const layout = await page.evaluate(() => {
    const rect = (selector) => {
      const bounds = document.querySelector(selector)?.getBoundingClientRect();
      return bounds
        ? {
            bottom: bounds.bottom,
            height: bounds.height,
            left: bounds.left,
            right: bounds.right,
            top: bounds.top,
            width: bounds.width,
          }
        : null;
    };
    return {
      hero: rect(".hero"),
      workspace: rect(".workspace"),
      sidebar: rect(".workspace .sidebar"),
      content: rect(".workspace .content"),
      completionBanner: rect("#completion-banner:not([hidden])"),
      completionPosition: window.getComputedStyle(
        document.querySelector("#completion-banner"),
      ).position,
      viewportHeight: window.innerHeight,
    };
  });

  assert.ok(
    layout.hero
      && layout.workspace
      && layout.sidebar
      && layout.content
      && layout.completionBanner,
  );
  assert.equal(layout.completionPosition, "static", "Completion feedback should own a layout row");
  assert.ok(
    layout.workspace.top - layout.hero.bottom <= 24,
    "Completion feedback must not reserve space between the hero and workspace",
  );
  assert.ok(
    layout.workspace.bottom <= layout.completionBanner.top,
    "Completion status bar must not overlap the operational workspace",
  );
  assert.ok(
    layout.completionBanner.height <= 64
      && layout.completionBanner.bottom <= layout.viewportHeight,
    "Completion status bar should remain compact and inside the viewport",
  );
  assert.ok(
    layout.hero.height <= layout.viewportHeight * 0.34,
    "Installer hero should not displace the operational workspace",
  );
  const visibleWorkspaceHeight = Math.min(layout.workspace.bottom, layout.viewportHeight)
    - Math.max(layout.workspace.top, 0);
  assert.ok(
    visibleWorkspaceHeight >= layout.viewportHeight * 0.56,
    "Installer workspace should own most of the visible viewport",
  );
  assert.ok(
    layout.content.width >= layout.sidebar.width * 2.25,
    "Installer content should remain wider than navigation",
  );
  assert.ok(
    Math.abs(layout.content.top - layout.sidebar.top) <= 1,
    "Installer content and navigation should remain side-by-side at desktop minimum width",
  );
}

async function assertSectionedInstallerPanels(page) {
  for (const [panelName, expectedSections] of [
    ["updates", ["overview", "runtime", "source", "delivery", "staging"]],
    ["remote", ["target", "authority", "certificates", "fleet", "agent"]],
  ]) {
    await page.locator(`.sidebar-tab[data-tab="${panelName}"]`).click();
    const panel = page.locator(`[data-panel="${panelName}"].panel-visible`);
    const sectionNames = await panel.locator("[data-installer-section-target]").evaluateAll(
      (buttons) => buttons.map((button) => button.dataset.installerSectionTarget),
    );
    assert.deepEqual(sectionNames, expectedSections);
    const firstTab = panel.locator("[data-installer-section-target]").first();
    await firstTab.focus();
    await firstTab.press("ArrowRight");
    assert.equal(
      await panel.locator(".installer-section-tab--active").getAttribute(
        "data-installer-section-target",
      ),
      expectedSections[1],
    );

    for (const sectionName of sectionNames) {
      await panel.locator(`[data-installer-section-target="${sectionName}"]`).click();
      const state = await panel.evaluate((element, activeSection) => {
        const section = element.querySelector(".installer-panel-section--active");
        const sidebar = document.querySelector(".sidebar");
        return {
          activeSection: section?.dataset.installerSection,
          activeSectionCount: element.querySelectorAll(".installer-panel-section--active").length,
          activeTabCount: element.querySelectorAll(".installer-section-tab--active").length,
          panelClientHeight: element.clientHeight,
          panelOverflowY: getComputedStyle(element).overflowY,
          panelScrollHeight: element.scrollHeight,
          sectionClientHeight: section?.clientHeight || 0,
          sectionScrollHeight: section?.scrollHeight || 0,
          sidebarClientHeight: sidebar?.clientHeight || 0,
          sidebarScrollHeight: sidebar?.scrollHeight || 0,
          expected: activeSection,
        };
      }, sectionName);
      assert.equal(state.activeSection, state.expected);
      assert.equal(state.activeSectionCount, 1);
      assert.equal(state.activeTabCount, 1);
      assert.equal(state.panelOverflowY, "hidden");
      assert.ok(state.panelScrollHeight <= state.panelClientHeight + 2);
      assert.ok(state.sectionClientHeight > 120);
      assert.ok(
        state.sectionScrollHeight / state.sectionClientHeight <= 6,
        `${panelName}/${sectionName} should remain a bounded task section`,
      );
      assert.ok(state.sidebarScrollHeight <= state.sidebarClientHeight + 2);
    }
  }
}

test("desktop shell layout keeps operational workspaces dominant", async () => {
  const environment = await createDesktopShellRegressionEnvironment();
  let browser;

  try {
    browser = await launchIntegrationBrowser(chromium);
    for (const viewport of [
      { width: 1440, height: 1100 },
      { width: 1180, height: 920 },
      { width: 1080, height: 760 },
    ]) {
      const page = await browser.newPage();
      try {
        await page.goto(environment.installerUrl, {
          waitUntil: "networkidle",
          timeout: 60_000,
        });
        await installerLayout(page, viewport);
        if (viewport.width === 1080 && viewport.height === 760) {
          await assertSectionedInstallerPanels(page);
        }
      } catch (error) {
        await captureDesktopGuiArtifacts(page, {
          suite: "desktop-gui-layout-priority",
          scenario: "installer-workspace-priority",
          viewport,
          error,
        });
        throw error;
      } finally {
        await page.close();
      }
    }
  } finally {
    await browser?.close();
    await environment.cleanup();
  }
}, { timeout: 180_000 });
