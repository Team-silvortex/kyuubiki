import { chromium } from "playwright";
import { isRestrictedPlaywrightLaunchError, reportRestrictedPlaywrightSkip } from "./playwright-runtime-guard.mjs";

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
];

function formatIssue(prefix, issue) {
  return `${prefix}: ${issue}`;
}

function resolveAuditUrl(pathname) {
  if (/^https?:\/\//.test(pathname)) return pathname;
  return new URL(pathname, baseUrl).toString();
}

async function run() {
  let browser;
  try {
    browser = await chromium.launch({ headless: true });
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
        const auditUrl = resolveAuditUrl(pagePath);

        let response = null;
        try {
          response = await page.goto(auditUrl, { waitUntil: "networkidle", timeout: 30_000 });
        } catch (error) {
          failures.push(
            formatIssue(
              `${viewport.name} ${pagePath}`,
              `unable to open ${auditUrl}. Start the frontend first, for example: npm run dev`,
            ),
          );
          await page.close();
          continue;
        }

        if (!response?.ok()) {
          failures.push(
            formatIssue(
              `${viewport.name} ${pagePath}`,
              `page responded with HTTP ${response?.status?.() ?? "unknown"}`,
            ),
          );
          await page.close();
          continue;
        }

        const audit = await page.evaluate((selectors) => {
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
                      `${selector}[${containerIndex}] child ${leftIndex} overlaps child ${rightIndex}`,
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

          return { overflowX, overlapIssues, clipIssues, runtimeErrorDetected };
        }, containerSelectors);

        if (audit.runtimeErrorDetected) {
          failures.push(formatIssue(`${viewport.name} ${pagePath}`, "runtime error screen detected"));
        }

        if (audit.overflowX > 1) {
          failures.push(
            formatIssue(
              `${viewport.name} ${pagePath}`,
              `horizontal overflow detected: ${audit.overflowX}px`,
            ),
          );
        }

        audit.overlapIssues.forEach((issue) => {
          failures.push(formatIssue(`${viewport.name} ${pagePath}`, issue));
        });
        audit.clipIssues.forEach((issue) => {
          failures.push(formatIssue(`${viewport.name} ${pagePath}`, issue));
        });

        await page.close();
      }
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
