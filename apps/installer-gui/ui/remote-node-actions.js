function focusField(fieldId) {
  const field = document.getElementById(fieldId);
  if (!field) return false;
  field.scrollIntoView({ block: "center", behavior: "smooth" });
  field.focus?.();
  if (typeof field.select === "function" && ("value" in field)) field.select();
  return true;
}

export function createRemoteNodeActionCoordinator({
  applyRemoteNodeToForm,
  executeNodeAction,
  getLastNodes,
  resolveCertificateForNodeIndex,
  showCompletion,
}) {
  const runRecommendedAction = async (actionKind, nodeIndex) => {
    const node = getLastNodes()[nodeIndex];
    if (!node) throw new Error("remote node no longer exists");
    applyRemoteNodeToForm(node);
    if (actionKind === "focus-cluster") {
      focusField("remote-cluster-id");
      showCompletion(`Focused mesh cluster id for ${node.label || node.target_host}.`);
      return;
    }
    if (actionKind === "focus-peer-endpoints") {
      focusField("remote-peer-endpoints");
      showCompletion(`Focused peer endpoints for ${node.label || node.target_host}.`);
      return;
    }
    if (actionKind === "focus-certificate") {
      const focused = focusField("remote-certificate-id") || focusField("certificate-revoke-id");
      showCompletion(
        focused
          ? `Focused certificate controls for ${node.label || node.target_host}.`
          : `Loaded certificate controls for ${node.label || node.target_host}.`,
      );
      return;
    }
    if (actionKind === "resolve-certificate") {
      const result = await resolveCertificateForNodeIndex(nodeIndex);
      if (result.outcome === "assigned" || result.outcome === "unchanged") {
        showCompletion(`Resolved certificate state for ${node.label || node.target_host} with ${result.certificateId}.`);
      } else if (result.outcome === "cleared") {
        showCompletion(`Cleared stale certificate binding for ${node.label || node.target_host}.`);
      } else if (result.outcome === "aligned") {
        showCompletion(`Certificate state already aligned for ${node.label || node.target_host}.`);
      } else if (result.outcome === "ambiguous") {
        showCompletion(`Multiple active certificates match ${node.label || node.target_host}; pick one from inventory.`);
      } else {
        showCompletion(`No active certificate match available for ${node.label || node.target_host}.`);
      }
      return;
    }
    if (actionKind === "inspect") {
      showCompletion(`Loaded ${node.label || node.target_host} into the form for manual review.`);
      return;
    }
    await executeNodeAction(nodeIndex, actionKind);
    const actionLabel = actionKind === "mesh-preflight"
      ? "mesh preflight"
      : actionKind === "start"
        ? "start agent"
        : actionKind;
    showCompletion(`Completed ${actionLabel} for ${node.label || node.target_host}.`);
  };

  return {
    focusField,
    runRecommendedAction,
  };
}
