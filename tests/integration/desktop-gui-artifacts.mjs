import path from "node:path";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

export const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
export const DESKTOP_GUI_ARTIFACT_DIR = path.join(ROOT, "tmp", "desktop-gui-regression-artifacts");

function slug(value) {
  return String(value || "artifact")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 80) || "artifact";
}

function viewportLabel(viewport) {
  if (!viewport || typeof viewport.width !== "number" || typeof viewport.height !== "number") {
    return "unknown-viewport";
  }
  return `${viewport.width}x${viewport.height}`;
}

function timestampLabel(date = new Date()) {
  return date
    .toISOString()
    .replace(/\.\d{3}Z$/u, "Z")
    .replace(/[:]/g, "-");
}

export async function captureDesktopGuiArtifacts(page, options = {}) {
  const suite = slug(options.suite || "desktop-gui");
  const scenario = slug(options.scenario || "failure");
  const viewport = viewportLabel(options.viewport);
  const capturedAt = new Date();
  const timestamp = timestampLabel(capturedAt);
  const outputDir = path.join(DESKTOP_GUI_ARTIFACT_DIR, suite);
  const prefix = `${scenario}-${viewport}-${timestamp}`;

  await mkdir(outputDir, { recursive: true });

  const state = await page.evaluate(() => {
    const activeText = document.body?.innerText?.slice(0, 4000) || "";
    return {
      location: window.location.href,
      title: document.title,
      readyState: document.readyState,
      mockErrors: window.__mockErrors || [],
      activeElement: document.activeElement?.id || document.activeElement?.tagName || null,
      bodyTextPreview: activeText,
    };
  }).catch((error) => ({
    location: page.url(),
    captureError: String(error?.message || error),
  }));

  const html = await page.content().catch((error) => `<!-- failed to capture page html: ${String(error?.message || error)} -->`);
  const screenshotPath = path.join(outputDir, `${prefix}.png`);
  const htmlPath = path.join(outputDir, `${prefix}.html`);
  const jsonPath = path.join(outputDir, `${prefix}.json`);

  await Promise.all([
    page.screenshot({ path: screenshotPath, fullPage: true }).catch(async (error) => {
      await writeFile(
        screenshotPath.replace(/\.png$/u, ".txt"),
        `failed to capture screenshot: ${String(error?.message || error)}\n`,
        "utf8",
      );
    }),
    writeFile(htmlPath, html, "utf8"),
    writeFile(
      jsonPath,
      JSON.stringify(
        {
          suite: options.suite || "desktop-gui",
          scenario: options.scenario || "failure",
          viewport,
          capturedAt: capturedAt.toISOString(),
          state,
          error: options.error ? String(options.error?.stack || options.error?.message || options.error) : null,
        },
        null,
        2,
      ),
      "utf8",
    ),
  ]);

  return { outputDir, screenshotPath, htmlPath, jsonPath };
}
