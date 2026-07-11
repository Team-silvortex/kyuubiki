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
        <button data-action="prepare-update" id="prepare-update-button">Prepare staged update</button>
      </div>
    </div>
    <div class="update-summary-grid">
      <article class="update-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">State</span><strong id="update-state-headline">Loading unified update plan…</strong><span class="integrity-pill" data-desktop-state="health" id="update-state-pill">unknown</span></article>
      <article class="update-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Current</span><strong id="update-current-version">1.18.0</strong><p>Current installed development version from the visible install contract.</p></article>
      <article class="update-summary-card desktop-shell-surface-card"><span class="section-eyebrow desktop-shell-eyebrow">Target</span><strong id="update-target-version">1.18.0</strong><p id="update-target-channel">stable</p></article>
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
    <div class="integrity-policy-grid">
      <article class="integrity-policy-card desktop-shell-surface-card">
        <div class="panel-header desktop-shell-section-header"><h3>Update source</h3><span class="desktop-shell-chip" id="update-source-schema">kyuubiki.update-source/v1</span></div>
        <div class="field-grid">
          <label class="field field-span-2"><span>Catalog path</span><input id="update-source-catalog-path" type="text" placeholder="releases/update-catalog.json" /></label>
          <label class="field"><span>Artifact root</span><input id="update-source-artifact-root" type="text" placeholder="." /></label>
          <label class="field"><span>Download dir</span><input id="update-source-download-dir" type="text" placeholder="dist/downloads" /></label>
        </div>
        <div class="action-row desktop-shell-action-row">
          <button data-action="refresh-update-source">Refresh source</button>
          <button data-action="save-update-source">Save source</button>
          <button data-action="download-update">Download selected update</button>
        </div>
        <pre id="update-source-output">Update source config will appear here.</pre>
      </article>
    </div>
    <div class="integrity-panel-grid">
      <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Visible update rules</h3></div><div class="integrity-contract-list" id="update-rule-list"></div></article>
      <article class="integrity-shell desktop-shell-surface-card"><div class="panel-header desktop-shell-section-header"><h3>Desktop artifact references</h3></div><div class="update-artifact-list" id="update-artifact-list"></div></article>
    </div>
    <div class="integrity-panel-grid">
      <article class="integrity-shell desktop-shell-surface-card">
        <div class="panel-header desktop-shell-section-header"><h3>Latest downloaded update</h3><span class="integrity-pill" data-desktop-state="health" id="downloaded-update-pill">idle</span></div>
        <div class="action-row desktop-shell-action-row"><button data-action="refresh-downloaded-update">Refresh downloaded record</button><button data-action="apply-downloaded-update" id="apply-downloaded-update-button">Apply downloaded update</button></div>
        <div class="sidebar-list sidebar-list--metrics">
          <div class="sidebar-list__row"><span>channel</span><strong id="downloaded-update-channel">--</strong></div>
          <div class="sidebar-list__row"><span>target version</span><strong id="downloaded-update-version">--</strong></div>
          <div class="sidebar-list__row"><span>download dir</span><strong id="downloaded-update-dir">--</strong></div>
          <div class="sidebar-list__row"><span>manifest</span><strong id="downloaded-update-manifest">--</strong></div>
          <div class="sidebar-list__row"><span>files</span><strong id="downloaded-update-count">0</strong></div>
        </div>
        <pre id="downloaded-update-output">No downloaded update record has been created yet.</pre>
      </article>
    </div>
    <div class="integrity-panel-grid">
      <article class="integrity-shell desktop-shell-surface-card">
        <div class="panel-header desktop-shell-section-header"><h3>Latest applied update</h3><span class="integrity-pill" data-desktop-state="health" id="applied-update-pill">idle</span></div>
        <div class="action-row desktop-shell-action-row"><button data-action="refresh-applied-update">Refresh applied record</button></div>
        <div class="sidebar-list sidebar-list--metrics">
          <div class="sidebar-list__row"><span>channel</span><strong id="applied-update-channel">--</strong></div>
          <div class="sidebar-list__row"><span>target version</span><strong id="applied-update-version">--</strong></div>
          <div class="sidebar-list__row"><span>apply dir</span><strong id="applied-update-dir">--</strong></div>
          <div class="sidebar-list__row"><span>manifest</span><strong id="applied-update-manifest">--</strong></div>
          <div class="sidebar-list__row"><span>source download</span><strong id="applied-update-source-manifest">--</strong></div>
        </div>
        <pre id="applied-update-output">No applied update record has been created yet.</pre>
      </article>
    </div>
    <div class="integrity-panel-grid">
      <article class="integrity-shell desktop-shell-surface-card">
        <div class="panel-header desktop-shell-section-header"><h3>Update preview</h3><span class="integrity-pill" data-desktop-state="health" id="update-preview-pill">unknown</span></div>
        <div class="update-preview-summary">
          <div><span>Blocking issues</span><strong id="update-preview-blockers">0</strong></div>
          <div><span>Removable residue</span><strong id="update-preview-residue">0</strong></div>
        </div>
        <p id="update-apply-copy">Run update preview to see whether this workspace can prepare a staged update safely.</p>
        <div class="update-preview-steps" id="update-preview-steps"></div>
        <pre id="update-preview-output">Run update preview to inspect pre-apply checks.</pre>
      </article>
    </div>
    <div class="integrity-panel-grid">
      <article class="integrity-shell desktop-shell-surface-card">
        <div class="panel-header desktop-shell-section-header"><h3>Latest staged update</h3><span class="integrity-pill" data-desktop-state="health" id="staged-update-pill">idle</span></div>
        <div class="action-row desktop-shell-action-row">
          <button data-action="refresh-staged-update">Refresh staged record</button>
          <button data-action="reprepare-update" id="reprepare-update-button">Re-prepare current target</button>
        </div>
        <div class="sidebar-list sidebar-list--metrics">
          <div class="sidebar-list__row"><span>channel</span><strong id="staged-update-channel">--</strong></div>
          <div class="sidebar-list__row"><span>target version</span><strong id="staged-update-version">--</strong></div>
          <div class="sidebar-list__row"><span>release dir</span><strong id="staged-update-release-dir">--</strong></div>
          <div class="sidebar-list__row"><span>manifest</span><strong id="staged-update-manifest-path">--</strong></div>
          <div class="sidebar-list__row"><span>audit</span><strong id="staged-update-audit-path">--</strong></div>
        </div>
        <pre id="staged-update-output">No staged update record has been prepared yet.</pre>
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
    const body = document.createElement("div");
    const title = document.createElement("strong");
    title.textContent = rule.label;
    const description = document.createElement("p");
    description.textContent = rule.description;
    const value = document.createElement("code");
    value.textContent = rule.value;
    body.append(title, description, value);

    const meta = document.createElement("div");
    meta.className = "integrity-contract-meta";
    const categoryPill = document.createElement("span");
    categoryPill.className = "integrity-pill";
    categoryPill.dataset.desktopState = "activity";
    categoryPill.textContent = rule.category;
    const modePill = document.createElement("span");
    modePill.className = "integrity-pill";
    modePill.dataset.desktopState = "health";
    modePill.textContent = "read-only";
    meta.append(categoryPill, modePill);

    row.append(body, meta);
    applyDesktopState(categoryPill, rule.category, { kind: "activity" });
    applyDesktopState(modePill, "idle", { kind: "health" });
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
    const body = document.createElement("div");
    const title = document.createElement("strong");
    title.textContent = `${artifact.product} · ${artifact.platform} · ${artifact.kind}`;
    const description = document.createElement("p");
    description.textContent = artifact.exists
      ? "Present on this workspace host."
      : "Declared by channel, not currently present here.";
    const path = document.createElement("code");
    path.textContent = artifact.path;
    body.append(title, description, path);

    const statePill = document.createElement("span");
    statePill.className = "integrity-pill";
    statePill.dataset.desktopState = "health";
    statePill.textContent = artifact.exists ? "present" : "declared";

    row.append(body, statePill);
    applyDesktopState(statePill, artifact.exists ? "ok" : "idle", {
      kind: "health",
    });
    list.appendChild(row);
  });
}

export function renderUpdatePreview(preview) {
  const steps = Array.isArray(preview?.steps) ? preview.steps : [];
  const status = preview?.overall_status || "unknown";
  const container = document.getElementById("update-preview-steps");
  if (container) {
    container.innerHTML = "";
    steps.forEach((step) => {
      const row = document.createElement("article");
      row.className = "update-preview-step";
      const body = document.createElement("div");
      const title = document.createElement("strong");
      title.textContent = step.label;
      const detail = document.createElement("p");
      detail.textContent = step.detail;
      body.append(title, detail);

      const statusPill = document.createElement("span");
      statusPill.className = "integrity-pill";
      statusPill.dataset.desktopState = "health";
      statusPill.textContent = step.status;

      row.append(body, statusPill);
      applyDesktopState(statusPill, step.status, { kind: "health" });
      container.appendChild(row);
    });
  }

  document.getElementById("update-preview-blockers").textContent = String(preview?.blocking_issues ?? 0);
  document.getElementById("update-preview-residue").textContent = String(preview?.removable_residue ?? 0);
  document.getElementById("update-apply-copy").textContent =
    status === "ready_for_apply"
      ? "This workspace passed preview checks. You can now prepare a staged update artifact set."
      : status === "blocked"
        ? "Visible install drift or residue blockers should be cleared before preparing the staged update."
        : status === "noop"
          ? "The selected channel already matches the current development version."
          : "Refresh the update preview to determine whether staged update preparation is allowed.";
  document.getElementById("update-preview-output").textContent =
    preview?.rendered || "Unified update preview unavailable.";
  document.getElementById("update-preview-pill").textContent = status;
  document.getElementById("prepare-update-button").disabled = status !== "ready_for_apply";
  applyDesktopState(document.getElementById("update-preview-pill"), status, { kind: "health" });
}

export function selectedUpdateChannel() {
  return document.getElementById("update-channel-select")?.value || "stable";
}

export function currentUpdateSourcePayload() {
  return {
    catalogPath: document.getElementById("update-source-catalog-path")?.value.trim() || "",
    artifactRoot: document.getElementById("update-source-artifact-root")?.value.trim() || "",
    downloadDir: document.getElementById("update-source-download-dir")?.value.trim() || "",
  };
}

export function hydrateUpdateSourceConfig(config) {
  document.getElementById("update-source-schema").textContent = config?.schema_version || "kyuubiki.update-source/v1";
  document.getElementById("update-source-catalog-path").value = config?.catalog_path || "releases/update-catalog.json";
  document.getElementById("update-source-artifact-root").value = config?.artifact_root || ".";
  document.getElementById("update-source-download-dir").value = config?.download_dir || "dist/downloads";
  document.getElementById("update-source-output").textContent = config?.rendered || "Update source config unavailable.";
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
      ? "This workspace can now preview blockers and prepare a staged update flow for the selected channel."
      : "Update behavior stays visible: channel tag, concrete shipped version, cleanup posture, and rollback posture are all declared here.";
  document.getElementById("update-report-output").textContent =
    plan?.rendered || "Unified update plan unavailable.";
  applyDesktopState(document.getElementById("update-state-pill"), state, { kind: "health" });
  renderRules(rules);
  renderArtifacts(artifacts);
}

export function renderLatestStagedUpdate(record) {
  const state = record ? "ready" : "idle";
  document.getElementById("staged-update-channel").textContent = record?.channel || "--";
  document.getElementById("staged-update-version").textContent = record?.target_version || "--";
  document.getElementById("staged-update-release-dir").textContent = record?.release_dir || "--";
  document.getElementById("staged-update-manifest-path").textContent = record?.manifest_path || "--";
  document.getElementById("staged-update-audit-path").textContent = record?.audit_path || "--";
  document.getElementById("staged-update-output").textContent =
    record?.rendered || "No staged update record has been prepared yet.";
  document.getElementById("staged-update-pill").textContent = state;
  document.getElementById("reprepare-update-button").disabled = !record;
  applyDesktopState(document.getElementById("staged-update-pill"), state, { kind: "health" });
}

export function renderLatestDownloadedUpdate(record) {
  const state = record ? "ready" : "idle";
  document.getElementById("downloaded-update-channel").textContent = record?.channel || "--";
  document.getElementById("downloaded-update-version").textContent = record?.target_version || "--";
  document.getElementById("downloaded-update-dir").textContent = record?.download_dir || "--";
  document.getElementById("downloaded-update-manifest").textContent = record?.manifest_path || "--";
  document.getElementById("downloaded-update-count").textContent = String(record?.downloaded_paths?.length || 0);
  document.getElementById("downloaded-update-output").textContent = record?.rendered || "No downloaded update record has been created yet.";
  document.getElementById("downloaded-update-pill").textContent = state;
  document.getElementById("apply-downloaded-update-button").disabled = !record;
  applyDesktopState(document.getElementById("downloaded-update-pill"), state, { kind: "health" });
}

export function renderLatestAppliedUpdate(record) {
  const state = record ? "ready" : "idle";
  document.getElementById("applied-update-channel").textContent = record?.channel || "--";
  document.getElementById("applied-update-version").textContent = record?.target_version || "--";
  document.getElementById("applied-update-dir").textContent = record?.apply_dir || "--";
  document.getElementById("applied-update-manifest").textContent = record?.manifest_path || "--";
  document.getElementById("applied-update-source-manifest").textContent = record?.source_download_manifest_path || "--";
  document.getElementById("applied-update-output").textContent = record?.rendered || "No applied update record has been created yet.";
  document.getElementById("applied-update-pill").textContent = state;
  applyDesktopState(document.getElementById("applied-update-pill"), state, { kind: "health" });
}
