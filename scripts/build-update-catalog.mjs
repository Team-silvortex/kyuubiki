#!/usr/bin/env node

import path from "node:path";
import {
  fileExists,
  readJson,
  releaseIndexPath,
  rootDir,
  updateCatalogPath,
  updateChannelsPath,
  writeJson,
  writeText,
} from "./release-metadata.mjs";

const updateCatalogDocs = {
  docs: {
    outputPath: path.join(rootDir, "docs/update-catalog.html"),
    cssHref: "../apps/hub-gui/ui/docs/docs.css",
    kicker: "Unified update catalog",
    extraCopy: "",
    links: [
      { href: "./README.md", label: "Back to docs readme" },
      { href: "../releases/update-catalog.json", label: "Open JSON source" },
    ],
  },
  hub: {
    outputPath: path.join(rootDir, "apps/hub-gui/ui/docs/update-catalog.html"),
    cssHref: "./docs.css",
    kicker: "Unified Update Catalog Mirror · Chapter 7",
    extraCopy: "This is the Hub mirror for the trust-and-safety chapter's update visibility material.",
    links: [
      { href: "./index.html", label: "Back to book entry" },
      { href: "../../../../docs/book-ch07-trust-and-safety.html", label: "Open central chapter" },
      { href: "../../../../docs/update-catalog.html", label: "Open source page" },
    ],
  },
};

function artifactEntry(product, key, relativePath) {
  const kind = key.split("_").pop();
  const platform =
    kind === "dmg" || kind === "app"
      ? "macos"
      : kind === "appimage" || kind === "deb" || kind === "rpm"
        ? "linux"
        : kind === "msi" || kind === "nsis"
          ? "windows"
          : "unknown";

  return {
    product,
    kind,
    platform,
    path: relativePath,
    exists: fileExists(relativePath),
  };
}

function collectArtifacts(snapshot) {
  const artifacts = [];
  const desktopArtifacts = snapshot.desktop_artifacts ?? {};

  for (const [key, value] of Object.entries(desktopArtifacts)) {
    if (typeof value !== "string" || !key.includes("_")) {
      continue;
    }
    const [product] = key.split("_");
    artifacts.push(artifactEntry(product, key, value));
  }

  return artifacts;
}

function buildCatalog() {
  const releaseIndex = readJson(releaseIndexPath);
  const channels = readJson(updateChannelsPath);
  const snapshotsByVersion = new Map();

  for (const entry of releaseIndex.snapshots ?? []) {
    const snapshotPath = path.join(rootDir, "releases", entry.snapshot_path);
    snapshotsByVersion.set(entry.version, {
      meta: entry,
      snapshot: readJson(snapshotPath),
      snapshotPath: `releases/${entry.snapshot_path}`,
    });
  }

  const catalogChannels = (channels.channels ?? []).map((channel) => {
    const snapshotRef = snapshotsByVersion.get(channel.version);
    if (!snapshotRef) {
      throw new Error(`missing release snapshot for channel ${channel.id}: ${channel.version}`);
    }

    const { snapshot, snapshotPath } = snapshotRef;
    return {
      id: channel.id,
      label: channel.label,
      tag: channel.tag,
      aliases: channel.aliases ?? [],
      line: snapshot.line ?? channels.line ?? releaseIndex.line,
      status: snapshot.status,
      version: snapshot.version,
      summary: snapshot.summary,
      date: snapshot.date,
      notes: channel.notes ?? [],
      visible_rules: channel.visible_rules ?? [],
      rollout: channel.rollout ?? {},
      snapshot_path: snapshotPath,
      docs: snapshot.docs ?? {},
      product_surfaces: snapshot.product_surfaces ?? {},
      desktop_artifacts: collectArtifacts(snapshot),
    };
  });

  const versions = Array.from(snapshotsByVersion.values()).map(({ snapshot, snapshotPath }) => {
    const boundChannels = catalogChannels
      .filter((channel) => channel.version === snapshot.version)
      .map((channel) => channel.id);
    const boundTags = catalogChannels
      .filter((channel) => channel.version === snapshot.version)
      .flatMap((channel) => [channel.tag, ...(channel.aliases ?? [])]);

    return {
      version: snapshot.version,
      line: snapshot.line,
      status: snapshot.status,
      date: snapshot.date,
      summary: snapshot.summary,
      snapshot_path: snapshotPath,
      channels: boundChannels,
      tags: boundTags,
      product_surfaces: snapshot.product_surfaces ?? {},
      desktop_artifacts: collectArtifacts(snapshot),
    };
  });

  return {
    schema_version: "kyuubiki.update-catalog/v1",
    generated_at: new Date().toISOString(),
    shipping_version: channels.shipping_version,
    default_channel: channels.default_channel,
    line: channels.line ?? releaseIndex.line,
    source: {
      release_index: "releases/index.json",
      channel_contract: path.relative(rootDir, updateChannelsPath),
    },
    channels: catalogChannels,
    versions,
  };
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function renderRules(rules) {
  return rules
    .map(
      (rule) => `<li><strong>${escapeHtml(rule.label)}</strong>: ${escapeHtml(rule.value)}<br />${escapeHtml(rule.description)}</li>`,
    )
    .join("\n");
}

function renderArtifacts(artifacts) {
  return artifacts
    .map(
      (artifact) =>
        `<li><strong>${escapeHtml(artifact.product)}</strong> · ${escapeHtml(artifact.platform)} · ${escapeHtml(artifact.kind)} · <code>${escapeHtml(artifact.path)}</code> · ${artifact.exists ? "present" : "declared"}</li>`,
    )
    .join("\n");
}

function renderChannels(channels) {
  return channels
    .map(
      (channel) => `
        <article class="docs-card">
          <div class="docs-kicker">channel</div>
          <h2>${escapeHtml(channel.label)} <code>${escapeHtml(channel.tag)}</code></h2>
          <p class="docs-copy">${escapeHtml(channel.summary)}</p>
          <div class="docs-meta">
            <span class="docs-chip">Version: ${escapeHtml(channel.version)}</span>
            <span class="docs-chip">Status: ${escapeHtml(channel.status)}</span>
            <span class="docs-chip">Aliases: ${escapeHtml((channel.aliases ?? []).join(", ") || "none")}</span>
          </div>
          <h3>Visible rules</h3>
          <ul class="docs-list">
            ${renderRules(channel.visible_rules ?? [])}
          </ul>
          <h3>Desktop artifact references</h3>
          <ul class="docs-list">
            ${renderArtifacts(channel.desktop_artifacts ?? [])}
          </ul>
        </article>`,
    )
    .join("\n");
}

function renderHtml(catalog, options) {
  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Kyuubiki Unified Update Catalog</title>
    <link rel="stylesheet" href="${escapeHtml(options.cssHref)}" />
  </head>
  <body>
    <main class="docs-shell">
      <section class="docs-hero">
        <div class="docs-kicker">${escapeHtml(options.kicker)}</div>
        <h1>${escapeHtml(catalog.line)} delivery channels</h1>
        <p class="docs-copy">
          This page is generated from the shared release index and the human-owned update channel contract.
          It defines the visible, Docker-like update tags that point to concrete shipped versions.
        </p>
        ${options.extraCopy ? `<p class="docs-copy">\n          ${escapeHtml(options.extraCopy)}\n        </p>` : ""}
        <div class="docs-meta">
          <span class="docs-chip">Shipping version: ${escapeHtml(catalog.shipping_version)}</span>
          <span class="docs-chip">Default channel: ${escapeHtml(catalog.default_channel)}</span>
          <span class="docs-chip">Schema: ${escapeHtml(catalog.schema_version)}</span>
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

      <section class="docs-section">
        <article class="docs-card">
          <div class="docs-kicker">source of truth</div>
          <h2>How this stays unified</h2>
          <ul class="docs-list">
            <li><strong>channel contract</strong>: <code>${escapeHtml(catalog.source.channel_contract)}</code></li>
            <li><strong>release registry</strong>: <code>${escapeHtml(catalog.source.release_index)}</code></li>
            <li><strong>tag model</strong>: human-facing channels point to immutable shipped versions</li>
            <li><strong>installer posture</strong>: update behavior must stay visible and bounded by the installation contract</li>
          </ul>
        </article>
      </section>

      <section class="docs-grid">
        ${renderChannels(catalog.channels ?? [])}
      </section>
    </main>
  </body>
</html>
`;
}

const catalog = buildCatalog();
writeJson(updateCatalogPath, catalog);
writeText(updateCatalogDocs.docs.outputPath, renderHtml(catalog, updateCatalogDocs.docs));
writeText(updateCatalogDocs.hub.outputPath, renderHtml(catalog, updateCatalogDocs.hub));

console.log(`wrote ${path.relative(rootDir, updateCatalogPath)}`);
console.log(`wrote ${path.relative(rootDir, updateCatalogDocs.docs.outputPath)}`);
console.log(`wrote ${path.relative(rootDir, updateCatalogDocs.hub.outputPath)}`);
