import { chromium } from "playwright";
import { isRestrictedPlaywrightLaunchError, launchPlaywrightChromium, reportRestrictedPlaywrightSkip } from "./playwright-runtime-guard.mjs";

const baseUrl = process.env.WORKFLOW_LAYOUT_URL || "http://127.0.0.1:3000/workflow-benchmark";
const viewports = [
  { name: "desktop", width: 1280, height: 800 },
  { name: "phone", width: 390, height: 844 },
];
const containerSelectors = [
  ".button-row",
  ".card-head",
  ".viewport-window-bar",
  ".sidebar-list__row",
  ".panel-tabs",
  ".form-grid",
  ".runtime-overview-grid",
];

function formatIssue(prefix, issue) {
  return `${prefix}: ${issue}`;
}

async function waitForDoublePaint(page) {
  await page.evaluate(
    () =>
      new Promise((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(resolve));
      }),
  );
}

async function openBenchmarkPage(page, viewportName, failures) {
  try {
    const response = await page.goto(baseUrl, { waitUntil: "domcontentloaded", timeout: 30_000 });
    if (!response?.ok()) {
      failures.push(
        formatIssue(viewportName, `page responded with HTTP ${response?.status?.() ?? "unknown"}`),
      );
      return false;
    }
    await page.waitForFunction(() => Boolean(window.__kyuubikiWorkflowDebug), { timeout: 10_000 });
    await waitForDoublePaint(page);
    return true;
  } catch (error) {
    failures.push(
      formatIssue(
        viewportName,
        `unable to open ${baseUrl}. Start the frontend first, for example: npm run dev`,
      ),
    );
    return false;
  }
}

async function clickSurfaceTab(page, surface, viewportName, failures) {
  const tab = page.locator(`[data-workflow-surface-tab="${surface}"]`);
  const count = await tab.count();
  if (count !== 1) {
    failures.push(formatIssue(viewportName, `expected exactly one surface action "${surface}", received ${count}`));
    return false;
  }
  await tab.click();
  await page.locator(`[data-workbench-workflow-surface="${surface}"]`).waitFor({ state: "visible" });
  await waitForDoublePaint(page);
  return true;
}

async function openBuilderView(page, attribute, view) {
  const target = page.locator(`[${attribute}-target="${view}"]`);
  await target.waitFor({ state: "visible", timeout: 10_000 });
  if ((await target.count()) !== 1) {
    throw new Error(`expected exactly one ${attribute} action "${view}"`);
  }
  await target.click();
  await page.locator(`[${attribute}="${view}"]`).waitFor({ state: "visible", timeout: 10_000 });
  await waitForDoublePaint(page);
}

async function fillCatalogSearch(page, query, viewportName, failures) {
  const search = page
    .locator('[data-workbench-workflow-surface="catalog"]')
    .locator('[data-workflow-catalog-search="query"]');
  try {
    await search.first().waitFor({ state: "visible", timeout: 10_000 });
  } catch {
    failures.push(formatIssue(viewportName, "catalog search did not mount in time"));
    return false;
  }
  const count = await search.count();
  if (count !== 1) {
    failures.push(formatIssue(viewportName, `expected one catalog search input, received ${count}`));
    return false;
  }
  await search.fill(query);
  await search.blur();
  await waitForDoublePaint(page);
  return true;
}

async function fillBuilderSearch(page, query, viewportName, failures) {
  const search = page
    .locator('[data-workbench-workflow-surface="builder"]')
    .locator(".form-grid.compact")
    .locator('[data-workflow-operator-search="query"]');
  try {
    await search.first().waitFor({ state: "visible", timeout: 10_000 });
  } catch {
    failures.push(formatIssue(viewportName, "builder search did not mount in time"));
    return false;
  }
  const count = await search.count();
  if (count !== 1) {
    failures.push(formatIssue(viewportName, `expected one builder search input, received ${count}`));
    return false;
  }
  await search.click();
  await search.fill(query);
  await search.blur();
  await waitForDoublePaint(page);
  return true;
}

async function fillTemplateChainSearch(page, query, viewportName, failures) {
  const search = page
    .locator('[data-workbench-workflow-surface="builder"]')
    .locator('[data-workflow-template-chain-search="query"]');
  try {
    await search.first().waitFor({ state: "visible", timeout: 10_000 });
  } catch {
    failures.push(formatIssue(viewportName, "template-chain search did not mount in time"));
    return false;
  }
  const count = await search.count();
  if (count !== 1) {
    failures.push(formatIssue(viewportName, `expected one template-chain search input, received ${count}`));
    return false;
  }
  await search.scrollIntoViewIfNeeded();
  await search.click();
  await search.fill(query);
  await search.blur();
  await waitForDoublePaint(page);
  return true;
}

async function focusScopedContainer(page, selector, viewportName, failures) {
  const container = page.locator(selector);
  try {
    await container.first().waitFor({ state: "attached", timeout: 10_000 });
  } catch {
    failures.push(formatIssue(viewportName, `scoped container ${selector} did not mount in time`));
    return false;
  }
  const count = await container.count();
  if (count !== 1) {
    failures.push(formatIssue(viewportName, `expected one scoped container ${selector}, received ${count}`));
    return false;
  }
  await container.scrollIntoViewIfNeeded();
  await waitForDoublePaint(page);
  return true;
}

async function waitForDeferredBuilderPanels(page, viewportName, failures) {
  const summary = page.locator('[data-workflow-dataset-editor="summary"]');
  try {
    await summary.waitFor({ state: "attached", timeout: 10_000 });
    await waitForDoublePaint(page);
    return true;
  } catch {
    failures.push(formatIssue(viewportName, "deferred builder panels did not mount in time"));
    return false;
  }
}

async function injectLongImportMessage(page, viewportName, failures) {
  const toolbar = page.locator('[data-workflow-builder-toolbar="actions"]');
  try {
    await toolbar.first().waitFor({ state: "attached", timeout: 10_000 });
  } catch {
    failures.push(formatIssue(viewportName, "builder toolbar did not mount in time"));
    return false;
  }
  if ((await toolbar.count()) !== 1) {
    failures.push(formatIssue(viewportName, "expected one builder toolbar container"));
    return false;
  }
  await page.evaluate(() => {
    let target = document.querySelector('[data-workflow-import-message="text"]');
    if (!target) {
      const toolbar = document.querySelector('[data-workflow-builder-toolbar="actions"]');
      if (!toolbar?.parentElement) return;
      const message = document.createElement("p");
      message.className = "card-copy";
      message.setAttribute("data-workflow-import-message", "text");
      toolbar.parentElement.insertBefore(message, toolbar.nextSibling);
      target = message;
    }
    target.textContent =
      "Import validation failed: dataset contract and workflow graph disagree on result_summary axis semantics, bridge normalization defaults, export payload shape, retained snapshot lineage, and mounted package policy ownership. Review node wiring, dataset ids, package diagnostics, and local repair history before promoting this draft.";
  });
  await waitForDoublePaint(page);
  return true;
}

async function auditLayout(page) {
  return page.evaluate((selectors) => {
    function isVisible(element) {
      const style = window.getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
    }

    function overlaps(a, b) {
      return !(a.right <= b.left || b.right <= a.left || a.bottom <= b.top || b.bottom <= a.top);
    }

    function axisOverflowValue(styleValue) {
      return (
        styleValue === "auto" ||
        styleValue === "scroll" ||
        styleValue === "hidden" ||
        styleValue === "clip"
      );
    }

    function axisCanScroll(container, axis) {
      const style = window.getComputedStyle(container);
      const overflowValue = axis === "x" ? style.overflowX : style.overflowY;
      const clientSize = axis === "x" ? container.clientWidth : container.clientHeight;
      const scrollSize = axis === "x" ? container.scrollWidth : container.scrollHeight;
      return (overflowValue === "auto" || overflowValue === "scroll") && scrollSize - clientSize > 1;
    }

    function axisClipsWithoutScroll(container, axis) {
      const style = window.getComputedStyle(container);
      const overflowValue = axis === "x" ? style.overflowX : style.overflowY;
      if (!axisOverflowValue(overflowValue)) return false;
      return !axisCanScroll(container, axis);
    }

    const overlapIssues = [];
    const clipIssues = [];

    for (const selector of selectors) {
      const containers = Array.from(document.querySelectorAll(selector)).filter(isVisible);
      containers.forEach((container, containerIndex) => {
        const children = Array.from(container.children).filter(isVisible);
        const containerRect = container.getBoundingClientRect();
        const clipsX = axisClipsWithoutScroll(container, "x");
        const clipsY = axisClipsWithoutScroll(container, "y");

        children.forEach((child, childIndex) => {
          const rect = child.getBoundingClientRect();
          const overflowRight = rect.right - containerRect.right > 1;
          const overflowLeft = containerRect.left - rect.left > 1;
          const overflowBottom = rect.bottom - containerRect.bottom > 1;
          const overflowTop = containerRect.top - rect.top > 1;
          if ((clipsX && (overflowLeft || overflowRight)) || (clipsY && (overflowTop || overflowBottom))) {
            clipIssues.push(`${selector}[${containerIndex}] child ${childIndex} clipped by container`);
          }
        });

        for (let leftIndex = 0; leftIndex < children.length; leftIndex += 1) {
          for (let rightIndex = leftIndex + 1; rightIndex < children.length; rightIndex += 1) {
            if (overlaps(children[leftIndex].getBoundingClientRect(), children[rightIndex].getBoundingClientRect())) {
              overlapIssues.push(
                `${selector}[${containerIndex}] child ${leftIndex} overlaps child ${rightIndex}`,
              );
            }
          }
        }
      });
    }

    const overflowX = Math.max(
      document.documentElement.scrollWidth - document.documentElement.clientWidth,
      0,
    );
    const runtimeErrorDetected = (document.body?.innerText ?? "").includes("TypeError:");
    return { overflowX, overlapIssues, clipIssues, runtimeErrorDetected };
  }, containerSelectors);
}

async function auditScopedLayout(page, selector) {
  return page.evaluate((containerSelector) => {
    function isVisible(element) {
      const style = window.getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.display !== "none" && style.visibility !== "hidden" && rect.width > 0 && rect.height > 0;
    }

    function overlaps(a, b) {
      return !(a.right <= b.left || b.right <= a.left || a.bottom <= b.top || b.bottom <= a.top);
    }

    const container = document.querySelector(containerSelector);
    if (!container || !isVisible(container)) {
      return { missing: true, overlapIssues: [], clipIssues: [], overflowX: 0, runtimeErrorDetected: false };
    }

    const children = Array.from(container.children).filter(isVisible);
    const containerRect = container.getBoundingClientRect();
    const overlapIssues = [];
    const clipIssues = [];

    children.forEach((child, childIndex) => {
      const rect = child.getBoundingClientRect();
      const overflow =
        rect.left < containerRect.left - 1 ||
        rect.right > containerRect.right + 1 ||
        rect.top < containerRect.top - 1 ||
        rect.bottom > containerRect.bottom + 1;
      if (overflow) clipIssues.push(`child ${childIndex} clipped by container`);
    });

    for (let leftIndex = 0; leftIndex < children.length; leftIndex += 1) {
      for (let rightIndex = leftIndex + 1; rightIndex < children.length; rightIndex += 1) {
        if (overlaps(children[leftIndex].getBoundingClientRect(), children[rightIndex].getBoundingClientRect())) {
          overlapIssues.push(`child ${leftIndex} overlaps child ${rightIndex}`);
        }
      }
    }

    return {
      missing: false,
      overlapIssues,
      clipIssues,
      overflowX: Math.max(document.documentElement.scrollWidth - document.documentElement.clientWidth, 0),
      runtimeErrorDetected: false,
    };
  }, selector);
}

function pushAuditFailures(failures, prefix, audit) {
  if (audit.runtimeErrorDetected) {
    failures.push(formatIssue(prefix, "runtime error screen detected"));
  }
  if (audit.overflowX > 1) {
    failures.push(formatIssue(prefix, `horizontal overflow detected: ${audit.overflowX}px`));
  }
  audit.overlapIssues.forEach((issue) => failures.push(formatIssue(prefix, issue)));
  audit.clipIssues.forEach((issue) => failures.push(formatIssue(prefix, issue)));
}

function pushScopedAuditFailures(failures, prefix, selector, audit) {
  if (audit.missing) {
    failures.push(formatIssue(prefix, `missing scoped container ${selector}`));
    return;
  }
  if (audit.overflowX > 1) {
    failures.push(formatIssue(prefix, `horizontal overflow detected: ${audit.overflowX}px`));
  }
  audit.overlapIssues.forEach((issue) => failures.push(formatIssue(prefix, `${selector} ${issue}`)));
  audit.clipIssues.forEach((issue) => failures.push(formatIssue(prefix, `${selector} ${issue}`)));
}

async function auditWorkflowSearchLayout(browser, viewport, failures) {
  const page = await browser.newPage({ viewport });
  try {
    if (!(await openBenchmarkPage(page, viewport.name, failures))) return;

    if (await clickSurfaceTab(page, "catalog", viewport.name, failures)) {
      if (await fillCatalogSearch(page, "bridge thermal export", `${viewport.name} catalog`, failures)) {
        pushAuditFailures(failures, `${viewport.name} catalog`, await auditLayout(page));
      }
    }

    if (await clickSurfaceTab(page, "builder", viewport.name, failures)) {
      await openBuilderView(page, "data-workflow-topology-view", "add");
      if (await fillBuilderSearch(page, "thermal bridge", `${viewport.name} builder`, failures)) {
        pushAuditFailures(failures, `${viewport.name} builder`, await auditLayout(page));
        pushScopedAuditFailures(
          failures,
          `${viewport.name} builder-toolbar`,
          '[data-workflow-builder-toolbar="actions"]',
          await auditScopedLayout(page, '[data-workflow-builder-toolbar="actions"]'),
        );
      }
      await openBuilderView(page, "data-workflow-topology-view", "templates");
      if (
        await fillTemplateChainSearch(
          page,
          "electrostatic thermo",
          `${viewport.name} template-library`,
          failures,
        )
      ) {
        pushAuditFailures(
          failures,
          `${viewport.name} template-library`,
          await auditLayout(page),
        );
      }
      await openBuilderView(page, "data-workflow-builder-stage", "contracts");
      await openBuilderView(page, "data-workflow-contract-view", "dataset");
      if (await waitForDeferredBuilderPanels(page, `${viewport.name} builder-deferred`, failures)) {
        await openBuilderView(page, "data-workflow-dataset-view", "editor");
        if (
          await focusScopedContainer(
            page,
            '[data-workflow-dataset-editor="editor"]',
            `${viewport.name} dataset-editor`,
            failures,
          )
        ) {
          pushScopedAuditFailures(
            failures,
            `${viewport.name} dataset-editor`,
            '[data-workflow-dataset-editor="editor"]',
            await auditScopedLayout(page, '[data-workflow-dataset-editor="editor"]'),
          );
        }
      }
      await openBuilderView(page, "data-workflow-builder-stage", "validation");
      for (const [panel, selector, label] of [
        ["validation", '[data-workflow-validation-card="card"]', "diagnostics-validation"],
        ["package", '[data-workflow-package-policy-card="card"]', "diagnostics-rules"],
        ["integrity", '[data-workflow-integrity-card="card"]', "diagnostics-integrity"],
      ]) {
        await openBuilderView(page, "data-workflow-diagnostics-panel", panel);
        if (
          await focusScopedContainer(
            page,
            selector,
            `${viewport.name} ${label}`,
            failures,
          )
        ) {
          pushScopedAuditFailures(
            failures,
            `${viewport.name} ${label}`,
            selector,
            await auditScopedLayout(page, selector),
          );
        }
      }
      if (await focusScopedContainer(page, '[data-workflow-snapshot-card="card"]', `${viewport.name} diagnostics-snapshots`, failures)) {
        pushScopedAuditFailures(
          failures,
          `${viewport.name} diagnostics-snapshots`,
          '[data-workflow-snapshot-card="card"]',
          await auditScopedLayout(page, '[data-workflow-snapshot-card="card"]'),
        );
      }
      if (
        await injectLongImportMessage(page, `${viewport.name} import-message`, failures)
      ) {
        pushScopedAuditFailures(
          failures,
          `${viewport.name} import-message`,
          '[data-workflow-builder-toolbar="actions"]',
          await auditScopedLayout(page, '[data-workflow-builder-toolbar="actions"]'),
        );
      }
    }

    if (await clickSurfaceTab(page, "runs", viewport.name, failures)) {
      pushAuditFailures(failures, `${viewport.name} runs`, await auditLayout(page));
      pushScopedAuditFailures(
        failures,
        `${viewport.name} runs-filters`,
        '[data-workflow-runs-filter-row="actions"]',
        await auditScopedLayout(page, '[data-workflow-runs-filter-row="actions"]'),
      );
    }
  } finally {
    await page.close();
  }
}

async function run() {
  let browser;
  try {
    browser = await launchPlaywrightChromium(chromium, { headless: true });
  } catch (error) {
    if (isRestrictedPlaywrightLaunchError(error)) {
      reportRestrictedPlaywrightSkip("Workflow search layout guard", error);
      return;
    }
    throw error;
  }

  const failures = [];
  try {
    for (const viewport of viewports) {
      await auditWorkflowSearchLayout(browser, viewport, failures);
    }
  } finally {
    await browser.close();
  }

  if (failures.length > 0) {
    console.error("Workflow search layout guard failed.");
    failures.forEach((failure) => console.error(`- ${failure}`));
    process.exit(1);
  }

  console.log("Workflow search layout guard passed.");
}

run().catch((error) => {
  if (isRestrictedPlaywrightLaunchError(error)) {
    reportRestrictedPlaywrightSkip("Workflow search layout guard", error);
    process.exit(0);
  }
  console.error(error);
  process.exit(1);
});
