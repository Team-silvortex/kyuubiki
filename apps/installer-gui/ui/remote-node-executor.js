export function createRemoteNodeExecutor({
  applyRemoteNodeToForm,
  currentRemoteAgentPayload,
  currentRemoteBootstrapPayload,
  invoke,
  invokeGuardedMutation,
  persistNodeState,
  runAction,
  stamp,
  workflowSnapshotFor,
}) {
  const validateMeshNode = (node) => {
    if ((node.control_mode || "orchestrated") !== "offline_mesh") {
      throw new Error("mesh preflight requires offline_mesh nodes");
    }
    if (!node.cluster_id?.trim()) {
      throw new Error("mesh preflight requires cluster_id");
    }
    if (!Array.isArray(node.peer_endpoints) || node.peer_endpoints.length === 0) {
      throw new Error("mesh preflight requires at least one peer endpoint");
    }
    return `cluster ${node.cluster_id} · peers ${node.peer_endpoints.length}`;
  };

  const runNodeAction = async (name, node, index, actionKind, runner, onSuccess) => {
    try {
      const result = await runAction(name, runner);
      const successPatch = onSuccess(result);
      await persistNodeState(index, {
        ...successPatch,
        last_action: actionKind,
        last_action_unix_ms: stamp(),
        workflowSnapshot: workflowSnapshotFor(
          node,
          actionKind,
          "succeeded",
          successPatch.last_probe_summary || (typeof result === "string" ? result : `${actionKind} succeeded`),
        ),
      });
      return result;
    } catch (error) {
      await persistNodeState(index, {
        last_probe_status: "failed",
        last_probe_summary: error.message || String(error),
        last_probe_unix_ms: stamp(),
        last_action: `${actionKind}_failed`,
        last_action_unix_ms: stamp(),
        workflowSnapshot: workflowSnapshotFor(node, actionKind, "failed", error.message || String(error)),
      });
      throw error;
    }
  };

  const executeNodeAction = async (nodeIndex, actionKind) => {
    const registry = await invoke("remote_node_registry");
    const node = registry.nodes?.[nodeIndex];
    if (!node) throw new Error("remote node no longer exists");
    applyRemoteNodeToForm(node);
    if (actionKind === "probe") {
      return runNodeAction("probe-remote-node-card", node, nodeIndex, "probe", () =>
        invokeGuardedMutation("probe_remote_node", {
          remoteBootstrap: currentRemoteBootstrapPayload(),
        }),
        (result) => ({
          last_probe_status: "ok",
          last_probe_summary: result,
          last_probe_unix_ms: stamp(),
        }),
      );
    }
    if (actionKind === "bootstrap") {
      return runNodeAction("remote-bootstrap-card", node, nodeIndex, "bootstrap", () =>
        invokeGuardedMutation("remote_bootstrap", {
          remoteBootstrap: currentRemoteBootstrapPayload(),
        }),
        () => ({}),
      );
    }
    if (actionKind === "start") {
      return runNodeAction("remote-start-agent-card", node, nodeIndex, "start_agent", () =>
        invokeGuardedMutation("remote_start_agent", {
          remoteAgent: currentRemoteAgentPayload(),
        }),
        (result) => ({
          last_probe_summary: typeof result === "string" ? result : "remote solver agent started",
        }),
      );
    }
    if (actionKind === "mesh-preflight") {
      const meshSummary = validateMeshNode(node);
      return runNodeAction("remote-mesh-preflight-card", node, nodeIndex, "mesh_preflight", async () => {
        const result = await invokeGuardedMutation("probe_remote_node", {
          remoteBootstrap: currentRemoteBootstrapPayload(),
        });
        return `${meshSummary} · ${result}`;
      }, (result) => ({
        last_probe_status: "ok",
        last_probe_summary: `mesh preflight ok · ${result}`,
        last_probe_unix_ms: stamp(),
      }));
    }
    return null;
  };

  return {
    executeNodeAction,
    runNodeAction,
    validateMeshNode,
  };
}
