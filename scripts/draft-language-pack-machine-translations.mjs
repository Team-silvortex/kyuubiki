#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const input = option("--in");
const output = option("--out");
const target = option("--target");
const customEndpoint = option("--endpoint");
const endpoint = customEndpoint ?? "https://translate.googleapis.com/translate_a/single";

function option(name) {
  const index = process.argv.indexOf(name);
  return index === -1 ? undefined : process.argv[index + 1];
}

function isBlank(value) {
  return typeof value === "string" ? !value.trim() : Array.isArray(value) && value.length === 0;
}

async function translate(text) {
  let lastError;
  for (let attempt = 1; attempt <= 6; attempt += 1) {
    try {
      const url = new URL(endpoint);
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
      const translated = payload?.[0]?.map((entry) => entry?.[0]).join("");
      if (typeof translated !== "string" || !translated.trim()) throw new Error("translation response was empty");
      return translated;
    } catch (error) {
      lastError = error;
      if (error.status === 429 && !customEndpoint) return translateWithMyMemory(text, error);
      if (attempt < 6) {
        const delayMs = Number.isFinite(error.delayMs) ? error.delayMs : 2 ** attempt * 250;
        await new Promise((resolve) => setTimeout(resolve, delayMs));
      }
    }
  }
  if (customEndpoint) throw lastError;
  return translateWithMyMemory(text, lastError);
}

async function translateWithMyMemory(text, primaryError) {
  const url = new URL("https://api.mymemory.translated.net/get");
  url.searchParams.set("q", text);
  url.searchParams.set("langpair", `en|${target}`);
  const response = await fetch(url, { signal: AbortSignal.timeout(20_000) });
  if (!response.ok) throw primaryError;
  const payload = await response.json();
  const translated = payload?.responseData?.translatedText;
  if (typeof translated !== "string" || !translated.trim()) throw primaryError;
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
  if (!isBlank(entry.translation)) continue;
  entry.translation = Array.isArray(entry.source)
    ? await Promise.all(entry.source.map(translate))
    : await translate(entry.source);
  checkpoint();
  console.log(`translated ${index + 1}/${batch.strings.length}: ${entry.key}`);
}
checkpoint();
console.log(`drafted ${batch.batch} machine translations for ${target}: ${output}`);
