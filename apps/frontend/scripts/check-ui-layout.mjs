import { chromium } from "playwright";

const baseUrl = process.env.UI_LAYOUT_URL || "http://127.0.0.1:3000";
const viewports = [
  { name: "desktop", width: 1440, height: 900 },
  { name: "laptop", width: 1280, height: 800 },
  { name: "tablet-landscape", width: 1024, height: 768 },
  { name: "tablet-portrait", width: 768, height: 1024 },
];

function formatIssue(prefix, issue) {
  return `${prefix}: ${issue}`;
}

async function run() {
  const browser = await chromium.launch({ headless: true });
  const failures = [];

  try {
    for (const viewport of viewports) {
      const page = await browser.newPage({ viewport });

      let response = null;
      try {
        response = await page.goto(baseUrl, { waitUntil: "networkidle", timeout: 30_000 });
      } catch (error) {
        failures.push(
          formatIssue(
            viewport.name,
            `unable to open ${baseUrl}. Start the frontend first, for example: npm run dev`,
          ),
        );
        await page.close();
        continue;
      }

      if (!response?.ok()) {
        failures.push(
          formatIssue(
            viewport.name,
            `page responded with HTTP ${response?.status?.() ?? "unknown"}`,
          ),
        );
        await page.close();
        continue;
      }

      const audit = await page.evaluate(() => {
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

        const overflowX = Math.max(
          document.documentElement.scrollWidth - document.documentElement.clientWidth,
          0,
        );

        const containerSelectors = [
          ".button-row",
          ".card-head",
          ".viewport-window-bar",
          ".sidebar-list__row",
        ];

        const overlapIssues = [];

        for (const selector of containerSelectors) {
          const containers = Array.from(document.querySelectorAll(selector)).filter(isVisible);
          containers.forEach((container, containerIndex) => {
            const children = Array.from(container.children).filter(isVisible);
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

        return { overflowX, overlapIssues, runtimeErrorDetected };
      });

      if (audit.runtimeErrorDetected) {
        failures.push(formatIssue(viewport.name, "runtime error screen detected"));
      }

      if (audit.overflowX > 1) {
        failures.push(
          formatIssue(viewport.name, `horizontal overflow detected: ${audit.overflowX}px`),
        );
      }

      audit.overlapIssues.forEach((issue) => {
        failures.push(formatIssue(viewport.name, issue));
      });

      await page.close();
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
  console.error(error);
  process.exit(1);
});
