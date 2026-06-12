import { applyDesktopState } from "./shared/tauri-bridge.js";

export function mountIntegrityPanel() {
  const root = document.getElementById("integrity-panel-root");
  if (!root) {
    return;
  }

  root.innerHTML = `
    <div class="section-header desktop-shell-section-header">
      <div><p class="section-eyebrow desktop-shell-eyebrow">Install contract</p><h2>Installation integrity</h2></div>
      <div class="action-row desktop-shell-action-row">
        <button class="primary desktop-shell-button-primary" data-action="refresh-integrity">Refresh report</button>
        <button data-action="repair-installation">Repair installation</button>
      </div>
    </div>
    <div class="integrity-subnav">
      <button class="integrity-subtab integrity-subtab--active" type="button" data-integrity-tab="overview">Overview</button>
      <button class="integrity-subtab" type="button" data-integrity-tab="details">Detailed contract</button>
    </div>
    <section class="integrity-subview integrity-subview--active" data-integrity-view="overview">
      <div class="integrity-summary-grid">
        <article class="integrity-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Status</span><strong id="integrity-headline">Loading installation integrity…</strong><span class="integrity-pill" data-desktop-state="health" id="integrity-status-pill">unknown</span></article>
        <article class="integrity-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Issues</span><strong id="integrity-issue-count">0 issues</strong><p>Version drift, missing standard paths, and cleanup candidates are collected here.</p></article>
        <article class="integrity-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Residue</span><strong id="integrity-residue-count">0 residue items</strong><p>Repair only removes known stale cache and runtime leftovers from the allowlist.</p></article>
        <article class="integrity-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Tracked size</span><strong id="integrity-layout-size">0 B</strong><p>Aggregated footprint across the standard workspace layout contract.</p></article>
      </div>
      <div class="integrity-policy-grid">
        <article class="integrity-policy-card desktop-shell-surface-card">
          <div class="panel-header desktop-shell-section-header"><h3>Standard install rules</h3><span class="desktop-shell-chip" id="integrity-schema-version">kyuubiki.installation-integrity/v1</span></div>
          <ul class="integrity-policy-list">
            <li>All platforms use the same repo-local runtime roots: \`tmp/\`, \`dist/\`, and \`releases/\`.</li>
            <li>User data and environment config are preserved during repair; only removable residue is cleaned.</li>
            <li>Desktop shells, release manifests, and runtime layout must stay version-aligned to \`tamamono 1.6.0\`.</li>
          </ul>
          <p id="integrity-policy-copy">Repair only clears known cache noise and stale runtime artifacts, so the install surface stays deterministic.</p>
        </article>
        <article class="integrity-policy-card desktop-shell-surface-card">
          <div class="panel-header desktop-shell-section-header"><h3>Current target</h3></div>
          <div class="integrity-target-grid">
            <div><span>Platform</span><strong id="integrity-platform-value">unknown</strong></div>
            <div><span>Version</span><strong id="integrity-version-value">1.6.0</strong></div>
          </div>
          <pre id="integrity-report-output">Waiting for installation integrity report…</pre>
        </article>
      </div>
      <div class="integrity-policy-grid">
        <article class="integrity-policy-card desktop-shell-surface-card">
          <div class="panel-header desktop-shell-section-header"><h3>Recommended next step</h3></div>
          <strong id="integrity-next-step">Run the first integrity report.</strong>
          <p id="integrity-breakdown">The panel will break down version alignment, required install paths, and removable residue.</p>
        </article>
        <article class="integrity-policy-card desktop-shell-surface-card">
          <div class="panel-header desktop-shell-section-header"><h3>Repair safety scope</h3></div>
          <p id="integrity-safe-scope">Repair keeps `.env.local`, model data, manifests, and desktop shells intact.</p>
        </article>
      </div>
    </section>
    <section class="integrity-subview" data-integrity-view="details">
      <div class="integrity-panel-grid">
        <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Visible behavior contract</h3></div><div class="integrity-contract-list" id="integrity-contract-list"></div></article>
      </div>
      <div class="integrity-panel-grid">
        <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Standard layout</h3></div><div class="integrity-layout-grid" id="integrity-layout-grid"></div></article>
        <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Version alignment</h3></div><div class="integrity-version-list" id="integrity-version-list"></div></article>
      </div>
      <div class="integrity-panel-grid">
        <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Cleanup candidates</h3></div><div class="integrity-residue-list" id="integrity-residue-list"></div></article>
        <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Issue log</h3></div><ul class="integrity-issues" id="integrity-issues"></ul></article>
      </div>
    </section>
  `;

  bindIntegritySubviewTabs(root);
}

function bindIntegritySubviewTabs(root) {
  const tabs = Array.from(root.querySelectorAll("[data-integrity-tab]"));
  const views = Array.from(root.querySelectorAll("[data-integrity-view]"));

  tabs.forEach((tab) => {
    tab.addEventListener("click", () => {
      const selected = tab.dataset.integrityTab;
      tabs.forEach((item) => {
        item.classList.toggle("integrity-subtab--active", item === tab);
      });
      views.forEach((view) => {
        view.classList.toggle("integrity-subview--active", view.dataset.integrityView === selected);
      });
    });
  });
}

function formatBytes(bytes) {
  const value = Number(bytes || 0);
  if (!Number.isFinite(value) || value <= 0) {
    return "0 B";
  }

  const units = ["B", "KB", "MB", "GB", "TB"];
  const exponent = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1);
  const scaled = value / 1024 ** exponent;
  return `${scaled >= 100 ? scaled.toFixed(0) : scaled.toFixed(exponent === 0 ? 0 : 1)} ${units[exponent]}`;
}

function renderSummaryCard(id, value, stateKind = "activity") {
  const element = document.getElementById(id);
  if (!element) {
    return;
  }

  element.textContent = value;
  applyDesktopState(element, value, { kind: stateKind });
}

function renderList(containerId, items, renderItem) {
  const container = document.getElementById(containerId);
  if (!container) {
    return;
  }

  container.innerHTML = "";
  items.forEach((item) => {
    container.appendChild(renderItem(item));
  });
}

function renderLayoutEntry(entry) {
  const card = document.createElement("article");
  card.className = "integrity-card desktop-shell-surface-card";
  const state = entry.present ? "present" : "missing";
  card.innerHTML = `
    <div class="integrity-card__top">
      <strong>${entry.label}</strong>
      <span class="integrity-pill" data-desktop-state="health">${state}</span>
    </div>
    <code>${entry.relative_path}</code>
    <p>${entry.required ? "Required install path" : "Optional path"} · ${formatBytes(entry.size_bytes)}</p>
  `;
  applyDesktopState(card.querySelector(".integrity-pill"), state, { kind: "health" });
  return card;
}

function renderVersionEntry(check) {
  const row = document.createElement("article");
  row.className = "integrity-version-row";
  row.innerHTML = `
    <div>
      <strong>${check.label}</strong>
      <p>Expected ${check.expected} · Actual ${check.actual}</p>
    </div>
    <span class="integrity-pill" data-desktop-state="health">${check.ok ? "aligned" : "mismatch"}</span>
  `;
  applyDesktopState(row.querySelector(".integrity-pill"), check.ok ? "ok" : "missing", {
    kind: "health",
  });
  return row;
}

function renderContractRule(rule) {
  const row = document.createElement("article");
  row.className = "integrity-contract-row";
  row.innerHTML = `
    <div>
      <strong>${rule.label}</strong>
      <p>${rule.description}</p>
      <code>${rule.value}</code>
    </div>
    <div class="integrity-contract-meta">
      <span class="integrity-pill" data-desktop-state="activity">${rule.category}</span>
      <span class="integrity-pill" data-desktop-state="health">${rule.editable ? "editable" : "read-only"}</span>
    </div>
  `;
  const pills = row.querySelectorAll(".integrity-pill");
  applyDesktopState(pills[0], rule.category, { kind: "activity" });
  applyDesktopState(pills[1], rule.editable ? "active" : "idle", { kind: "health" });
  pills[1].textContent = rule.editable ? "editable" : "read-only";
  return row;
}

function renderResidueEntry(residue) {
  const row = document.createElement("article");
  row.className = "integrity-residue-row";
  row.innerHTML = `
    <div>
      <strong>${residue.relative_path}</strong>
      <p>${residue.reason}</p>
    </div>
    <span class="integrity-pill" data-desktop-state="health">${residue.removable ? "removable" : "review"}</span>
  `;
  applyDesktopState(row.querySelector(".integrity-pill"), residue.removable ? "warning" : "active", {
    kind: "health",
  });
  return row;
}

function renderIssueEntry(issue) {
  const item = document.createElement("li");
  item.textContent = issue;
  return item;
}

export function renderIntegrityReport(report, brandConfig = null) {
  const issues = Array.isArray(report?.issues) ? report.issues : [];
  const contractRules = Array.isArray(report?.contract_rules) ? report.contract_rules : [];
  const residues = Array.isArray(report?.residues) ? report.residues : [];
  const layout = Array.isArray(report?.layout) ? report.layout : [];
  const versionChecks = Array.isArray(report?.version_checks) ? report.version_checks : [];
  const totalLayoutBytes = layout.reduce((sum, entry) => sum + Number(entry.size_bytes || 0), 0);
  const removableResidues = residues.filter((item) => item.removable);
  const missingPaths = layout.filter((entry) => entry.required && !entry.present).length;
  const versionMismatches = versionChecks.filter((check) => !check.ok).length;
  const releaseVersion = String(brandConfig?.releaseVersion || report?.current_version || "1.6.0").replace(
    /^v/u,
    "",
  );

  const headline =
    issues.length === 0
      ? `Standard install contract is healthy for tamamono ${releaseVersion}.`
      : `${issues.length} integrity issue${issues.length === 1 ? "" : "s"} need attention.`;

  const policyText =
    removableResidues.length === 0
      ? "No removable residue detected. Current installation follows the standard workspace contract."
      : `Repair can safely clean ${removableResidues.length} known residue item${
          removableResidues.length === 1 ? "" : "s"
        } without touching model data or env configuration.`;
  const nextStep =
    issues.length === 0
      ? "Installation contract is healthy. Re-run this report after releases, upgrades, or manual cleanup."
      : removableResidues.length > 0
        ? "Run Repair installation first, then refresh the report to confirm the contract is clean again."
        : versionMismatches > 0
          ? "Align version drift next. The installer contract expects every desktop surface to ship the same version."
          : missingPaths > 0
            ? "Recreate missing standard paths next. Repair installation can restore the required local layout."
            : "Review the issue log and clear the remaining contract drift before shipping this build.";
  const breakdown = `${versionMismatches} version mismatches · ${missingPaths} missing required paths · ${removableResidues.length} removable residue items`;
  const safeScope =
    removableResidues.length > 0
      ? "Repair only removes allowlisted cache noise plus stale `tmp/run/*.pid`, `tmp/run/*.sock`, `tmp/run/*.lock`, and `tmp/run/*.tmp` runtime artifacts."
      : "Repair is idle right now because no allowlisted cleanup candidate was found. It still recreates standard runtime roots when needed.";

  const reportText = String(report?.rendered || headline).trim();
  const schemaVersion = report?.schema_version || "kyuubiki.installation-integrity/v1";

  document.getElementById("integrity-headline").textContent = headline;
  document.getElementById("integrity-policy-copy").textContent = policyText;
  document.getElementById("integrity-next-step").textContent = nextStep;
  document.getElementById("integrity-breakdown").textContent = breakdown;
  document.getElementById("integrity-safe-scope").textContent = safeScope;
  document.getElementById("integrity-schema-version").textContent = schemaVersion;
  document.getElementById("integrity-platform-value").textContent = report?.platform || "unknown";
  document.getElementById("integrity-version-value").textContent = report?.current_version || releaseVersion;
  document.getElementById("integrity-report-output").textContent = reportText;

  renderSummaryCard("integrity-status-pill", issues.length === 0 ? "healthy" : "attention", "health");
  renderSummaryCard("integrity-issue-count", `${issues.length} issues`, "health");
  renderSummaryCard("integrity-residue-count", `${residues.length} residue items`, "health");
  renderSummaryCard("integrity-layout-size", formatBytes(totalLayoutBytes), "activity");

  renderList("integrity-layout-grid", layout, renderLayoutEntry);
  renderList("integrity-contract-list", contractRules, renderContractRule);
  renderList("integrity-version-list", versionChecks, renderVersionEntry);
  renderList("integrity-residue-list", residues, renderResidueEntry);

  const issueList = document.getElementById("integrity-issues");
  issueList.innerHTML = "";
  if (issues.length === 0) {
    const empty = document.createElement("li");
    empty.textContent = "No integrity issues detected.";
    issueList.appendChild(empty);
  } else {
    issues.forEach((issue) => issueList.appendChild(renderIssueEntry(issue)));
  }
}
