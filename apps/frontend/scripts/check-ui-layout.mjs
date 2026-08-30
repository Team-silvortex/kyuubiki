import { chromium } from "playwright";
import { isRestrictedPlaywrightLaunchError, launchPlaywrightChromium, reportRestrictedPlaywrightSkip } from "./playwright-runtime-guard.mjs";

const baseUrl = process.env.UI_LAYOUT_URL || "http://127.0.0.1:3000";
const pagePaths = (process.env.UI_LAYOUT_PATHS || "/,/workflow-benchmark,/docs,/docs/workflow-architecture")
  .split(",")
  .map((entry) => entry.trim())
  .filter(Boolean);
const viewports = [
  { name: "desktop", width: 1440, height: 900 },
  { name: "laptop", width: 1280, height: 800 },
  { name: "tablet-landscape", width: 1024, height: 768 },
  { name: "tablet-portrait", width: 768, height: 1024 },
  { name: "phone-large", width: 430, height: 932 },
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

function resolveAuditUrl(pathname) {
  if (/^https?:\/\//.test(pathname)) return pathname;
  return new URL(pathname, baseUrl).toString();
}

async function waitForDoublePaint(page) {
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function waitForResponsiveWorkbench(page) {
  await page.waitForFunction(() => {
    const shell = document.querySelector('[data-workbench-shell="root"]');
    if (!shell) return true;
    const expected = window.innerWidth <= 680
      ? "phone"
      : window.innerWidth <= 980
        ? "tablet"
        : window.innerWidth <= 1440
          ? "compact"
          : "desktop";
    return shell.getAttribute("data-workbench-viewport-profile") === expected;
  }, undefined, { timeout: 5_000 });
}

async function auditPageState(page) {
  return page.evaluate((selectors) => {
    function isVisible(element) {
      const style = window.getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return (
        style.display !== "none" &&
        style.visibility !== "hidden" &&
        rect.width > 0 &&
        rect.height > 0
      );
    }

    function overlaps(a, b) {
      return !(
        a.right <= b.left ||
        b.right <= a.left ||
        a.bottom <= b.top ||
        b.bottom <= a.top
      );
    }

    function describeElement(element) {
      const marker = Array.from(element.attributes)
        .find((attribute) => attribute.name.startsWith("data-"));
      const identity = marker
        ? `${marker.name}=${marker.value}`
        : element.className || element.tagName.toLowerCase();
      const text = (element.textContent || "").trim().replace(/\s+/g, " ").slice(0, 36);
      return text ? `${identity} \"${text}\"` : identity;
    }

    function describeRect(element) {
      const rect = element.getBoundingClientRect();
      return `[${rect.left.toFixed(1)},${rect.top.toFixed(1)} ${rect.width.toFixed(1)}x${rect.height.toFixed(1)}]`;
    }

    function describeParentPath(element) {
      const path = [];
      let current = element.parentElement;
      for (let depth = 0; current && depth < 6; depth += 1, current = current.parentElement) {
        path.push(`${current.className || current.tagName.toLowerCase()} ${describeRect(current)}`);
      }
      return path.join(" <- ");
    }

    function axisOverflowValue(styleValue) {
      return styleValue === "auto" || styleValue === "scroll" || styleValue === "hidden" || styleValue === "clip";
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

    const overflowX = Math.max(
      document.documentElement.scrollWidth - document.documentElement.clientWidth,
      0,
    );
    const overlapIssues = [];
    const clipIssues = [];
    const layoutIssues = [];
    const stackedShell = document.querySelector('[data-workbench-stack-panels="true"]');
    const stackedWorkspace = stackedShell?.querySelector(":scope > .workbench-shell__workspace");
    if (stackedShell && stackedWorkspace) {
      const shellWidth = stackedShell.getBoundingClientRect().width;
      const workspaceWidth = stackedWorkspace.getBoundingClientRect().width;
      if (workspaceWidth < shellWidth * 0.8) {
        const shellStyle = getComputedStyle(stackedShell);
        const workspaceStyle = getComputedStyle(stackedWorkspace);
        const childLayout = Array.from(stackedShell.children)
          .map((child) => `${describeElement(child)} ${describeRect(child)}`)
          .join(" | ");
        layoutIssues.push(
          `stacked workspace collapsed to ${workspaceWidth.toFixed(1)}px inside ${shellWidth.toFixed(1)}px shell; columns=${shellStyle.gridTemplateColumns}; areas=${shellStyle.gridTemplateAreas}; workspace-display=${workspaceStyle.display}; workspace-position=${workspaceStyle.position}; children=${childLayout}`,
        );
      }
    }

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
            clipIssues.push(
              `${selector}[${containerIndex}] child ${childIndex} clipped by container`,
            );
          }
        });

        for (let leftIndex = 0; leftIndex < children.length; leftIndex += 1) {
          for (let rightIndex = leftIndex + 1; rightIndex < children.length; rightIndex += 1) {
            const left = children[leftIndex];
            const right = children[rightIndex];
            if (overlaps(left.getBoundingClientRect(), right.getBoundingClientRect())) {
              overlapIssues.push(
                `${selector}[${containerIndex}] (${describeElement(container)} ${describeRect(container)} <- ${describeParentPath(container)}) child ${leftIndex} (${describeElement(left)} ${describeRect(left)}) overlaps child ${rightIndex} (${describeElement(right)} ${describeRect(right)})`,
              );
            }
          }
        }
      });
    }

    const rootText = document.body?.innerText ?? "";
    const runtimeErrorDetected =
      rootText.includes("TypeError:") ||
      rootText.includes("ReferenceError:") ||
      rootText.includes("Application error") ||
      rootText.includes("Internal Server Error");

    return { overflowX, overlapIssues, clipIssues, layoutIssues, runtimeErrorDetected };
  }, containerSelectors);
}

function pushAuditFailures(failures, audit, prefix) {
  if (audit.runtimeErrorDetected) {
    failures.push(formatIssue(prefix, "runtime error screen detected"));
  }
  if (audit.overflowX > 1) {
    failures.push(formatIssue(prefix, `horizontal overflow detected: ${audit.overflowX}px`));
  }
  audit.overlapIssues.forEach((issue) => failures.push(formatIssue(prefix, issue)));
  audit.clipIssues.forEach((issue) => failures.push(formatIssue(prefix, issue)));
  audit.layoutIssues.forEach((issue) => failures.push(formatIssue(prefix, issue)));
}

async function openAuditedPage(page, pagePath, prefix, failures) {
  const auditUrl = resolveAuditUrl(pagePath);
  try {
    const response = await page.goto(auditUrl, { waitUntil: "domcontentloaded", timeout: 30_000 });
    if (!response?.ok()) {
      failures.push(formatIssue(prefix, `page responded with HTTP ${response?.status?.() ?? "unknown"}`));
      return false;
    }
    await waitForDoublePaint(page);
    await waitForResponsiveWorkbench(page);
    return true;
  } catch (error) {
    failures.push(
      formatIssue(prefix, `unable to open ${auditUrl}. Start the frontend first, for example: npm run dev`),
    );
    return false;
  }
}

async function runWorkflowBenchmarkInteractiveAudit(browser, viewport, failures) {
  const page = await browser.newPage({ viewport });
  try {
    const prefixBase = `${viewport.name} /workflow-benchmark`;
    if (!(await openAuditedPage(page, "/workflow-benchmark", prefixBase, failures))) return;
    await page.waitForFunction(() => Boolean(window.__kyuubikiWorkflowDebug), { timeout: 30_000 });

    await page.evaluate(() => window.__kyuubikiWorkflowDebug?.setSurfaceTab("catalog"));
    await waitForDoublePaint(page);
    const catalogSearch = page.locator('[data-workflow-catalog-search="query"]');
    await catalogSearch.fill("bridge thermal export");
    await catalogSearch.blur();
    await waitForDoublePaint(page);
    pushAuditFailures(failures, await auditPageState(page), `${prefixBase} [catalog-search]`);

    await page.evaluate(() => window.__kyuubikiWorkflowDebug?.setSurfaceTab("builder"));
    await waitForDoublePaint(page);
    await page.locator('[data-workflow-topology-view-target="add"]').click();
    await waitForDoublePaint(page);
    const operatorSearch = page.locator('[data-workflow-operator-search="query"]');
    await operatorSearch.fill("thermal bridge");
    await operatorSearch.blur();
    await waitForDoublePaint(page);
    pushAuditFailures(failures, await auditPageState(page), `${prefixBase} [builder-search]`);
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
      reportRestrictedPlaywrightSkip("UI layout guard", error);
      return;
    }
    throw error;
  }
  const failures = [];

  try {
    for (const viewport of viewports) {
      for (const pagePath of pagePaths) {
        const page = await browser.newPage({ viewport });
        const prefix = `${viewport.name} ${pagePath}`;
        try {
          if (!(await openAuditedPage(page, pagePath, prefix, failures))) continue;
          pushAuditFailures(failures, await auditPageState(page), prefix);
        } finally {
          await page.close();
        }
      }
      await runWorkflowBenchmarkInteractiveAudit(browser, viewport, failures);
    }
  } finally {
    await browser.close();
  }

  if (failures.length > 0) {
    console.error("UI layout guard failed.");
    failures.forEach((failure) => console.error(`- ${failure}`));
    process.exit(1);
  }

  console.log("UI layout guard passed.");
}

run().catch((error) => {
  if (isRestrictedPlaywrightLaunchError(error)) {
    reportRestrictedPlaywrightSkip("UI layout guard", error);
    process.exit(0);
  }
  console.error(error);
  process.exit(1);
});
