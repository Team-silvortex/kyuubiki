import assert from "node:assert/strict";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

const requireFromFrontend = createRequire(`${ROOT}/apps/frontend/package.json`);
export const { chromium } = requireFromFrontend("playwright");

const MIME_TYPES = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".png": "image/png",
};

function injectMockScript(indexHtml) {
  return indexHtml.replace(
    '<script type="module" src="./app.js"></script>',
    '<script src="./mock-tauri.js"></script>\n    <script type="module" src="./app.js"></script>',
  );
}

function copyWorkbenchPreview() {
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), "kyuubiki-workbench-regression-"));
  const destination = path.join(tempRoot, "workbench-gui");
  cpSync(path.join(ROOT, "apps", "workbench-gui", "ui"), destination, { recursive: true });
  cpSync(path.join(ROOT, "apps", "desktop-shared", "ui"), path.join(destination, "desktop-shared", "ui"), {
    recursive: true,
  });
  const indexPath = path.join(destination, "index.html");
  writeFileSync(indexPath, injectMockScript(readFileSync(indexPath, "utf8")));
  return { tempRoot, destination };
}

function workbenchMockSource() {
  const embeddedHtml = [
    "<!doctype html>",
    "<html><body style='margin:0;background:#111827;color:#dbe7f5;font-family:sans-serif'>",
    "<main style='padding:24px'>Embedded workbench mock</main>",
    "</body></html>",
  ].join("");
  const workbenchUrl = `data:text/html;charset=utf-8,${encodeURIComponent(embeddedHtml)}`;

  return `(() => {
  window.__mockErrors = [];
  window.__mockInvocations = [];
  window.addEventListener("error", (event) => {
    window.__mockErrors.push({
      type: "error",
      message: event.message,
      filename: event.filename,
    });
  });
  window.addEventListener("unhandledrejection", (event) => {
    window.__mockErrors.push({
      type: "unhandledrejection",
      message: String(event.reason?.message || event.reason || "unknown rejection"),
    });
  });

  const originalFetch = window.fetch.bind(window);
  window.fetch = async (input, init) => {
    const url = new URL(typeof input === "string" ? input : input.url, window.location.href);
    if (url.origin === "http://127.0.0.1:4000" || url.origin === "http://localhost:4000") {
      if (url.pathname === "/api/health") {
        return new Response(JSON.stringify({ status: "ok", nodes: [] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      return new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    }
    return originalFetch(input, init);
  };

  window.__TAURI__ = {
    core: {
      invoke: async (command, payload) => {
        window.__mockInvocations.push({ command, payload });
        switch (command) {
          case "get_global_language_preference":
            return { language: "en" };
          case "set_global_language_preference":
            return { language: payload?.payload?.language || "en" };
          case "workbench_environment":
            return {
              workbench_url: ${JSON.stringify(workbenchUrl)},
              orchestrator_url: "http://127.0.0.1:4000",
              deployment_mode: "direct_mesh_gui",
              host_platform: "macos",
            };
          case "service_status":
            return {
              rendered: "frontend healthy | orchestrator healthy | agents healthy",
              summary: {
                overall_status: "healthy",
                entries: [],
              },
            };
          case "read_runtime_log": {
            const service = payload?.payload?.service || payload?.service || "frontend";
            return {
              service,
              rendered: service + " log mock",
            };
          }
          case "guarded_mutation_action":
            return "ok";
          default:
            return null;
        }
      },
    },
    event: {
      listen: async () => () => {},
    },
  };
})();`;
}

async function serveDirectory(rootDir) {
  const server = createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url || "/", "http://127.0.0.1");
      let relativePath = decodeURIComponent(requestUrl.pathname);
      if (relativePath === "/") relativePath = "/index.html";
      const filePath = path.join(rootDir, relativePath);
      if (!filePath.startsWith(rootDir)) {
        response.writeHead(403);
        response.end("forbidden");
        return;
      }
      const resolvedPath = existsSync(filePath) ? filePath : `${filePath}/index.html`;
      const data = await fs.readFile(resolvedPath);
      response.writeHead(200, {
        "content-type": MIME_TYPES[path.extname(resolvedPath)] || "application/octet-stream",
      });
      response.end(data);
    } catch (_error) {
      response.writeHead(404);
      response.end("not found");
    }
  });

  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  assert.ok(address && typeof address === "object" && "port" in address);
  return {
    url: `http://127.0.0.1:${address.port}/`,
    async close() {
      await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
    },
  };
}

function overlaps(left, right) {
  const x = Math.min(left.right, right.right) - Math.max(left.left, right.left);
  const y = Math.min(left.bottom, right.bottom) - Math.max(left.top, right.top);
  return x > 1 && y > 1;
}

async function assertActionInvokes(page, action, command, guardedAction) {
  const before = await page.evaluate(
    ({ expectedCommand, expectedAction }) =>
      (window.__mockInvocations || []).filter(
        (entry) =>
          entry.command === expectedCommand &&
          (!expectedAction || entry.payload?.payload?.action === expectedAction),
      ).length,
    { expectedCommand: command, expectedAction: guardedAction },
  );
  const button = page.locator(`button[data-action="${action}"]:visible`).first();
  await button.click();
  await page.waitForFunction(
    ({ expectedCommand, expectedAction, count }) =>
      (window.__mockInvocations || []).filter(
        (entry) =>
          entry.command === expectedCommand &&
          (!expectedAction || entry.payload?.payload?.action === expectedAction),
      ).length > count,
    { expectedCommand: command, expectedAction: guardedAction, count: before },
  );
}

async function assertLanguageChange(page, language) {
  const before = await page.evaluate(
    () =>
      (window.__mockInvocations || []).filter(
        (entry) => entry.command === "set_global_language_preference",
      ).length,
  );
  await page.locator("#shell-language-select").selectOption(language);
  await page.waitForFunction(
    ({ expectedLanguage, count }) =>
      (window.__mockInvocations || []).filter(
        (entry) =>
          entry.command === "set_global_language_preference" &&
          entry.payload?.payload?.language === expectedLanguage,
      ).length > count,
    { expectedLanguage: language, count: before },
    { timeout: 5_000 },
  );
  assert.equal(await page.locator("#shell-language-select").inputValue(), language);
}

async function rectsFor(page, selectors) {
  return page.evaluate((passedSelectors) => {
    return passedSelectors.map((selector) => {
      const element = document.querySelector(selector);
      if (!element) return { selector, exists: false };
      const rect = element.getBoundingClientRect();
      return {
        selector,
        exists: true,
        width: rect.width,
        height: rect.height,
        top: rect.top,
        left: rect.left,
        right: rect.right,
        bottom: rect.bottom,
      };
    });
  }, selectors);
}

export async function createWorkbenchRegressionEnvironment() {
  const preview = copyWorkbenchPreview();
  writeFileSync(path.join(preview.destination, "mock-tauri.js"), workbenchMockSource());
  const server = await serveDirectory(preview.destination);
  return {
    workbenchUrl: server.url,
    async cleanup() {
      await server.close();
      rmSync(preview.tempRoot, { recursive: true, force: true });
    },
  };
}

export async function assertWorkbenchShellRegression(page, viewport) {
  await page.setViewportSize(viewport);
  await page.goto(page.url(), { waitUntil: "networkidle", timeout: 60_000 });
  await page.waitForFunction(() => {
    const deploymentMode = document.getElementById("deployment-mode")?.textContent?.trim();
    const workbenchUrl = document.getElementById("workbench-url")?.textContent?.trim();
    return Boolean(deploymentMode && deploymentMode !== "--" && workbenchUrl && workbenchUrl !== "--");
  });

  assert.equal(await page.locator("#deployment-mode").textContent(), "direct_mesh_gui");
  assert.match(await page.locator("#status-output").textContent(), /runtimes:\s*\d+/);
  assert.match(await page.locator("#viewer-caption").textContent(), /^data:text\/html/);
  await assertLanguageChange(page, "zh");

  const panelRects = await rectsFor(page, [
    '[data-shell-pane="control"]:not(.hidden) .panel:nth-of-type(1)',
    '[data-shell-pane="control"]:not(.hidden) .panel:nth-of-type(2)',
    '[data-shell-pane="control"]:not(.hidden) .panel:nth-of-type(3)',
  ]);
  panelRects.forEach((rect) => {
    assert.equal(rect.exists, true, `${rect.selector} should exist`);
    assert.ok(rect.width > 40, `${rect.selector} should have width`);
    assert.ok(rect.height > 40, `${rect.selector} should have height`);
  });
  assert.equal(overlaps(panelRects[0], panelRects[1]), false, "Workbench control panels should not overlap");
  assert.equal(overlaps(panelRects[1], panelRects[2]), false, "Workbench control panels should not overlap");

  await page.locator('[data-console-tab="logs"]').click();
  await page.waitForSelector('#logs-panel:not(.is-hidden)');
  assert.equal(await page.locator("#log-output").textContent(), "frontend log mock");

  await page.locator('[data-log-service="agent-5002"]').click();
  assert.equal(await page.locator("#log-output").textContent(), "agent-5002 log mock");

  await assertActionInvokes(page, "refresh", "service_status");
  await assertActionInvokes(page, "start-local", "guarded_mutation_action", "service_start");
  await assertActionInvokes(page, "restart-local", "guarded_mutation_action", "service_restart");
  await assertActionInvokes(page, "stop", "guarded_mutation_action", "service_stop");

  await page.locator('[data-shell-target="workbench"]').click();
  await page.waitForSelector('[data-shell-pane="workbench"]:not(.hidden) #workbench-frame');
  const frameSrc = await page.locator("#workbench-frame").getAttribute("src");
  assert.match(frameSrc || "", /^data:text\/html/);

  await page.locator(".viewer__back-button").click();
  await page.waitForSelector('[data-shell-pane="control"]:not(.hidden) .control-grid');

  const commands = await page.evaluate(() =>
    (window.__mockInvocations || []).map((entry) => entry.command),
  );
  for (const command of ["workbench_environment", "service_status", "read_runtime_log"]) {
    assert.ok(commands.includes(command), `expected Tauri invocation: ${command}`);
  }

  const errors = await page.evaluate(() => window.__mockErrors || []);
  assert.deepEqual(errors, []);
}
