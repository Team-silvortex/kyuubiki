#!/usr/bin/env node

import fs from "node:fs/promises";
import path from "node:path";
import {
  installationIntegrityContractPath,
  installationIntegrityDocs,
  readJson,
  rootDir,
  updateChannelsPath,
} from "./release-metadata.mjs";

async function main() {
  const contractSource = readJson(installationIntegrityContractPath);
  const channelContract = readJson(updateChannelsPath);
  const contract = {
    ...contractSource,
    shipping_version: contractSource.shipping_version ?? channelContract.shipping_version,
  };
  const docsHtml = renderHtml(contract, {
    title: "Installation Integrity Contract",
    kicker: "Installation Contract",
    cssHref: installationIntegrityDocs.docsCssHref,
    extraCopy: "",
    links: [
      { href: "./README.md", label: "Back to docs readme" },
      { href: installationIntegrityDocs.docsJsonHref, label: "Open JSON source" },
    ],
  });
  const hubHtml = renderHtml(contract, {
    title: "Installation Integrity Contract",
    kicker: "Installation Contract Mirror · Chapter 7",
    cssHref: installationIntegrityDocs.hubCssHref,
    extraCopy:
      "This is the Hub mirror for the trust-and-safety chapter's installation integrity material.",
    links: [
      { href: "./index.html", label: "Back to book entry" },
      { href: "../../../../docs/book-ch07-trust-and-safety.html", label: "Open central chapter" },
      { href: "../../../../docs/installation-integrity-contract.html", label: "Open source page" },
      { href: installationIntegrityDocs.hubJsonHref, label: "Open JSON source" },
    ],
  });

  await fs.writeFile(installationIntegrityDocs.docsOutputPath, docsHtml);
  await fs.writeFile(installationIntegrityDocs.hubOutputPath, hubHtml);

  process.stdout.write(
    [
      `wrote ${path.relative(rootDir, installationIntegrityDocs.docsOutputPath)}`,
      `wrote ${path.relative(rootDir, installationIntegrityDocs.hubOutputPath)}`,
    ].join("\n"),
  );
}

function renderHtml(contract, options) {
  const version = escapeHtml(contract.shipping_version || "unknown");
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
        ${options.extraCopy ? `<p class="docs-copy">\n          ${escapeHtml(options.extraCopy)}\n        </p>` : ""}
        <div class="docs-meta">
          <span class="docs-chip">Shipping version: ${version}</span>
          <span class="docs-chip">Schema: ${schemaVersion}</span>
        </div>
        <div class="docs-links">
          ${options.links
            .map(
              (link) =>
                `<a class="docs-link" href="${escapeHtml(link.href)}">${escapeHtml(link.label)}</a>`,
            )
            .join("\n          ")}
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
