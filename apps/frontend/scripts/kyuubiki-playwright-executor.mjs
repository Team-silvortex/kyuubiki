import { mkdir } from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { findAutomationActionContract } from "./kyuubiki-automation-actions.mjs";

function normalizeActionName(action) {
  return String(action ?? "")
    .trim()
    .toLowerCase()
    .replaceAll(/[.\s-]+/g, "_");
}

function pickFirstString(payload, keys) {
  for (const key of keys) {
    if (typeof payload?.[key] === "string" && payload[key].trim()) return payload[key].trim();
  }
  return null;
}

function pickNumber(payload, keys, fallback) {
  for (const key of keys) {
    if (typeof payload?.[key] === "number" && Number.isFinite(payload[key])) return payload[key];
    if (typeof payload?.[key] === "string" && payload[key].trim()) {
      const value = Number(payload[key]);
      if (Number.isFinite(value)) return value;
    }
  }
  return fallback;
}

function ensurePage(context) {
  if (!context.page) throw new Error("Browser page is not initialized.");
  return context.page;
}

async function ensureArtifactsDirectory(artifactsDir) {
  await mkdir(artifactsDir, { recursive: true });
  return artifactsDir;
}

function snapshotFileName(step) {
  const suffix = String(step.action ?? "step")
    .trim()
    .toLowerCase()
    .replaceAll(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${String(step.index + 1).padStart(2, "0")}-${suffix || "snapshot"}.png`;
}

async function executeOpen(step, runtime) {
  const page = ensurePage(runtime);
  const payload = step.payload ?? {};
  const url = pickFirstString(payload, ["url", "href"]);
  if (!url) throw new Error("open_page requires payload.url");
  const waitUntil = pickFirstString(payload, ["waitUntil", "wait_until"]) ?? "networkidle";
  const timeout = pickNumber(payload, ["timeout", "timeoutMs", "timeout_ms"], 30_000);
  const response = await page.goto(url, { waitUntil, timeout });
  return {
    message: `Opened ${url}`,
    result: {
      url: page.url(),
      status: response?.status() ?? null,
      ok: response?.ok() ?? null,
    },
  };
}

async function executeClick(step, runtime) {
  const page = ensurePage(runtime);
  const payload = step.payload ?? {};
  const selector = pickFirstString(payload, ["selector", "target"]);
  if (!selector) throw new Error("click requires payload.selector");
  const timeout = pickNumber(payload, ["timeout", "timeoutMs", "timeout_ms"], 15_000);
  await page.locator(selector).click({ timeout });
  return { message: `Clicked ${selector}`, result: { selector } };
}

async function executeType(step, runtime) {
  const page = ensurePage(runtime);
  const payload = step.payload ?? {};
  const selector = pickFirstString(payload, ["selector", "target"]);
  if (!selector) throw new Error("type requires payload.selector");
  const value = payload.value ?? payload.text ?? payload.input;
  if (typeof value !== "string") throw new Error("type requires payload.value");
  const timeout = pickNumber(payload, ["timeout", "timeoutMs", "timeout_ms"], 15_000);
  await page.locator(selector).fill(value, { timeout });
  return { message: `Filled ${selector}`, result: { selector, value } };
}

async function executePress(step, runtime) {
  const page = ensurePage(runtime);
  const payload = step.payload ?? {};
  const key = pickFirstString(payload, ["key"]);
  if (!key) throw new Error("press requires payload.key");
  const selector = pickFirstString(payload, ["selector", "target"]);
  const timeout = pickNumber(payload, ["timeout", "timeoutMs", "timeout_ms"], 15_000);
  if (selector) {
    await page.locator(selector).press(key, { timeout });
    return { message: `Pressed ${key} on ${selector}`, result: { selector, key } };
  }
  await page.keyboard.press(key);
  return { message: `Pressed ${key}`, result: { key } };
}

async function executeSelect(step, runtime) {
  const page = ensurePage(runtime);
  const payload = step.payload ?? {};
  const selector = pickFirstString(payload, ["selector", "target"]);
  if (!selector) throw new Error("select requires payload.selector");
  const timeout = pickNumber(payload, ["timeout", "timeoutMs", "timeout_ms"], 15_000);
  const values = Array.isArray(payload.values)
    ? payload.values.map((entry) => String(entry))
    : pickFirstString(payload, ["value"])
      ? [pickFirstString(payload, ["value"])]
      : [];
  if (values.length === 0) throw new Error("select requires payload.value");
  await page.locator(selector).selectOption(values, { timeout });
  return { message: `Selected ${values.join(", ")} on ${selector}`, result: { selector, values } };
}

async function executeWait(step, runtime) {
  const page = ensurePage(runtime);
  const payload = step.payload ?? {};
  const selector = pickFirstString(payload, ["selector", "target"]);
  const timeout = pickNumber(payload, ["timeout", "timeoutMs", "timeout_ms", "duration", "durationMs"], 1_000);
  const text = pickFirstString(payload, ["text", "value"]);
  const state = pickFirstString(payload, ["state"]) ?? "attached";
  if (selector) {
    const locator = page.locator(selector);
    if (text) {
      await locator.filter({ hasText: text }).first().waitFor({ state, timeout });
      return {
        message: `Waited for ${selector} text "${text}"`,
        result: { selector, text, state, timeout_ms: timeout },
      };
    }
    await locator.first().waitFor({ state, timeout });
    return { message: `Waited for ${selector}`, result: { selector, state, timeout_ms: timeout } };
  }
  await page.waitForTimeout(timeout);
  return { message: `Waited ${timeout}ms`, result: { timeout_ms: timeout } };
}

async function executeSnapshot(step, runtime) {
  const page = ensurePage(runtime);
  const payload = step.payload ?? {};
  const artifactsDir = await ensureArtifactsDirectory(runtime.artifactsDir);
  const outputName = pickFirstString(payload, ["file", "filename", "name"]) ?? snapshotFileName(step);
  const outputPath = path.resolve(artifactsDir, outputName);
  await page.screenshot({ path: outputPath, fullPage: Boolean(payload.fullPage ?? payload.full_page ?? true) });
  return { message: `Saved snapshot to ${outputPath}`, result: { path: outputPath } };
}

async function executeAssertText(step, runtime) {
  const page = ensurePage(runtime);
  const payload = step.payload ?? {};
  const selector = pickFirstString(payload, ["selector", "target"]);
  const text = pickFirstString(payload, ["text"]);
  if (!selector) throw new Error("assert_text requires payload.selector");
  if (!text) throw new Error("assert_text requires payload.text");
  const timeout = pickNumber(payload, ["timeout", "timeoutMs", "timeout_ms"], 15_000);
  await page.locator(selector).filter({ hasText: text }).first().waitFor({ state: "attached", timeout });
  return { message: `Asserted ${selector} contains "${text}"`, result: { selector, text } };
}

function resolveActionExecutor(step) {
  const contract = findAutomationActionContract(step.action);
  switch (normalizeActionName(contract?.id ?? step.action)) {
    case "open_page":
      return executeOpen;
    case "click":
      return executeClick;
    case "type":
      return executeType;
    case "press":
      return executePress;
    case "select":
      return executeSelect;
    case "wait":
      return executeWait;
    case "assert_text":
      return executeAssertText;
    case "snapshot":
      return executeSnapshot;
    default:
      return null;
  }
}

export async function createPlaywrightExecutor(options = {}) {
  const browser = await chromium.launch({ headless: options.headless !== false });
  const context = await browser.newContext({
    viewport: options.viewport ?? { width: 1440, height: 900 },
    baseURL: options.baseUrl ?? undefined,
  });
  const page = await context.newPage();
  const runtime = {
    browser,
    context,
    page,
    artifactsDir: path.resolve(options.artifactsDir ?? path.join(process.cwd(), ".kyuubiki", "headless-artifacts")),
  };

  return {
    artifactsDir: runtime.artifactsDir,
    executor: async (step) => {
      const actionExecutor = resolveActionExecutor(step);
      if (!actionExecutor) {
        throw new Error(`Unsupported live action: ${step.action}`);
      }
      const result = await actionExecutor(step, runtime);
      return {
        executor: "playwright",
        ...result,
      };
    },
    dispose: async () => {
      await page.close();
      await context.close();
      await browser.close();
    },
  };
}
