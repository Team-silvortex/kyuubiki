export function createRemoteNodeCertificateController({
  certificateHealthHost,
  certificateFilterSelect,
  certificateBulkActionButton,
  getActiveCertificates,
  getLastNodes,
  getVisibleNodes,
  showCompletion,
  syncRegistry,
  rerender,
}) {
  const certificateFingerprintFor = (entry) =>
    [
      entry.agent_id?.trim(),
      entry.control_mode || "orchestrated",
      entry.target_host?.trim(),
      entry.advertise_host?.trim(),
    ].join("::");

  const matchCertificate = (node, certificates = getActiveCertificates?.() || []) => {
    if (!Array.isArray(certificates) || certificates.length === 0) {
      return { kind: "missing" };
    }
    const nodeFingerprint = certificateFingerprintFor(node);
    const matches = certificates.filter((entry) => certificateFingerprintFor(entry) === nodeFingerprint);
    if (matches.length === 1) {
      return { kind: "matched", certificateId: matches[0].certificate_id };
    }
    return matches.length > 1 ? { kind: "ambiguous" } : { kind: "missing" };
  };

  const certificateStatusFor = (node) => {
    const match = matchCertificate(node);
    if (node.certificate_id) {
      if (match.kind === "matched" && match.certificateId === node.certificate_id) {
        return {
          tone: "aligned",
          summary: `certificate ${node.certificate_id}`,
          detail: "bound to active inventory match",
        };
      }
      if (match.kind === "matched" && match.certificateId !== node.certificate_id) {
        return {
          tone: "stale",
          summary: `certificate ${node.certificate_id}`,
          detail: `inventory now prefers ${match.certificateId}`,
        };
      }
      return {
        tone: "stale",
        summary: `certificate ${node.certificate_id}`,
        detail: match.kind === "ambiguous" ? "binding conflicts with multiple active matches" : "binding has no active inventory match",
      };
    }
    if (match.kind === "matched") {
      return {
        tone: "available",
        summary: "certificate auto-match",
        detail: `ready to bind ${match.certificateId}`,
      };
    }
    if (match.kind === "ambiguous") {
      return {
        tone: "ambiguous",
        summary: "certificate auto-match",
        detail: "multiple active certificates match this node",
      };
    }
    return {
      tone: "missing",
      summary: "certificate auto-match",
      detail: "no active certificate match",
    };
  };

  const assignCertificatesForVisibleNodes = async () => {
    const lastNodes = getLastNodes();
    const visible = getVisibleNodes(lastNodes);
    if (visible.length === 0) throw new Error("no visible remote nodes");
    const certificates = getActiveCertificates?.() || [];
    let assigned = 0;
    let unchanged = 0;
    let missing = 0;
    let ambiguous = 0;
    const visibleIndexes = new Set(visible.map((node) => lastNodes.indexOf(node)).filter((index) => index >= 0));
    await syncRegistry((nodes) =>
      nodes.map((node, index) => {
        if (!visibleIndexes.has(index)) return node;
        const match = matchCertificate(node, certificates);
        if (match.kind === "matched") {
          if (node.certificate_id === match.certificateId) {
            unchanged += 1;
            return node;
          }
          assigned += 1;
          return { ...node, certificate_id: match.certificateId };
        }
        if (match.kind === "ambiguous") ambiguous += 1;
        if (match.kind === "missing") missing += 1;
        return node;
      }),
    );
    return { assigned, unchanged, missing, ambiguous, total: visible.length };
  };

  const assignCertificateForNodeIndex = async (nodeIndex) => {
    const lastNodes = getLastNodes();
    const node = lastNodes[nodeIndex];
    if (!node) throw new Error("remote node no longer exists");
    const certificates = getActiveCertificates?.() || [];
    const match = matchCertificate(node, certificates);
    if (match.kind === "matched") {
      await syncRegistry((nodes) =>
        nodes.map((entry, index) => (
          index === nodeIndex ? { ...entry, certificate_id: match.certificateId } : entry
        )),
      );
      return { outcome: node.certificate_id === match.certificateId ? "unchanged" : "assigned", certificateId: match.certificateId };
    }
    if (match.kind === "ambiguous") {
      return { outcome: "ambiguous" };
    }
    return { outcome: "missing" };
  };

  const clearCertificatesForVisibleNodes = async () => {
    const lastNodes = getLastNodes();
    const visible = getVisibleNodes(lastNodes);
    if (visible.length === 0) throw new Error("no visible remote nodes");
    let cleared = 0;
    const visibleIndexes = new Set(visible.map((node) => lastNodes.indexOf(node)).filter((index) => index >= 0));
    await syncRegistry((nodes) =>
      nodes.map((node, index) => {
        if (!visibleIndexes.has(index) || !node.certificate_id) return node;
        cleared += 1;
        return { ...node, certificate_id: "" };
      }),
    );
    return { cleared, total: visible.length };
  };

  const clearCertificateForNodeIndex = async (nodeIndex) => {
    const lastNodes = getLastNodes();
    const node = lastNodes[nodeIndex];
    if (!node) throw new Error("remote node no longer exists");
    if (!node.certificate_id) return { outcome: "empty" };
    await syncRegistry((nodes) =>
      nodes.map((entry, index) => (index === nodeIndex ? { ...entry, certificate_id: "" } : entry)),
    );
    return { outcome: "cleared" };
  };

  const resolveCertificateForNodeIndex = async (nodeIndex) => {
    const lastNodes = getLastNodes();
    const node = lastNodes[nodeIndex];
    if (!node) throw new Error("remote node no longer exists");
    const status = certificateStatusFor(node).tone;
    if (status === "aligned") return { outcome: "aligned" };
    if (status === "stale") return clearCertificateForNodeIndex(nodeIndex);
    return assignCertificateForNodeIndex(nodeIndex);
  };

  const renderCertificateHealth = (nodes) => {
    if (!certificateHealthHost) return;
    const counts = ["aligned", "available", "ambiguous", "stale", "missing"]
      .map((tone) => ({ tone, count: nodes.filter((node) => certificateStatusFor(node).tone === tone).length }))
      .filter((entry) => entry.count > 0 || entry.tone === "missing");
    certificateHealthHost.innerHTML = counts.map((entry) => `
      <button class="remote-certificate-health__metric" data-certificate-focus="${entry.tone}" data-active="${(certificateFilterSelect?.value || "all") === entry.tone ? "true" : "false"}">
        <strong>${entry.tone}</strong>
        <span>${entry.count} node(s)</span>
      </button>
    `).join("");
    if (!certificateBulkActionButton) return;
    certificateBulkActionButton.textContent = certificateFilterSelect?.value === "missing" || certificateFilterSelect?.value === "available"
      ? "Assign certificates for visible state"
      : certificateFilterSelect?.value === "stale"
        ? "Clear stale certificates"
        : "Resolve visible certificate state";
  };

  const runFocusedCertificateAction = async () => {
    if (certificateFilterSelect?.value === "missing" || certificateFilterSelect?.value === "available") {
      const result = await assignCertificatesForVisibleNodes();
      showCompletion(`Assigned ${result.assigned} certificate binding(s); kept ${result.unchanged}; ${result.missing} missing; ${result.ambiguous} ambiguous across ${result.total} visible nodes.`);
      return true;
    }
    if (certificateFilterSelect?.value === "stale") {
      const result = await clearCertificatesForVisibleNodes();
      showCompletion(`Cleared ${result.cleared} certificate binding(s) across ${result.total} visible nodes.`);
      return true;
    }
    showCompletion("Focus missing, available, or stale certificate state to unlock a direct bulk action.");
    return true;
  };

  const bindEvents = () => {
    certificateHealthHost?.addEventListener("click", (event) => {
      const target = event.target.closest("[data-certificate-focus]");
      if (!target || !certificateFilterSelect) return;
      certificateFilterSelect.value = certificateFilterSelect.value === target.dataset.certificateFocus ? "all" : target.dataset.certificateFocus;
      rerender();
      showCompletion(`Focused certificate state ${certificateFilterSelect.value}.`);
    });
  };

  return {
    assignCertificateForNodeIndex,
    assignCertificatesForVisibleNodes,
    bindEvents,
    certificateStatusFor,
    clearCertificateForNodeIndex,
    clearCertificatesForVisibleNodes,
    renderCertificateHealth,
    resolveCertificateForNodeIndex,
    runFocusedCertificateAction,
  };
}
