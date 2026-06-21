export function createRemoteNodeBulkActionCoordinator({
  assignCertificatesForVisibleNodes,
  clearCertificatesForVisibleNodes,
  executeNodeAction,
  getLastNodes,
  getVisibleNodes,
  renderMeshRolloutFailures,
  runFocusedCertificateAction,
  runVisibleMeshRollout,
  showCompletion,
}) {
  const runBulkAction = async (actionKind) => {
    if (actionKind === "assign-certificates") {
      const result = await assignCertificatesForVisibleNodes();
      showCompletion(
        `Assigned ${result.assigned} certificate binding(s); kept ${result.unchanged}; ${result.missing} missing; ${result.ambiguous} ambiguous across ${result.total} visible nodes.`,
      );
      return;
    }
    if (actionKind === "clear-certificates") {
      const result = await clearCertificatesForVisibleNodes();
      showCompletion(`Cleared ${result.cleared} certificate binding(s) across ${result.total} visible nodes.`);
      return;
    }
    if (actionKind === "certificate-focus-action") {
      await runFocusedCertificateAction();
      return;
    }
    if (actionKind === "mesh-rollout") {
      const result = await runVisibleMeshRollout(getVisibleNodes(getLastNodes()));
      const failureSummary = result.failures.length === 0
        ? "no failures"
        : `failures ${result.failures.length}: ${result.failures.map((entry) => `${entry.stage} ${entry.label}`).join(", ")}`;
      showCompletion(
        `Completed mesh rollout for ${result.total} visible node(s): bootstrap ${result.bootstrapped}, preflight ${result.preflighted}, start ${result.started}; ${failureSummary}.`,
      );
      return;
    }

    const visible = getVisibleNodes(getLastNodes()).filter((node) =>
      actionKind === "mesh-preflight" ? (node.control_mode || "orchestrated") === "offline_mesh" : true,
    );
    if (visible.length === 0) throw new Error("no visible remote nodes");
    if (actionKind === "mesh-preflight") renderMeshRolloutFailures([]);
    for (const node of visible) {
      const nodeIndex = getLastNodes().indexOf(node);
      await executeNodeAction(nodeIndex, actionKind);
    }
    showCompletion(`Completed ${actionKind} for ${visible.length} visible nodes.`);
  };

  return {
    runBulkAction,
  };
}
