import { applyDesktopState } from "./shared/tauri-bridge.js";

export function mountUpdatePanel() {
  const root = document.getElementById("update-panel-root");
  if (!root) {
    return;
  }

  root.innerHTML = `
    <div class="section-header desktop-shell-section-header">
      <div><p class="section-eyebrow desktop-shell-eyebrow">Unified delivery</p><h2>Update channels</h2></div>
      <div class="action-row desktop-shell-action-row">
        <label class="field update-panel__channel">
          <span>Channel</span>
          <select id="update-channel-select">
            <option value="stable">stable</option>
            <option value="tamamono:stable">tamamono:stable</option>
            <option value="tamamono:latest">tamamono:latest</option>
          </select>
        </label>
        <button class="primary desktop-shell-button-primary" data-action="refresh-update-plan">Refresh update plan</button>
        <button data-action="refresh-update-preview">Preview update</button>
      </div>
    </div>
    <div class="update-summary-grid">
      <article class="update-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">State</span><strong id="update-state-headline">Loading unified update plan…</strong><span class="integrity-pill" data-desktop-state="health" id="update-state-pill">unknown</span></article>
      <article class="update-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Current</span><strong id="update-current-version">1.6.0</strong><p>Current installed shipping version from the visible install contract.</p></article>
      <article class="update-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Target</span><strong id="update-target-version">1.6.0</strong><p id="update-target-channel">stable</p></article>
      <article class="update-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Artifacts</span><strong id="update-artifact-count">0 references</strong><p>Desktop bundles declared by the selected delivery channel.</p></article>
    </div>
    <div class="integrity-policy-grid">
      <article class="integrity-policy-card desktop-shell-surface-card">
        <div class="panel-header desktop-shell-section-header"><h3>Channel posture</h3><span class="desktop-shell-chip" id="update-schema-version">kyuubiki.update-catalog/v1</span></div>
        <p id="update-summary-copy">Waiting for update plan…</p>
        <p id="update-guidance-copy">Refresh the selected channel to see whether this install is current, behind, or ahead of that target.</p>
      </article>
      <article class="integrity-policy-card desktop-shell-surface-card">
        <div class="panel-header desktop-shell-section-header"><h3>Rendered plan</h3></div>
        <pre id="update-report-output">Waiting for unified update plan…</pre>
      </article>
    </div>
    <div class="integrity-panel-grid">
      <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Visible update rules</h3></div><div class="integrity-contract-list" id="update-rule-list"></div></article>
      <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Desktop artifact references</h3></div><div class="update-artifact-list" id="update-artifact-list"></div></article>
    </div>
    <div class="integrity-panel-grid">
      <article class="integrity-shell desktop-shell-surface-card">
        <div class="panel-header desktop-shell-section-header"><h3>Update preview</h3><span class="integrity-pill" data-desktop-state="health" id="update-preview-pill">unknown</span></div>
        <div class="update-preview-summary">
          <div><span>Blocking issues</span><strong id="update-preview-blockers">0</strong></div>
          <div><span>Removable residue</span><strong id="update-preview-residue">0</strong></div>
        </div>
        <div class="update-preview-steps" id="update-preview-steps"></div>
        <pre id="update-preview-output">Run update preview to inspect pre-apply checks.</pre>
      </article>
    </div>
  `;
}

function renderRules(rules) {
  const list = document.getElementById("update-rule-list");
  if (!list) {
    return;
  }

  list.innerHTML = "";
  rules.forEach((rule) => {
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
        <span class="integrity-pill" data-desktop-state="health">read-only</span>
      </div>
    `;
    const pills = row.querySelectorAll(".integrity-pill");
    applyDesktopState(pills[0], rule.category, { kind: "activity" });
    applyDesktopState(pills[1], "idle", { kind: "health" });
    list.appendChild(row);
  });
}

function renderArtifacts(artifacts) {
  const list = document.getElementById("update-artifact-list");
  if (!list) {
    return;
  }

  list.innerHTML = "";
  artifacts.forEach((artifact) => {
    const row = document.createElement("article");
    row.className = "update-artifact-row";
    row.innerHTML = `
      <div>
        <strong>${artifact.product} · ${artifact.platform} · ${artifact.kind}</strong>
        <p>${artifact.exists ? "Present on this workspace host." : "Declared by channel, not currently present here."}</p>
        <code>${artifact.path}</code>
      </div>
      <span class="integrity-pill" data-desktop-state="health">${artifact.exists ? "present" : "declared"}</span>
    `;
    applyDesktopState(row.querySelector(".integrity-pill"), artifact.exists ? "ok" : "idle", {
      kind: "health",
    });
    list.appendChild(row);
  });
}

export function renderUpdatePreview(preview) {
  const steps = Array.isArray(preview?.steps) ? preview.steps : [];
  const container = document.getElementById("update-preview-steps");
  if (container) {
    container.innerHTML = "";
    steps.forEach((step) => {
      const row = document.createElement("article");
      row.className = "update-preview-step";
      row.innerHTML = `
        <div>
          <strong>${step.label}</strong>
          <p>${step.detail}</p>
        </div>
        <span class="integrity-pill" data-desktop-state="health">${step.status}</span>
      `;
      applyDesktopState(row.querySelector(".integrity-pill"), step.status, { kind: "health" });
      container.appendChild(row);
    });
  }

  document.getElementById("update-preview-blockers").textContent = String(preview?.blocking_issues ?? 0);
  document.getElementById("update-preview-residue").textContent = String(preview?.removable_residue ?? 0);
  document.getElementById("update-preview-output").textContent =
    preview?.rendered || "Unified update preview unavailable.";
  document.getElementById("update-preview-pill").textContent = preview?.overall_status || "unknown";
  applyDesktopState(document.getElementById("update-preview-pill"), preview?.overall_status || "unknown", {
    kind: "health",
  });
}

export function selectedUpdateChannel() {
  return document.getElementById("update-channel-select")?.value || "stable";
}

export function renderUpdatePlan(plan) {
  const rules = Array.isArray(plan?.contract_rules) ? plan.contract_rules : [];
  const artifacts = Array.isArray(plan?.artifacts) ? plan.artifacts : [];
  const state = plan?.update_state || "unknown";
  const headline =
    state === "up_to_date"
      ? `Selected channel is aligned with ${plan?.target_version || "the current version"}.`
      : state === "update_available"
        ? `Channel target ${plan?.target_version || ""} is newer than the current install.`
        : state === "ahead_of_channel"
          ? "Current install is ahead of the selected channel target."
          : "Update state is unknown.";

  document.getElementById("update-state-headline").textContent = headline;
  document.getElementById("update-state-pill").textContent = state.replaceAll("_", " ");
  document.getElementById("update-current-version").textContent = plan?.current_version || "unknown";
  document.getElementById("update-target-version").textContent = plan?.target_version || "unknown";
  document.getElementById("update-target-channel").textContent = `${plan?.target_channel || "unknown"} · ${plan?.target_tag || ""}`;
  document.getElementById("update-artifact-count").textContent = `${artifacts.length} references`;
  document.getElementById("update-schema-version").textContent = plan?.schema_version || "kyuubiki.update-catalog/v1";
  document.getElementById("update-summary-copy").textContent = plan?.summary || "No summary available.";
  document.getElementById("update-guidance-copy").textContent =
    state === "update_available"
      ? "This workspace can now surface a concrete upgrade target before we add the actual apply-update executor."
      : "Update behavior stays visible: channel tag, concrete shipped version, cleanup posture, and rollback posture are all declared here.";
  document.getElementById("update-report-output").textContent =
    plan?.rendered || "Unified update plan unavailable.";
  applyDesktopState(document.getElementById("update-state-pill"), state, { kind: "health" });
  renderRules(rules);
  renderArtifacts(artifacts);
}
