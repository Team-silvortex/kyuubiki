#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const input = option("--in");
const output = option("--out");
const target = option("--target");
const force = process.argv.includes("--force");
const customEndpoint = option("--endpoint");
const endpoint = customEndpoint ?? "https://translate.googleapis.com/translate_a/single";
const endpointUrl = new URL(endpoint);
const useMyMemoryPrimary = endpointUrl.hostname.includes("mymemory.translated.net");
const useTranslateShell = endpointUrl.protocol === "translate-shell:";
const translateShellEngine = endpointUrl.pathname?.replace(/^\//, "") || "bing";
const translateShellFallbackEndpoint = "https://translate.googleapis.com/translate_a/single";
const forceCloudFallback = useTranslateShell || (!customEndpoint || useMyMemoryPrimary);
const maxAttempts = Number(option("--attempts")) || 20;
const maxDelayMs = 8_000;
const pauseBetweenRequestsMs = Math.max(0, Number(option("--pause-ms")) || 250);

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function option(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
}

function isBlank(value) {
  return typeof value === "string" ? !value.trim() : Array.isArray(value) && value.length === 0;
}

function isSourceTranslation(translation, source) {
  return JSON.stringify(translation) === JSON.stringify(source);
}

function decodeHtmlEntities(value) {
  return value
    .replace(/&amp;/g, "&")
    .replace(/&quot;/g, "\"")
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">");
}

function normalizeTranslationCandidate(value) {
  return typeof value === "string" ? value.trim() : "";
}

function pickMymemoryTranslation(payload) {
  const responseText = normalizeTranslationCandidate(payload?.responseData?.translatedText);
  if (responseText) return responseText;

  const matches = Array.isArray(payload?.matches) ? payload.matches : [];
  for (const match of matches) {
    const candidate = normalizeTranslationCandidate(match?.translation);
    if (candidate) return candidate;
  }

  const responseTextNoPunctuation = normalizeTranslationCandidate(payload?.responseData?.match);
  if (responseTextNoPunctuation) return responseTextNoPunctuation;

  return "";
}

function normalizeTranslation(text) {
  return decodeHtmlEntities(text).trim();
}

function normalizeTranslateShellTranslation(value) {
  return typeof value === "string" ? value.trim() : "";
}

function fallbackTranslation(value) {
  return "";
}

async function translateWithShell(text) {
  try {
    const result = execFileSync(
      "trans",
      ["-b", "-e", translateShellEngine, `:${target}`, text],
      { stdio: ["ignore", "pipe", "pipe"], encoding: "utf8", timeout: 20_000 },
    );
    const translated = normalizeTranslateShellTranslation(result);
    if (!translated) throw new Error("translate-shell returned empty output");
    return translated;
  } catch (error) {
    const wrapped = new Error(`translate-shell request failed: ${error.message}`);
    throw wrapped;
  }
}

async function translate(text) {
  if (useTranslateShell) {
    try {
      const translated = normalizeTranslation(await translateWithShell(text));
      if (translated === text) throw new Error("translation output was identical to source text");
      await sleep(pauseBetweenRequestsMs);
      return translated;
    } catch (error) {
      console.warn(`translate-shell fallback: ${error.message}`);
      if (!forceCloudFallback) return fallbackTranslation(text);
    }
  }
  if (useMyMemoryPrimary) {
    let lastError;
    for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
      try {
        const translated = normalizeTranslation(await translateWithMyMemory(text, undefined, endpoint));
        if (translated === text) throw new Error("translation output was identical to source text");
        await sleep(pauseBetweenRequestsMs);
        return translated;
      } catch (error) {
        lastError = error;
        if (error.status === 429 && attempt < maxAttempts) {
          const delayMs = Math.min(maxDelayMs, Number.isFinite(error.delayMs) ? error.delayMs : 2 ** attempt * 500);
          await new Promise((resolve) => setTimeout(resolve, delayMs));
          continue;
        }
        throw error;
      }
    }
    throw lastError;
  }
  let lastError;
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      const url = new URL(useTranslateShell ? translateShellFallbackEndpoint : endpoint);
      url.searchParams.set("client", "gtx");
      url.searchParams.set("sl", "en");
      url.searchParams.set("tl", target);
      url.searchParams.set("dt", "t");
      url.searchParams.set("q", text);
      const response = await fetch(url, { signal: AbortSignal.timeout(20_000) });
      if (!response.ok) {
        const retryAfter = Number(response.headers.get("retry-after"));
        const delayMs = Number.isFinite(retryAfter) && retryAfter > 0
          ? retryAfter * 1_000
          : 2 ** attempt * 500;
        const error = new Error(`translation request failed: ${response.status}`);
        error.status = response.status;
        error.delayMs = delayMs;
        throw error;
      }
      const payload = await response.json();
      const translated = normalizeTranslation(payload?.[0]?.map((entry) => entry?.[0]).join(""));
      if (typeof translated !== "string" || !translated.trim()) throw new Error("translation response was empty");
      if (translated === text) throw new Error("translation output was identical to source text");
      await sleep(pauseBetweenRequestsMs);
      return translated;
    } catch (error) {
      lastError = error;
      if (error.status === 429 && forceCloudFallback) {
        try {
          const translated = normalizeTranslation(await translateWithMyMemory(text, error));
          if (translated === text) throw new Error("translation output was identical to source text");
          await sleep(pauseBetweenRequestsMs);
          return translated;
        } catch (fallbackError) {
          if (attempt < maxAttempts) {
            const delayMs = Math.min(maxDelayMs, Number.isFinite(fallbackError.delayMs) ? fallbackError.delayMs : 2 ** attempt * 500);
            await new Promise((resolve) => setTimeout(resolve, delayMs));
            continue;
          }
          console.warn(`translate fallback exhausted for ${text}`);
          return fallbackTranslation(text);
        }
      }
      if (attempt < 6) {
        const delayMs = Math.min(maxDelayMs, Number.isFinite(error.delayMs) ? error.delayMs : 2 ** attempt * 250);
        await new Promise((resolve) => setTimeout(resolve, delayMs));
        continue;
      }
      return fallbackTranslation(text);
    }
  }
  if (!forceCloudFallback && customEndpoint) throw lastError;
  if (!lastError) return fallbackTranslation(text);
  try {
    return await translateWithMyMemory(text, lastError);
  } catch (error) {
    console.warn(`final translation fallback: ${error.message}`);
    return fallbackTranslation(text);
  }
}

async function translateWithMyMemory(text, primaryError, customEndpointUrl = "https://api.mymemory.translated.net/get") {
  const url = new URL(customEndpointUrl);
  url.searchParams.set("q", text);
  url.searchParams.set("langpair", `en|${target}`);
  const response = await fetch(url, { signal: AbortSignal.timeout(20_000) });
  if (!response.ok) {
    if (primaryError instanceof Error) throw primaryError;
    const error = new Error(`mymemory translation request failed: ${response.status}`);
    error.status = response.status;
    throw error;
  }
  const payload = await response.json();
  const translated = normalizeTranslation(pickMymemoryTranslation(payload));
  if (
    payload?.responseStatus !== 200
    || typeof translated !== "string"
    || !translated.trim()
    || translated.includes("MYMEMORY WARNING")
  ) {
    const error = new Error("mymemory translation response was invalid");
    error.status = payload?.responseStatus;
    error.body = payload;
    if (primaryError instanceof Error) {
      error.cause = primaryError;
    }
    throw error;
  }
  return translated;
}

if (!input || !output || !target) {
  throw new Error("--in, --out, and --target are required");
}
const outputPath = path.resolve(root, output);
const batch = JSON.parse(fs.readFileSync(fs.existsSync(outputPath) ? outputPath : path.resolve(root, input), "utf8"));
if (batch.language !== target || !Array.isArray(batch.strings)) {
  throw new Error("translation batch language does not match --target");
}
function checkpoint() {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  const temporaryPath = `${outputPath}.${process.pid}.tmp`;
  fs.writeFileSync(temporaryPath, `${JSON.stringify(batch, null, 2)}\n`);
  fs.renameSync(temporaryPath, outputPath);
}
for (const [index, entry] of batch.strings.entries()) {
  const shouldTranslate = force
    ? isBlank(entry.translation) || isSourceTranslation(entry.translation, entry.source)
    : isBlank(entry.translation);
  if (!shouldTranslate) continue;
  entry.translation = Array.isArray(entry.source)
    ? await Promise.all(entry.source.map(translate))
    : await translate(entry.source);
  checkpoint();
  console.log(`translated ${index + 1}/${batch.strings.length}: ${entry.key}`);
}
checkpoint();
console.log(`drafted ${batch.batch} machine translations for ${target}: ${output}`);
