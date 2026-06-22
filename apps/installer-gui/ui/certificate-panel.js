function ids(id) {
  return document.getElementById(id);
}

function formatTime(unixMs) {
  return typeof unixMs === "number" && unixMs > 0 ? new Date(unixMs).toLocaleString() : "n/a";
}

function appendText(parent, tagName, text, className = "") {
  const element = document.createElement(tagName);
  if (className) element.className = className;
  element.textContent = String(text ?? "");
  parent.appendChild(element);
  return element;
}

export function mountCertificatePanel() {
  const inventoryHost = ids("certificate-inventory-list");
  const summaryHost = ids("certificate-inventory-summary");
  const revokeSelect = ids("certificate-revoke-id");
  const remoteCertificateSelect = ids("remote-certificate-id");
  let lastCertificates = [];

  const currentCertificatePolicyPayload = () => ({
    storageRoot: ids("certificate-policy-storage-root").value.trim(),
    rootCommonName: ids("certificate-policy-root-common-name").value.trim(),
    defaultValidityDays: Number(ids("certificate-policy-default-validity-days").value || "365"),
    requireForOrchestrated: ids("certificate-policy-require-orchestrated").value === "true",
    requireForOfflineMesh: ids("certificate-policy-require-offline-mesh").value === "true",
    allowSshTrustBootstrap: ids("certificate-policy-allow-ssh-bootstrap").value === "true",
  });

  const currentCertificateIssuePayload = () => ({
    label: ids("certificate-issue-label").value.trim(),
    targetHost: ids("certificate-issue-target-host").value.trim(),
    advertiseHost: ids("certificate-issue-advertise-host").value.trim(),
    agentId: ids("certificate-issue-agent-id").value.trim(),
    controlMode: ids("certificate-issue-control-mode").value || "orchestrated",
    validityDays: ids("certificate-issue-validity-days").value
      ? Number(ids("certificate-issue-validity-days").value)
      : null,
    subjectAltNames: ids("certificate-issue-sans").value
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean),
  });

  const currentCertificateRevokePayload = () => ({
    certificateId: revokeSelect.value.trim(),
  });

  function renderInventory(certificates) {
    lastCertificates = Array.isArray(certificates) ? certificates : [];
    inventoryHost.innerHTML = "";
    if (lastCertificates.length === 0) {
      inventoryHost.innerHTML =
        `<article class="certificate-record"><strong>No issued certificates</strong><span>Initialize a CA, then issue node certificates for orchestrated or offline mesh agents.</span></article>`;
      return;
    }

    lastCertificates.forEach((entry) => {
      const article = document.createElement("article");
      article.className = "certificate-record";

      const top = document.createElement("div");
      top.className = "certificate-record__top";
      appendText(top, "strong", entry.label);
      const pill = appendText(top, "span", entry.status, "certificate-record__pill");
      pill.dataset.status = entry.status || "";

      const meta = document.createElement("div");
      meta.className = "certificate-record__meta";
      appendText(meta, "span", `${entry.control_mode} · ${entry.agent_id}`);
      appendText(meta, "span", `${entry.target_host} -> ${entry.advertise_host}`);
      appendText(meta, "span", `serial ${entry.serial || "n/a"}`);
      appendText(meta, "span", `expires ${entry.not_after || "n/a"}`);

      const paths = document.createElement("div");
      paths.className = "certificate-record__paths";
      appendText(paths, "span", `cert ${entry.cert_path}`);
      appendText(paths, "span", `key ${entry.key_path}`);
      appendText(paths, "span", `issued ${formatTime(entry.issued_at_unix_ms)}`);
      appendText(paths, "span", `revoked ${formatTime(entry.revoked_at_unix_ms)}`);

      const actions = document.createElement("div");
      actions.className = "action-row";
      const useButton = document.createElement("button");
      useButton.dataset.certificateAction = "use-for-remote-agent";
      useButton.dataset.certificateId = entry.certificate_id || "";
      useButton.disabled = entry.status !== "active";
      useButton.textContent = "Use for remote agent";
      actions.appendChild(useButton);

      appendText(article, "code", entry.certificate_id);
      article.prepend(top, meta);
      appendText(article, "code", entry.fingerprint || "fingerprint unavailable");
      article.append(paths, actions);
      inventoryHost.appendChild(article);
    });
  }

  function hydrateRemoteCertificateSelect(certificates) {
    if (!remoteCertificateSelect) return;
    const currentValue = remoteCertificateSelect.value;
    remoteCertificateSelect.innerHTML = "";
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = "Auto-match active certificate";
    remoteCertificateSelect.appendChild(placeholder);
    certificates
      .filter((entry) => entry.status === "active")
      .forEach((entry) => {
        const option = document.createElement("option");
        option.value = entry.certificate_id;
        option.textContent = `${entry.label} · ${entry.agent_id} · ${entry.control_mode}`;
        option.dataset.agentId = entry.agent_id || "";
        option.dataset.controlMode = entry.control_mode || "orchestrated";
        option.dataset.targetHost = entry.target_host || "";
        option.dataset.advertiseHost = entry.advertise_host || "";
        remoteCertificateSelect.appendChild(option);
      });
    remoteCertificateSelect.value = Array.from(remoteCertificateSelect.options).some((option) => option.value === currentValue)
      ? currentValue
      : "";
  }

  function hydrateCertificateAuthority(payload) {
    if (!payload) return;
    ids("certificate-policy-storage-root").value = payload.storage_root || "";
    ids("certificate-policy-root-common-name").value = payload.root_common_name || "";
    ids("certificate-policy-default-validity-days").value = payload.default_validity_days || 365;
    ids("certificate-policy-require-orchestrated").value = payload.require_for_orchestrated ? "true" : "false";
    ids("certificate-policy-require-offline-mesh").value = payload.require_for_offline_mesh ? "true" : "false";
    ids("certificate-policy-allow-ssh-bootstrap").value = payload.allow_ssh_trust_bootstrap ? "true" : "false";
    ids("certificate-ca-state").textContent = payload.ca_initialized ? "initialized" : "not initialized";
    ids("certificate-ca-fingerprint").textContent = payload.ca_fingerprint || "(not initialized)";
    ids("certificate-ca-subject").textContent = payload.ca_subject || "(not initialized)";
    ids("certificate-ca-cert-path").textContent = payload.ca_cert_path || "";
    ids("certificate-ca-key-path").textContent = payload.ca_key_path || "";
    ids("certificate-policy-config-path").textContent = payload.config_path || "";
    ids("certificate-policy-inventory-path").textContent = payload.inventory_path || "";
    summaryHost.textContent =
      `active ${payload.active_certificate_count || 0} · revoked ${payload.revoked_certificate_count || 0} · default ${payload.default_validity_days || 365} days`;

    revokeSelect.innerHTML = "";
    const placeholder = document.createElement("option");
    placeholder.value = "";
    placeholder.textContent = "Select certificate id";
    revokeSelect.appendChild(placeholder);
    (payload.certificates || []).forEach((entry) => {
      const option = document.createElement("option");
      option.value = entry.certificate_id;
      option.textContent = `${entry.label} · ${entry.certificate_id}`;
      revokeSelect.appendChild(option);
    });

    hydrateRemoteCertificateSelect(payload.certificates || []);
    renderInventory(payload.certificates || []);
  }

  inventoryHost?.addEventListener("click", (event) => {
    const target = event.target.closest("[data-certificate-action]");
    if (!target) return;
    if (target.dataset.certificateAction !== "use-for-remote-agent") return;
    if (remoteCertificateSelect) {
      remoteCertificateSelect.value = target.dataset.certificateId || "";
      remoteCertificateSelect.dispatchEvent(new Event("change", { bubbles: true }));
    }
  });
  remoteCertificateSelect?.addEventListener("change", () => {
    const option = remoteCertificateSelect.selectedOptions?.[0];
    if (!option || !option.value) return;
    const agentId = ids("remote-agent-id");
    const controlMode = ids("remote-control-mode");
    const targetHost = ids("remote-target-host");
    const advertiseHost = ids("remote-advertise-host");
    if (agentId) agentId.value = option.dataset.agentId || agentId.value;
    if (controlMode) controlMode.value = option.dataset.controlMode || controlMode.value;
    if (targetHost && !targetHost.value.trim()) targetHost.value = option.dataset.targetHost || "";
    if (advertiseHost && !advertiseHost.value.trim()) advertiseHost.value = option.dataset.advertiseHost || "";
  });

  return {
    currentCertificateIssuePayload,
    currentCertificatePolicyPayload,
    currentCertificateRevokePayload,
    getActiveCertificates: () => lastCertificates.filter((entry) => entry?.status === "active"),
    hydrateCertificateAuthority,
  };
}
