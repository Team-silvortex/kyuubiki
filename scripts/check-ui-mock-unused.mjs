#!/usr/bin/env node
/**
 * Lightweight UX/interaction hygiene scan for mock testing.
 *
 * Outputs:
 * - `tmp/ui-mock-interaction-audit.json`
 * - `tmp/ui-mock-interaction-audit.md`
 *
 * This is a heuristic scan, useful for catch-aid candidates.
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(new URL(".", import.meta.url))));

const TARGET_DIRS = [
  "apps/frontend/src/components",
  "apps/frontend/src/app",
  "apps/frontend/ui",
  "apps/hub-gui/ui",
  "apps/installer-gui/ui",
  "apps/workbench-gui/ui",
];

const CSS_EXT = new Set([".css", ".scss"]);
const CODE_EXT = new Set([".ts", ".tsx", ".js", ".jsx", ".mjs"]);
const IGNORE_DIR = new Set([".git", "node_modules", ".next", "dist", "build", "tmp", ".venv"]);

const EVENT_PROPS = [
  "onClick",
  "onChange",
  "onSubmit",
  "onInput",
  "onBlur",
  "onFocus",
  "onKeyDown",
  "onKeyUp",
  "onMouseEnter",
  "onMouseLeave",
  "onMouseDown",
  "onMouseUp",
  "onMouseMove",
  "onTouchStart",
  "onTouchEnd",
  "onPointerDown",
  "onPointerUp",
  "onContextMenu",
];

function isCodeFile(filePath) {
  return CODE_EXT.has(path.extname(filePath).toLowerCase());
}

function isCssFile(filePath) {
  return CSS_EXT.has(path.extname(filePath).toLowerCase());
}

function safeRead(filePath) {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch {
    return null;
  }
}

function collectFiles(dir, out = []) {
  let stat;
  try {
    stat = fs.statSync(dir);
  } catch {
    return out;
  }

  if (!stat.isDirectory()) {
    if (isCodeFile(dir) || isCssFile(dir)) out.push(dir);
    return out;
  }

  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (IGNORE_DIR.has(entry.name)) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) collectFiles(full, out);
    else if (isCodeFile(full) || isCssFile(full)) out.push(full);
  }

  return out;
}

function splitClasses(raw) {
  return raw
    .split(/\s+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function scanCodeFile(filePath, state) {
  const content = safeRead(filePath);
  if (content === null) return;

  const rel = path.relative(ROOT, filePath);
  const ext = path.extname(filePath).toLowerCase();
  if (ext !== ".tsx" && ext !== ".jsx") return;

  const lines = content.split("\n");
  const eventPropsPattern = EVENT_PROPS.join("|");
  const noopArrowRe = new RegExp(
    `\\b(on(?:${eventPropsPattern}))\\s*=\\s*{\\s*\\(?[^)]*\\)?\\s*=>\\s*{\\s*(?:return\\s*;?)?\\s*}`,
    "g",
  );
  const noopArrowEmptyRe = /=>\s*{\s*(?:return\s*;)?\s*}/g;
  const noopValueRe = /\bon(?:Click|Change|Submit|Input|Blur|Focus|KeyDown|KeyUp|MouseEnter|MouseLeave|MouseDown|MouseUp|MouseMove|TouchStart|TouchEnd|PointerDown|PointerUp|ContextMenu)\s*=\s*{\s*(undefined|null|noop|noOp|void\s+0|void 0|undefined\s*\/\/.*)?\s*}/g;

  for (let i = 0; i < lines.length; i += 1) {
    const line = lines[i];
    for (const match of line.matchAll(noopArrowRe)) {
      if (noopArrowEmptyRe.test(match[0])) {
        state.noopHandlers.push({
          file: rel,
          line: i + 1,
          prop: match[1],
          code: match[0].slice(0, 260),
        });
      }
    }

    for (const match of line.matchAll(noopValueRe)) {
      state.noopHandlers.push({
        file: rel,
        line: i + 1,
        prop: match[0].replace(/=.*/, "").trim(),
        code: match[0].slice(0, 260),
      });
    }

    const idRe = /\bid\s*=\s*["'`]([^"'`]+)["'`]/g;
    for (const match of line.matchAll(idRe)) {
      const value = match[1].trim();
      if (!value) continue;
      const refs = state.idUsages.get(value) || [];
      refs.push({ file: rel, line: i + 1 });
      state.idUsages.set(value, refs);
    }

    const classNameRe = /\bclassName\s*=\s*["'`]([^"'`]+)["'`]/g;
    for (const match of line.matchAll(classNameRe)) {
      for (const token of splitClasses(match[1].trim())) {
        if (!token || token.length <= 1) continue;
        if (token.startsWith("__") || token.startsWith("icon")) continue;
        const refs = state.classUsages.get(token) || [];
        refs.push({ file: rel, line: i + 1 });
        state.classUsages.set(token, refs);
      }
    }
  }
}

function scanCssForClasses(filePath, cssClassSet) {
  const content = safeRead(filePath);
  if (content === null) return;

  const selectorRegex = /\.([A-Za-z0-9_-]+(?:--?[A-Za-z0-9_-]*)?)\b/g;
  let match = selectorRegex.exec(content);
  while (match) {
    cssClassSet.add(match[1]);
    match = selectorRegex.exec(content);
  }
}

function uniqueByFileThenLine(list, limit) {
  return list
    .slice()
    .sort((a, b) => {
      const aFile = typeof a.file === "string" ? a.file : a.refs?.[0]?.file || "";
      const bFile = typeof b.file === "string" ? b.file : b.refs?.[0]?.file || "";
      const aLine = typeof a.line === "number" ? a.line : a.refs?.[0]?.line || 0;
      const bLine = typeof b.line === "number" ? b.line : b.refs?.[0]?.line || 0;
      return aFile.localeCompare(bFile) || aLine - bLine;
    })
    .slice(0, limit);
}

function buildReport(state) {
  const duplicateIds = [];
  for (const [id, refs] of state.idUsages.entries()) {
    if (refs.length > 1) duplicateIds.push({ id, refs });
  }

  duplicateIds.sort((a, b) => a.id.localeCompare(b.id));

  const unknownClasses = [];
  for (const [token, refs] of state.classUsages.entries()) {
    if (!state.cssClasses.has(token)) {
      unknownClasses.push({ token, refs });
    }
  }

  const payload = {
    generatedAt: new Date().toISOString(),
    targets: TARGET_DIRS.map((dir) => path.join(ROOT, dir)).filter((dir) => fs.existsSync(dir)),
    findings: {
      noopHandlers: state.noopHandlers,
      duplicateIds,
      unknownClasses: uniqueByFileThenLine(unknownClasses, Number.MAX_SAFE_INTEGER),
      summary: {
        noopHandlers: state.noopHandlers.length,
        duplicateIds: duplicateIds.length,
        unknownClasses: unknownClasses.length,
      },
    },
  };

  const lines = [];
  lines.push("# UI Mock Interaction Hygiene Report");
  lines.push(`Generated: ${payload.generatedAt}`);
  lines.push("");
  lines.push("## Summary");
  lines.push(`- No-op handlers: ${payload.findings.summary.noopHandlers}`);
  lines.push(`- Duplicate ids: ${payload.findings.summary.duplicateIds}`);
  lines.push(`- Unknown static class tokens: ${payload.findings.summary.unknownClasses}`);
  lines.push("");
  lines.push("## No-op Handlers (high confidence)");
  if (payload.findings.noopHandlers.length === 0) {
    lines.push("- No candidates found.");
  } else {
    for (const item of uniqueByFileThenLine(payload.findings.noopHandlers, 120)) {
      lines.push(`- ${item.file}:${item.line} | ${item.prop} | ${item.code}`);
    }
  }
  if (payload.findings.noopHandlers.length > 120) {
    lines.push(`- ... and ${payload.findings.noopHandlers.length - 120} more.`);
  }
  lines.push("");
  lines.push("## Duplicate ids");
  if (payload.findings.duplicateIds.length === 0) {
    lines.push("- No duplicate static ids found.");
  } else {
    for (const item of uniqueByFileThenLine(payload.findings.duplicateIds, 120)) {
      const refs = item.refs.map((entry) => `${entry.file}:${entry.line}`).join(" ; ");
      lines.push(`- ${item.id} -> ${refs}`);
    }
  }
  if (payload.findings.duplicateIds.length > 120) {
    lines.push(`- ... and ${payload.findings.duplicateIds.length - 120} more.`);
  }

  lines.push("");
  lines.push("## Unknown class tokens (static only)");
  if (payload.findings.unknownClasses.length === 0) {
    lines.push("- No static class token misses found.");
  } else {
    for (const item of payload.findings.unknownClasses.slice(0, 160)) {
      const refs = item.refs.map((entry) => `${entry.file}:${entry.line}`).join(" ; ");
      lines.push(`- ${item.token} -> ${refs}`);
    }
  }
  if (payload.findings.unknownClasses.length > 160) {
    lines.push(`- ... and ${payload.findings.unknownClasses.length - 160} more.`);
  }

  return {
    markdown: `${lines.join("\n")}\n`,
    json: payload,
  };
}

function main() {
  const state = {
    noopHandlers: [],
    idUsages: new Map(),
    classUsages: new Map(),
    cssClasses: new Set(),
  };

  for (const dir of TARGET_DIRS.map((it) => path.join(ROOT, it)).filter((it) => fs.existsSync(it))) {
    const files = collectFiles(dir, []);
    for (const filePath of files) {
      if (isCodeFile(filePath)) scanCodeFile(filePath, state);
      if (isCssFile(filePath)) scanCssForClasses(filePath, state.cssClasses);
    }
  }

  const { markdown, json } = buildReport(state);
  const reportMd = path.join(ROOT, "tmp", "ui-mock-interaction-audit.md");
  const reportJson = path.join(ROOT, "tmp", "ui-mock-interaction-audit.json");
  fs.mkdirSync(path.dirname(reportMd), { recursive: true });
  fs.writeFileSync(reportMd, markdown, "utf8");
  fs.writeFileSync(reportJson, `${JSON.stringify(json, null, 2)}\n`, "utf8");

  console.log(`no-op handlers: ${json.findings.summary.noopHandlers}`);
  console.log(`duplicate ids: ${json.findings.summary.duplicateIds}`);
  console.log(`unknown class tokens: ${json.findings.summary.unknownClasses}`);
  console.log(`saved report: ${reportMd}`);
}

main();
