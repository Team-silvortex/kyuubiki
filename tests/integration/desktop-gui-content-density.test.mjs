import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { chromium, ROOT, createDesktopShellRegressionEnvironment } from "./desktop-shell-regression.shared.mjs";
import { createWorkbenchRegressionEnvironment } from "./workbench-shell-regression.shared.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { captureDesktopGuiArtifacts } from "./desktop-gui-artifacts.mjs";

async function openSurface(page, url, surface) {
  await page.goto(url, { waitUntil: "networkidle" });
  if (surface === "workbench") await page.locator(".viewer__back-button").click();
  if (surface === "hub") await page.locator('[data-target="runtimes"]').click();
  if (surface === "installer") await page.locator('[data-tab="services"]').click();
}

async function renderUnevenSummary(page, selector) {
  return page.evaluate(async (selector) => {
    const { renderRuntimeStatusPlane } = await import("./shared/runtime-status-summary.js");
    const target = document.querySelector(selector);
    renderRuntimeStatusPlane(target, {
      summary: "Density regression",
      runtimes: [
        { id: "long", label: "Long runtime description", status: "stopped", note: "详细说明 / Ausführliche Laufzeitinformationen ".repeat(30) },
        { id: "short", label: "Agent A", status: "stopped", note: "Local runtime summary" },
        { id: "other", label: "Agent B", status: "running", note: "Ready" },
      ],
      meshPanels: [
        { label: "Local shell", title: "Frontend runtime", pills: [{ label: "Status", value: "stopped" }], items: [] },
        { label: "Long detail", title: "Runtime information", pills: [], items: [{ title: "Deployment", copy: "Full detail remains available. ".repeat(30) }] },
      ], filters: [], topology: [], detailSelection: null,
    });
    const rect = (node) => { const r = node.getBoundingClientRect(); return { top:r.top, bottom:r.bottom, height:r.height, left:r.left, right:r.right }; };
    return {
      cards: Array.from(target.querySelectorAll(".desktop-runtime-plane__card"), rect),
      panels: Array.from(target.querySelectorAll(".desktop-runtime-plane__mesh-panel"), rect),
      longText: target.textContent.includes("Ausführliche Laufzeitinformationen"),
      width: target.clientWidth, scrollWidth: target.scrollWidth,
    };
  }, selector);
}

test("runtime summaries keep short cards compact across all three desktop shells", async () => {
  const desktop = await createDesktopShellRegressionEnvironment();
  const workbench = await createWorkbenchRegressionEnvironment();
  let browser;
  try {
    browser = await launchIntegrationBrowser(chromium);
    for (const [surface, url, selector] of [
      ["workbench", workbench.workbenchUrl, "#status-plane"],
      ["hub", desktop.hubUrl, "#runtime-status-plane"],
      ["installer", desktop.installerUrl, "#service-status-plane"],
    ]) {
      for (const viewport of [{ width:1440, height:1000 }, { width:1080, height:760 }, { width:768, height:900 }, { width:390, height:844 }]) {
        const page = await browser.newPage({ viewport });
        try {
          await openSurface(page, url, surface);
          const layout = await renderUnevenSummary(page, selector);
          assert.equal(layout.cards.length, 3);
          assert.ok(layout.cards[1].height <= 180, `${surface}: short runtime card stretched to ${layout.cards[1].height}px`);
          assert.ok(layout.panels[0].height <= 200, `${surface}: brief mesh panel stretched to ${layout.panels[0].height}px`);
          assert.ok(Math.min(...layout.panels.map(p => p.top)) >= Math.max(...layout.cards.map(c => c.bottom)) - 1, "Details must occupy their own row after runtime summaries");
          assert.ok(layout.longText, "Compaction must not remove long descriptions");
          assert.ok(layout.scrollWidth <= layout.width + 1, "Long descriptions must wrap inside the status plane");
        } catch (error) {
          await captureDesktopGuiArtifacts(page, { suite:"desktop-gui-content-density", scenario:surface, viewport, error });
          throw error;
        } finally { await page.close(); }
      }
    }
  } finally {
    await browser?.close();
    await workbench.cleanup();
    await desktop.cleanup();
  }
}, { timeout:180_000 });

test("Workbench control overview stacks auxiliary cards without reserved empty height", async () => {
  const environment = await createWorkbenchRegressionEnvironment();
  let browser;
  try {
    browser = await launchIntegrationBrowser(chromium);
    for (const viewport of [{ width:1440, height:1000 }, { width:1080, height:760 }, { width:390, height:844 }]) {
      const page = await browser.newPage({ viewport });
      try {
        await openSurface(page, environment.workbenchUrl, "workbench");
        await page.evaluate(() => {
          const invoke = window.__TAURI__.core.invoke;
          window.__TAURI__.core.invoke = async (command, payload) => command === "service_status" ? {
            rendered: "frontend stopped | orchestrator stopped | agents stopped",
            summary: {
              deployment_mode:"local", control_mode:"standalone", authority_mode:"self_directed",
              frontend_status:"stopped", orchestrator_status:"stopped", agent_count:2,
              agents:[{label:"Agent[5001]",status:"stopped"},{label:"Agent[5002]",status:"stopped"}],
            },
          } : invoke(command,payload);
        });
        await page.locator('.panel--console [data-action="refresh"]').click();
        await page.waitForSelector("#status-plane .desktop-runtime-plane__card--state-stopped");
        assert.equal(await page.locator(".panel--console .panel__title").textContent(), "Runtime Console");
        assert.equal(await page.locator(".panel--runtime .panel__title").textContent(), "Runtime");
        assert.equal(await page.locator(".panel--controls .panel__title").textContent(), "Viewer controls");
        await page.locator("#shell-language-select").selectOption("zh");
        await page.waitForFunction(() => document.querySelector(".panel--console .panel__title")?.textContent === "运行控制台");
        assert.equal(await page.locator(".panel--runtime .panel__title").textContent(), "运行时");
        assert.equal(await page.locator(".panel--controls .panel__title").textContent(), "视图控制");
        await page.locator("#shell-language-select").selectOption("en");
        await page.waitForFunction(() => document.querySelector(".panel--console .panel__title")?.textContent === "Runtime Console");
        const layout = await page.evaluate(() => {
          const rect = s => { const r=document.querySelector(s).getBoundingClientRect(); return { height:r.height, top:r.top, bottom:r.bottom }; };
          return { runtime:rect(".panel--runtime"), controls:rect(".panel--controls"), ribbon:rect(".workflow-ribbon"), documentWidth:document.documentElement.scrollWidth, viewportWidth:innerWidth };
        });
        assert.ok(layout.runtime.height <= 240, `Runtime metadata reserves ${layout.runtime.height}px`);
        assert.ok(layout.controls.top - layout.runtime.bottom <= 24, "Auxiliary cards should form a compact stack");
        if (viewport.width >= 1080) assert.ok(layout.ribbon.height <= 140, "Workflow hints must not reserve oversized tiles");
        assert.ok(layout.documentWidth <= layout.viewportWidth + 1, "Control overview must not scroll horizontally");
        await page.locator('[data-console-tab="logs"]').click();
        await page.waitForSelector("#logs-panel:not(.is-hidden)");
        assert.match(await page.locator("#log-output").textContent(), /log mock/);
        await page.locator('[data-console-tab="status"]').click();
        assert.equal(await page.locator("#status-plane .desktop-runtime-plane__card").count(), 4);
        await page.locator('#status-plane [data-runtime-detail="runtime:frontend"]').click();
        await page.waitForSelector(".desktop-runtime-detail");
        assert.equal(await page.locator(".desktop-runtime-detail__title").textContent(), "Frontend runtime");
        await page.locator('#status-plane [data-runtime-detail="runtime:orchestrator"]').focus();
        await page.keyboard.press("Enter");
        assert.equal(await page.locator(".desktop-runtime-detail__title").textContent(), "Orchestrator runtime");
        await page.locator('#status-plane [data-runtime-detail="runtime:frontend"]').click();
        if (viewport.width === 1440) {
          await page.evaluate(() => Promise.all(document.querySelector("#status-plane").getAnimations({subtree:true}).map(animation => animation.finished.catch(() => {}))));
          await page.screenshot({ path:path.join(ROOT,"tmp/desktop-control-density.png"), fullPage:true });
        }
      } catch (error) {
        await captureDesktopGuiArtifacts(page, { suite:"desktop-gui-content-density", scenario:"workbench-control", viewport, error });
        throw error;
      } finally { await page.close(); }
    }
  } finally { await browser?.close(); await environment.cleanup(); }
}, { timeout:120_000 });

test("informational overview grids do not stretch short cards to match long siblings", async () => {
  let browser;
  try {
    browser = await launchIntegrationBrowser(chromium);
    const page = await browser.newPage({ viewport:{width:1280,height:900} });
    await page.setContent('<main style="width:900px;max-width:100%"></main>');
    for (const file of [
      "apps/hub-gui/ui/styles/hub-panels.css",
      "apps/installer-gui/ui/styles/installer-workspace/layout.css",
      "apps/installer-gui/ui/styles/installer-update.css",
      "apps/frontend/src/app/styles/globals-workflow-diagnostics.css",
    ]) await page.addStyleTag({ content:readFileSync(path.join(ROOT,file),"utf8") });
    for (const name of ["hub-overview-strip", "service-grid", "doctor-grid", "integrity-summary-grid", "update-summary-grid", "runtime-overview-grid"]) {
      const sizes = await page.evaluate(name => {
        const grid=document.createElement("section"); grid.className=name;
        for (const long of [false,true]) { const card=document.createElement("article"); card.textContent=long ? "Long translated information. ".repeat(100) : "Ready"; grid.append(card); }
        document.querySelector("main").replaceChildren(grid);
        return Array.from(grid.children, c=>c.getBoundingClientRect().height);
      }, name);
      assert.ok(sizes[0] < sizes[1]/2, `${name}: short card stretched (${sizes})`);
    }
  } finally { await browser?.close(); }
}, { timeout:60_000 });
