#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");
const contractPath = path.join(repoRoot, "deploy", "installation-integrity-contract.json");
const docsOutputPath = path.join(repoRoot, "docs", "installation-integrity-contract.html");
const hubOutputPath = path.join(repoRoot, "apps", "hub-gui", "ui", "docs", "installation-integrity.html");

const docsCssHref = "../apps/hub-gui/ui/docs/docs.css";
const hubCssHref = "./docs.css";
const docsJsonHref = "../deploy/installation-integrity-contract.json";
const hubJsonHref = "../../../../deploy/installation-integrity-contract.json";

async function main() {
  const contract = JSON.parse(await fs.readFile(contractPath, "utf8"));
  const docsHtml = renderHtml(contract, {
    title: "Installation Integrity Contract",
    kicker: "Installation Contract",
    cssHref: docsCssHref,
    backHref: "./README.md",
    backLabel: "Back to docs readme",
    jsonHref: docsJsonHref,
    jsonLabel: "Open JSON source",
  });
  const hubHtml = renderHtml(contract, {
    title: "Installation Integrity Contract",
    kicker: "Installation Contract",
    cssHref: hubCssHref,
    backHref: "./index.html",
    backLabel: "Back to docs home",
    jsonHref: hubJsonHref,
    jsonLabel: "Open JSON source",
  });

  await fs.writeFile(docsOutputPath, docsHtml);
  await fs.writeFile(hubOutputPath, hubHtml);

  process.stdout.write(
    [
      `wrote ${path.relative(repoRoot, docsOutputPath)}`,
      `wrote ${path.relative(repoRoot, hubOutputPath)}`,
    ].join("\n"),
  );
}

function renderHtml(contract, options) {
  const version = escapeHtml(contract.shipping_version || "1.4.0");
  const productLine = escapeHtml(contract.product_line || "tamamono 1.x");
  const schemaVersion = escapeHtml(contract.schema_version || "kyuubiki.installation-contract/v1");
  const requiredLayout = Array.isArray(contract.required_layout) ? contract.required_layout : [];
  const protectedPaths = Array.isArray(contract.protected_paths) ? contract.protected_paths : [];
  const removablePatterns = Array.isArray(contract.removable_patterns) ? contract.removable_patterns : [];
  const visibleRules = Array.isArray(contract.visible_rules) ? contract.visible_rules : [];

  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>${escapeHtml(options.title)}</title>
    <link rel="stylesheet" href="${escapeHtml(options.cssHref)}" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">${escapeHtml(options.kicker)}</div>
        <h1>${productLine} installation integrity</h1>
        <p class="docs-copy">
          Generated from <code>deploy/installation-integrity-contract.json</code>.
          This page exposes the same contract used by the desktop installer integrity report and repair workflow.
        </p>
        <div class="docs-meta">
          <span class="docs-chip">Shipping version: ${version}</span>
          <span class="docs-chip">Schema: ${schemaVersion}</span>
        </div>
        <div class="docs-links">
          <a class="docs-link" href="${escapeHtml(options.backHref)}">${escapeHtml(options.backLabel)}</a>
          <a class="docs-link" href="${escapeHtml(options.jsonHref)}">${escapeHtml(options.jsonLabel)}</a>
        </div>
      </section>

      <section class="docs-stack">
        <article class="docs-card">
          <h2>Visible behavior contract</h2>
          <p class="docs-copy">
            Every repair-side behavior should be operator-visible. Some rules are intentionally read-only,
            but none of them should be hidden.
          </p>
          <div class="docs-stack">
            ${visibleRules.map(renderRuleCard).join("\n")}
          </div>
        </article>

        <article class="docs-card">
          <h2>Required install layout</h2>
          <ul class="docs-list">
            ${requiredLayout
              .map(
                (entry) =>
                  `<li><strong>${escapeHtml(entry.label)}</strong>: <code>${escapeHtml(entry.path)}</code>${
                    entry.required ? " (required)" : " (optional)"
                  }</li>`,
              )
              .join("\n")}
          </ul>
        </article>

        <article class="docs-card">
          <h2>Protected paths</h2>
          <ul class="docs-list">
            ${protectedPaths.map((item) => `<li><code>${escapeHtml(item)}</code></li>`).join("\n")}
          </ul>
        </article>

        <article class="docs-card">
          <h2>Cleanup allowlist</h2>
          <ul class="docs-list">
            ${removablePatterns.map((item) => `<li><code>${escapeHtml(item)}</code></li>`).join("\n")}
          </ul>
        </article>
      </section>
    </main>
  </body>
</html>
`;
}

function renderRuleCard(rule) {
  return `<article class="docs-card">
    <div class="docs-kicker">${escapeHtml(rule.category || "rule")}</div>
    <h3>${escapeHtml(rule.label || "Untitled rule")}</h3>
    <p class="docs-copy">${escapeHtml(rule.description || "")}</p>
    <p class="docs-copy"><strong>Value:</strong> <code>${escapeHtml(rule.value || "")}</code></p>
    <p class="docs-copy"><strong>Mode:</strong> ${rule.editable ? "Editable" : "Read-only"}</p>
  </article>`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

main().catch((error) => {
  console.error(error?.stack || String(error));
  process.exitCode = 1;
});
