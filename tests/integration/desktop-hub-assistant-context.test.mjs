import assert from "node:assert/strict";
import test from "node:test";
import { chromium, createDesktopShellRegressionEnvironment, assertNoPageErrors } from "./desktop-shell-regression.shared.mjs";
import { launchIntegrationBrowser } from "./playwright-browser.shared.mjs";
import { captureDesktopGuiArtifacts } from "./desktop-gui-artifacts.mjs";

const LONG_PATH = `/tmp/${"long-runtime-directory/".repeat(20)}runtime`;
const REPORT = {
  rendered: ["deployment-mode: local", "control-mode: standalone", "authority-mode: self_directed",
    "runtime-policy: installer-managed", "lifecycle-policy: explicit-background (closing GUI does not stop services)",
    `lifecycle-lock: ${LONG_PATH}/state/run/lifecycle.lock`, `runtime-root: ${LONG_PATH}`,
    `runtime-service[agent]: installed -> ${LONG_PATH}/bin/kyuubiki-cli`,
    "orchestrator: running on http://127.0.0.1:4000 (pid 40)",
    "frontend: disabled by runtime configuration", "agent[5001]: running on tcp://127.0.0.1:5001 (pid 41)",
    "agent[5002]: stopped", '<img src="invalid" onerror="window.runtimeInjected = true">'].join("\n"),
  summary: { orchestrator_status: "running", frontend_status: "disabled",
    agents: [{ label: "agent[5001]", status: "running" }, { label: "agent[5002]", status: "stopped" }] },
};

async function preparePage(browser, environment, viewport, language = "en") {
  const page = await browser.newPage({ viewport });
  await page.route("**/mock-tauri.js", async (route) => {
    const response = await route.fetch();
    await route.fulfill({ response, body: `${await response.text()}\n(() => {
      const invoke = window.__TAURI__.core.invoke;
      window.__TAURI__.core.invoke = async (command, payload) => {
        if (command === "service_status") return ${JSON.stringify(REPORT)};
        if (command === "get_global_language_preference") return { language: ${JSON.stringify(language)} };
        return invoke(command, payload);
      };
    })();` });
  });
  await page.goto(environment.hubUrl, { waitUntil: "networkidle" });
  await page.waitForFunction(() => document.querySelector("#assistant-context-runtime")?.textContent.startsWith("2/4"));
  await page.locator("#hub-assistant-fab").click();
  return page;
}

async function assertWidth(page) {
  const widths = await page.evaluate(() => [...document.querySelectorAll(
    ".hub-assistant-sheet, .hub-assistant-runtime__body, .hub-assistant-runtime__fields, .hub-assistant-runtime__raw",
  )].filter((element) => element.getBoundingClientRect().width > 0).map((element) => ({
    selector: element.className, actual: element.scrollWidth, allowed: element.clientWidth,
  })));
  for (const width of widths) assert.ok(width.actual <= width.allowed + 1, JSON.stringify(width));
}

test("Hub assistant uses a collapsed, bounded runtime list across desktop, narrow and RTL layouts", { timeout: 180_000 }, async (t) => {
  const environment = await createDesktopShellRegressionEnvironment();
  const browser = await launchIntegrationBrowser(chromium);
  try {
    for (const [viewport, language] of [
      [{ width: 1440, height: 900 }, "zh"],
      [{ width: 1024, height: 700 }, "en"],
      [{ width: 390, height: 844 }, "ar"],
    ]) {
      await t.test(`${language} ${viewport.width}`, async () => {
        const page = await preparePage(browser, environment, viewport, language);
        try {
          if (language === "ar") await page.evaluate(() => { document.documentElement.dir = "rtl"; });
          const details = page.locator("#assistant-runtime-details");
          const summary = details.locator(":scope > summary");
          assert.equal(await details.getAttribute("open"), null);
          assert.ok((await details.boundingBox()).height < 80, "raw status must not fill the collapsed context");
          const shortcuts = await page.locator(".hub-assistant-sheet .hub-side-shortcuts").boundingBox();
          const sheet = await page.locator(".hub-assistant-sheet").boundingBox();
          assert.ok(shortcuts.y + shortcuts.height <= sheet.y + sheet.height, "quick actions should be visible without scrolling");
          await assertWidth(page);
          await summary.focus();
          await page.keyboard.press("Enter");
          assert.equal(await details.getAttribute("open"), "");
          assert.equal(await details.locator("[data-runtime-services] li").count(), 4);
          assert.equal(await details.locator('[data-state="running"]').count(), 2);
          assert.equal(await details.locator('[data-state="disabled"]').count(), 1);
          assert.equal(await details.locator('[data-state="stopped"]').count(), 1);
          const listLayout = await details.locator(".hub-assistant-runtime__body").evaluate((body) => {
            const services = body.querySelector("[data-runtime-service-group]").getBoundingClientRect();
            const configuration = body.querySelector("[data-runtime-configuration]").getBoundingClientRect();
            return { servicesWidth: services.width, bodyWidth: body.clientWidth,
              servicesBottom: services.bottom, configurationTop: configuration.top };
          });
          assert.ok(listLayout.servicesWidth >= listLayout.bodyWidth * 0.85, "services must use the list width, not shrink into a column");
          assert.ok(listLayout.configurationTop >= listLayout.servicesBottom - 1, "configuration must follow the services vertically");
          await details.locator("[data-runtime-configuration] > summary").click();
          assert.equal(await details.locator("[data-runtime-fields] > div").count(), 8);
          await assertWidth(page);
          await details.locator("[data-runtime-raw-label]").click();
          assert.equal(await details.locator("[data-runtime-raw]").textContent(), REPORT.rendered);
          assert.equal(await details.locator("img, script").count(), 0, "diagnostics must be rendered as text, never HTML");
          assert.equal(await page.evaluate(() => Boolean(window.runtimeInjected)), false);
          await assertWidth(page);
          await page.locator("#hub-assistant-close").click();
          await page.locator("#hub-assistant-fab").click();
          assert.equal(await details.getAttribute("open"), "", "reopening the assistant must preserve the chosen disclosure");
          await assertNoPageErrors(page);
        } catch (error) {
          await captureDesktopGuiArtifacts(page, { suite: "desktop-hub-assistant-context", scenario: language, viewport, error });
          throw error;
        } finally { await page.close(); }
      });
    }
  } finally { await browser.close(); await environment.cleanup(); }
});

test("runtime refresh and language changes preserve disclosure and focus while failure clears stale state", { timeout: 90_000 }, async () => {
  const environment = await createDesktopShellRegressionEnvironment();
  const browser = await launchIntegrationBrowser(chromium);
  const page = await preparePage(browser, environment, { width: 1280, height: 800 });
  try {
    await page.evaluate(async (report) => {
      const { createAssistantRuntimeView } = await import("./hub-assistant-runtime.js");
      window.renderTestRuntime = createAssistantRuntimeView(document.querySelector("#assistant-runtime-details"));
      window.renderTestRuntime(report, "en");
    }, REPORT);
    const details = page.locator("#assistant-runtime-details");
    await details.locator(":scope > summary").click();
    const config = details.locator("[data-runtime-configuration] > summary");
    await config.click();
    await config.focus();
    await page.evaluate((report) => window.renderTestRuntime(report, "zh"), REPORT);
    assert.equal(await details.getAttribute("open"), "");
    assert.equal(await details.locator("[data-runtime-configuration]").getAttribute("open"), "");
    assert.equal(await config.textContent(), "配置与路径");
    assert.equal(await config.evaluate((element) => document.activeElement === element), true);
    assert.equal(await page.locator("#assistant-context-runtime").textContent(), "2/4 运行中");
    await page.evaluate(() => window.renderTestRuntime({ rendered: "Error: timeout", summary: null, failed: true }, "zh"));
    assert.equal(await page.locator("#assistant-context-runtime").textContent(), "无法读取状态");
    assert.equal(await details.locator("[data-runtime-services] li").count(), 0);
    assert.equal(await details.locator("[data-runtime-raw]").textContent(), "Error: timeout");
    assert.equal(await details.getAttribute("open"), "");
    await page.evaluate((report) => window.renderTestRuntime(report, "zh"), REPORT);
    assert.equal(await details.locator("[data-runtime-services] li").count(), 4);
    await details.locator(":scope > summary").focus();
    await page.keyboard.press("Space");
    await page.evaluate((report) => window.renderTestRuntime(report, "en"), REPORT);
    assert.equal(await details.getAttribute("open"), null, "refresh must not force a collapsed section open");
    await assertNoPageErrors(page);
  } finally { await page.close(); await browser.close(); await environment.cleanup(); }
});
