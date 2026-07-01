import assert from "node:assert/strict";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import {
  cpSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
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
  ".mjs": "text/javascript; charset=utf-8",
  ".png": "image/png",
  ".svg": "image/svg+xml; charset=utf-8",
};

const HUB_REGRESSION_GATE = {
  schema_version: "kyuubiki.regression-gate-report/v1",
  generated_at_unix_s: 1782022702,
  catalog_path: "tmp/regression-lane-catalog.json",
  overall_gate_status: "warn",
  failing_lane_count: 0,
  warning_lane_count: 1,
  lanes: [
    {
      id: "workflow-mesh",
      title: "Workflow mesh",
      category: "workflow-regression",
      status: "pass",
      gate_status: "pass",
      gate_reasons: [],
      generated_at_unix_s: 1782021499,
      links: ["workflow-mesh-regression/index.json"],
    },
    {
      id: "direct-mesh-docker",
      title: "Direct-mesh Docker",
      category: "benchmark",
      status: "pass",
      gate_status: "pass",
      gate_reasons: [],
      generated_at_unix_s: 1782009037,
      links: ["direct-mesh-benchmark-container/latest/summary.json"],
    },
    {
      id: "workflow-catalog",
      title: "Workflow catalog",
      category: "workflow-benchmark",
      status: "pass",
      gate_status: "warn",
      gate_reasons: [
        "workflow.heat-thermo-quad-benchmark-json median regression 308% exceeded warn threshold 20%",
        "workflow.heat-thermo-quad-benchmark-json average regression 126.667% exceeded warn threshold 30%",
      ],
      generated_at_unix_s: 1781942450,
      links: ["workflow-catalog-benchmark/latest/compare.json"],
    },
  ],
  rendered: "overall gate: warn | failing lanes: 0 | warning lanes: 1",
};

const INSTALLER_REGRESSION_GATE = {
  schema_version: "kyuubiki.regression-gate-report/v1",
  generated_at_unix_s: 1782022702,
  catalog_path: "tmp/regression-lane-catalog.json",
  overall_gate_status: "warn",
  failing_lane_count: 0,
  warning_lane_count: 1,
  lanes: HUB_REGRESSION_GATE.lanes,
  rendered: "overall gate: warn | failing lanes: 0 | warning lanes: 1",
};

function injectMockScript(indexHtml) {
  return indexHtml.replace(
    '<script type="module" src="./app.js"></script>',
    '<script src="./mock-tauri.js"></script>\n    <script type="module" src="./app.js"></script>',
  );
}

function ensureCopiedPreview(appDirName) {
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), "kyuubiki-desktop-regression-"));
  const destination = path.join(tempRoot, appDirName);
  const source = path.join(ROOT, "apps", appDirName, "ui");
  cpSync(source, destination, { recursive: true });
  cpSync(path.join(ROOT, "apps", "desktop-shared", "ui"), path.join(destination, "desktop-shared", "ui"), {
    recursive: true,
  });
  return { tempRoot, destination };
}

function writePreviewMock(destination, mockSource) {
  const indexPath = path.join(destination, "index.html");
  writeFileSync(indexPath, injectMockScript(readFileSync(indexPath, "utf8")));
  writeFileSync(path.join(destination, "mock-tauri.js"), mockSource);
}

function hubMockSource() {
  return `(() => {
  window.__mockErrors = [];
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

  const regressionGateReport = ${JSON.stringify(HUB_REGRESSION_GATE, null, 2)};
  const directMeshSnapshot = {
    baseline_path: "tests/integration/benchmarks/direct-mesh-docker-baseline.json",
    output_root: "tmp/direct-mesh-benchmark-container/latest",
    baseline_mean_elapsed_s: 53.457,
    baseline_mean_rss_kib: 84725,
    repeat: 3,
    docker_run_network: "host",
    latest_exists: true,
    latest_generated_at: "2026-06-21T05:57:48Z",
    latest_mean_elapsed_s: 54.771,
    latest_mean_rss_kib: 85102,
    elapsed_delta_pct: 2.458,
    rss_delta_pct: 0.445,
    status: "within_baseline",
  };

  const originalFetch = window.fetch.bind(window);
  window.fetch = async (input, init) => {
    const url = new URL(typeof input === "string" ? input : input.url, window.location.href);
    if (url.origin === "http://127.0.0.1:4000" || url.origin === "http://localhost:4000") {
      if (url.pathname === "/api/health") {
        return new Response(JSON.stringify({ status: "ok" }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url.pathname === "/api/v1/workloads/catalog") {
        return new Response(JSON.stringify({ entries: [] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url.pathname === "/api/v1/workflows/catalog") {
        return new Response(JSON.stringify({ workflows: [] }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url.pathname.startsWith("/api/v1/jobs/")) {
        return new Response(JSON.stringify({ status: "completed", rendered: "workflow job completed" }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url.pathname === "/api/v1/security-events") {
        return new Response(JSON.stringify({ ok: true }), {
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
        switch (command) {
          case "get_global_language_preference":
            return { language: "en" };
          case "set_global_language_preference":
            return { language: payload?.payload?.language || "en" };
          case "hub_environment":
            return {
              deployment_mode: "orchestrated_gui",
              host_platform: "macos",
              installer_gui_hint: "apps/installer-gui",
              workbench_gui_hint: "apps/workbench-gui",
              workbench_url: "http://127.0.0.1:3000",
              orchestrator_url: "http://127.0.0.1:4000",
            };
          case "hub_direct_mesh_regression_snapshot":
            return directMeshSnapshot;
          case "hub_regression_gate_report":
            return regressionGateReport;
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

function installerMockSource() {
  return `(() => {
  window.__mockErrors = [];
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

  const regressionGateReport = ${JSON.stringify(INSTALLER_REGRESSION_GATE, null, 2)};
  const doctorReport = {
    platform: "macos",
    workspace: ".",
    checks: [
      { label: "Rust toolchain", ok: true },
      { label: "Node runtime", ok: true },
      { label: "Release layout", ok: true },
    ],
    rendered: "doctor ok",
  };

  window.__TAURI__ = {
    core: {
      invoke: async (command, payload) => {
        switch (command) {
          case "get_global_language_preference":
            return { language: "en" };
          case "set_global_language_preference":
            return { language: payload?.payload?.language || "en" };
          case "doctor_report":
            return doctorReport;
          case "installation_integrity_report":
            return {
              schema_version: "kyuubiki.installation-integrity/v1",
              platform: "macos",
              workspace: ".",
              current_version: "1.13.0",
              contract_rules: [],
              layout: [],
              version_checks: [],
              residues: [],
              issues: [],
              rendered: "integrity clear",
            };
          case "unified_update_plan":
            return {
              schema_version: "kyuubiki.unified-update-plan/v1",
              workspace: ".",
              current_version: "1.13.0",
              target_channel: "stable",
              target_tag: "tamamono",
              target_version: "1.13.0",
              update_state: "unknown",
              summary: "No update pending.",
              contract_rules: [],
              artifacts: [],
              rendered: "update plan ready",
            };
          case "update_source_config":
            return {
              schema_version: "kyuubiki.update-source-config/v1",
              catalog_path: "tmp/unified-update/catalog.json",
              artifact_root: "tmp/unified-update/artifacts",
              download_dir: "tmp/unified-update/downloads",
              rendered: "local update source",
            };
          case "unified_update_preview":
            return {
              schema_version: "kyuubiki.unified-update-preview/v1",
              channel: "stable",
              target_version: "1.13.0",
              overall_status: "ready_for_apply",
              blocking_issues: 0,
              removable_residue: 0,
              steps: [],
              rendered: "preview ready",
            };
          case "latest_downloaded_update_record":
          case "latest_applied_update_record":
          case "latest_staged_update_record":
          case "read_env_file":
          case "remote_deploy_policy":
          case "remote_node_registry":
            return null;
          case "service_status":
            return {
              rendered: "frontend healthy | orchestrator healthy | agents healthy",
              summary: {
                overall_status: "healthy",
                entries: [],
              },
            };
          case "regression_gate_report":
            return regressionGateReport;
          case "read_runtime_log":
            return {
              service: payload?.service || "orchestrator",
              rendered: "runtime log mock",
            };
          case "start_log_stream":
          case "stop_log_stream":
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

async function visibleRects(page, selectors) {
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

function overlaps(left, right) {
  const x = Math.min(left.right, right.right) - Math.max(left.left, right.left);
  const y = Math.min(left.bottom, right.bottom) - Math.max(left.top, right.top);
  return x > 1 && y > 1;
}

export async function createDesktopShellRegressionEnvironment() {
  const hubPreview = ensureCopiedPreview("hub-gui");
  const installerPreview = ensureCopiedPreview("installer-gui");

  writePreviewMock(hubPreview.destination, hubMockSource());
  writePreviewMock(installerPreview.destination, installerMockSource());

  const hubServer = await serveDirectory(hubPreview.destination);
  const installerServer = await serveDirectory(installerPreview.destination);

  return {
    hubUrl: hubServer.url,
    installerUrl: installerServer.url,
    async cleanup() {
      await Promise.all([hubServer.close(), installerServer.close()]);
      rmSync(hubPreview.tempRoot, { recursive: true, force: true });
      rmSync(installerPreview.tempRoot, { recursive: true, force: true });
    },
  };
}

export async function assertNoPageErrors(page) {
  const errors = await page.evaluate(() => window.__mockErrors || []);
  assert.deepEqual(errors, []);
}

export async function assertHubRegression(page, viewport) {
  await page.setViewportSize(viewport);
  await page.goto(page.url(), { waitUntil: "networkidle", timeout: 60_000 });
  await page.locator("#projects-tab-guides").click();
  await page.waitForSelector('[data-projects-pane="guides"]:not(.hidden) #guides-gate-status-value');

  assert.equal(await page.locator("#guides-gate-status-value").textContent(), "warn");
  assert.equal(await page.locator("#guides-gate-warning-count").textContent(), "1");
  assert.equal(await page.locator("#guides-gate-failing-count").textContent(), "0");
  assert.equal(await page.locator("#guides-gate-lane-count").textContent(), "3");

  const reasons = await page.locator("#guides-gate-reasons").textContent();
  assert.match(reasons, /Workflow catalog:/);
  assert.match(reasons, /median regression 308%/);

  const rects = await visibleRects(page, [
    '[data-projects-pane="guides"]:not(.hidden) .hub-card:nth-of-type(1)',
    '[data-projects-pane="guides"]:not(.hidden) .hub-card:nth-of-type(2)',
  ]);
  rects.forEach((rect) => {
    assert.equal(rect.exists, true, `${rect.selector} should exist`);
    assert.ok(rect.width > 40, `${rect.selector} should have width`);
    assert.ok(rect.height > 40, `${rect.selector} should have height`);
  });
  assert.equal(overlaps(rects[0], rects[1]), false, "Hub guides cards should not overlap");

  await assertNoPageErrors(page);
}

export async function assertInstallerRegression(page, viewport) {
  await page.setViewportSize(viewport);
  await page.goto(page.url(), { waitUntil: "networkidle", timeout: 60_000 });

  await page.waitForSelector("#completion-banner:not([hidden])");
  await page.waitForFunction(() =>
    /Unified regression gate is warn/.test(
      document.querySelector("#completion-message")?.textContent || "",
    ),
  );
  assert.match(
    await page.locator("#completion-message").textContent(),
    /Unified regression gate is warn/,
  );
  assert.equal(await page.locator("#doctor-grid > *").count(), 3);
  assert.equal(await page.locator("#regression-gate-status").textContent(), "warn");
  assert.match(await page.locator("#regression-gate-reasons").textContent(), /Workflow catalog:/);

  await page.locator('button.sidebar-tab[data-tab="integrity"]').click();
  await page.waitForSelector('[data-panel="integrity"].panel-visible #integrity-headline');
  assert.match(
    await page.locator("#integrity-headline").textContent(),
    /Standard install contract is healthy/,
  );

  await page.locator('button.sidebar-tab[data-tab="updates"]').click();
  await page.waitForSelector('[data-panel="updates"].panel-visible #update-state-headline');
  assert.match(await page.locator("#update-state-headline").textContent(), /Update state is unknown/);
  assert.equal(await page.locator("#update-source-output").textContent(), "local update source");

  const rects = await visibleRects(page, [
    '[data-panel="updates"].panel-visible .update-summary-card:nth-of-type(1)',
    '[data-panel="updates"].panel-visible .update-summary-card:nth-of-type(2)',
    '[data-panel="updates"].panel-visible .update-summary-card:nth-of-type(3)',
  ]);
  rects.forEach((rect) => {
    assert.equal(rect.exists, true, `${rect.selector} should exist`);
    assert.ok(rect.width > 40, `${rect.selector} should have width`);
    assert.ok(rect.height > 40, `${rect.selector} should have height`);
  });
  assert.equal(overlaps(rects[0], rects[1]), false, "Installer update cards should not overlap");
  assert.equal(overlaps(rects[1], rects[2]), false, "Installer update cards should not overlap");

  await page.locator('button.sidebar-tab[data-tab="services"]').click();
  await page.waitForSelector('[data-panel="services"].panel-visible #runtime-log');
  assert.equal(await page.locator("#runtime-log").textContent(), "runtime log mock");

  await assertNoPageErrors(page);
}
