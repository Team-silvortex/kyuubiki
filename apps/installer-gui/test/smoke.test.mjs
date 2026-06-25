import test from "node:test";
import assert from "node:assert/strict";
import {
  assertMatches,
  createFixtureReader,
  createFixtureRoot,
} from "../../desktop-shared/test/smoke-test-helpers.mjs";

const ROOT = createFixtureRoot(import.meta.url);
const read = createFixtureReader(ROOT);

test("installer shell defines a least-privilege main-window capability", () => {
  const tauriConfig = JSON.parse(read("src-tauri/tauri.conf.json"));
  const capability = JSON.parse(read("src-tauri/capabilities/main.json"));
  const permissions = read("src-tauri/permissions/installer.toml");

  assert.equal(tauriConfig.app.windows[0]?.label, "main");
  assert.equal(capability.identifier, "main");
  assert.deepEqual(capability.windows, ["main"]);
  assert.ok(capability.permissions.includes("core:default"));
  assert.ok(capability.permissions.includes("allow-guarded-mutation-action"));
  assert.ok(capability.permissions.includes("allow-service-status"));
  assert.ok(capability.permissions.includes("allow-read-env-file"));
  assert.ok(capability.permissions.includes("allow-certificate-authority-policy"));
  assert.ok(capability.permissions.includes("allow-regression-gate-report"));
  assert.match(permissions, /identifier = "allow-guarded-mutation-action"/);
  assert.match(permissions, /commands\.allow = \["guarded_mutation_action"\]/);
  assert.match(permissions, /identifier = "allow-certificate-authority-policy"/);
  assert.match(permissions, /commands\.allow = \["certificate_authority_policy"\]/);
  assert.match(permissions, /identifier = "allow-regression-gate-report"/);
  assert.match(permissions, /commands\.allow = \["regression_gate_report"\]/);
});

test("installer shell exposes setup, services, remote, and release surfaces", () => {
  const html = read("ui/index.html");
  const remotePanel = read("ui/remote-panel.js");

  assertMatches(html, [
    /data-tab="setup"/,
    /data-tab="services"/,
    /data-tab="remote"/,
    /data-tab="release"/,
    /Run doctor/,
    /Bootstrap workspace/,
    /id="remote-panel-root"/,
    /id="regression-gate-title"/,
    /id="regression-gate-status"/,
    /id="regression-gate-warning-count"/,
    /id="regression-gate-failing-count"/,
    /id="regression-gate-catalog-path"/,
    /id="regression-gate-summary"/,
    /id="regression-gate-reasons"/,
    /placeholder="dist\/\{platform\}"/,
  ]);

  assertMatches(remotePanel, [
    /id="remote-control-mode"/,
    /id="remote-certificate-id"/,
    /id="remote-node-mode-filter"/,
    /id="remote-node-certificate-filter"/,
    /id="remote-certificate-bulk-action"/,
    /id="remote-certificate-health"/,
    /id="remote-mesh-health"/,
    /id="remote-mesh-issues"/,
    /id="remote-mesh-rollout-failures"/,
    /id="remote-mesh-clusters"/,
    /id="remote-node-timeline"/,
    /id="certificate-policy-storage-root"/,
    /id="certificate-ca-fingerprint"/,
    /id="certificate-issue-agent-id"/,
    /id="certificate-revoke-id"/,
    /id="certificate-inventory-list"/,
    /mesh-preflight/,
    /mesh-rollout/,
    /assign-certificates/,
    /clear-certificates/,
    /<option value="ambiguous">Ambiguous<\/option>/,
    /offline_mesh/,
  ]);
});

test("installer shell wires core install and runtime actions", () => {
  const js = read("ui/app.js");
  const certificatePanel = read("ui/certificate-panel.js");
  const regressionGatePanel = read("ui/regression-gate-panel.js");
  const remoteNodeCertificates = read("ui/remote-node-certificates.js");
  const remoteNodeActions = read("ui/remote-node-actions.js");
  const remoteNodeBulkActions = read("ui/remote-node-bulk-actions.js");
  const remoteNodeExecutor = read("ui/remote-node-executor.js");
  const remoteNodeMesh = read("ui/remote-node-mesh.js");
  const remoteNodeRenderer = read("ui/remote-node-renderer.js");
  const remoteNodeTimeline = read("ui/remote-node-timeline.js");
  const remotePanel = read("ui/remote-panel.js");
  const remoteNodeStyles = read("ui/styles/installer-remote-nodes.css");
  const remoteTimelineStyles = read("ui/styles/installer-remote-timeline.css");
  const bridge = read("ui/shared/tauri-bridge.js");
  const platform = read("ui/shared/platform.js");

  assertMatches(js, [
    /doctor_report/,
    /guarded_mutation_action/,
    /invokeGuardedMutation/,
    /watchDesktopLanguagePreference/,
    /populateDesktopPlatformSelect/,
    /syncDesktopReleaseTargetInput/,
    /certificate_authority_policy/,
    /initialize_certificate_authority/,
    /issue_node_certificate/,
    /revoke_node_certificate/,
    /mountRemotePanel/,
  ]);
  assert.match(certificatePanel, /currentCertificatePolicyPayload/);
  assert.match(read("ui/installer-workflows.js"), /appendRemoteNodeWorkflowSnapshot/);
  assert.match(read("ui/installer-workflows.js"), /workflow_snapshots/);
  assert.match(certificatePanel, /getActiveCertificates/);
  assert.match(certificatePanel, /hydrateCertificateAuthority/);
  assert.match(certificatePanel, /use-for-remote-agent/);
  assert.match(certificatePanel, /selectedOptions/);
  assert.match(js, /getActiveCertificates/);
  assert.match(js, /mountRemoteNodePanel\(\{/);
  assert.match(js, /getActiveCertificates,\s*showCompletion/);
  assert.match(regressionGatePanel, /renderRegressionGateReport/);
  assert.match(remotePanel, /mountRemotePanel/);
  assert.match(remoteNodeActions, /createRemoteNodeActionCoordinator/);
  assert.match(remoteNodeActions, /runRecommendedAction/);
  assert.match(remoteNodeActions, /focusField/);
  assert.match(remoteNodeActions, /resolveCertificateForNodeIndex/);
  assert.match(remoteNodeBulkActions, /createRemoteNodeBulkActionCoordinator/);
  assert.match(remoteNodeBulkActions, /runBulkAction/);
  assert.match(remoteNodeBulkActions, /mesh-rollout/);
  assert.match(remoteNodeBulkActions, /assign-certificates/);
  assert.match(remoteNodeExecutor, /createRemoteNodeExecutor/);
  assert.match(remoteNodeExecutor, /executeNodeAction/);
  assert.match(remoteNodeExecutor, /validateMeshNode/);
  assert.match(remoteNodeExecutor, /workflowSnapshotFor/);
  assert.match(read("ui/remote-node-panel.js"), /createRemoteNodeCertificateController/);
  assert.match(read("ui/remote-node-panel.js"), /createRemoteNodeActionCoordinator/);
  assert.match(read("ui/remote-node-panel.js"), /createRemoteNodeBulkActionCoordinator/);
  assert.match(read("ui/remote-node-panel.js"), /createRemoteNodeExecutor/);
  assert.match(read("ui/remote-node-panel.js"), /createRemoteNodeMeshController/);
  assert.match(read("ui/remote-node-panel.js"), /createRemoteNodeTimelineController/);
  assert.match(read("ui/remote-node-panel.js"), /renderRemoteNodeGroups/);
  assert.match(read("ui/remote-node-panel.js"), /workflowSnapshotFor/);
  assert.match(read("ui/remote-node-panel.js"), /actionCoordinator\.runRecommendedAction/);
  assert.match(read("ui/remote-node-panel.js"), /bulkActionCoordinator\.runBulkAction/);
  assert.match(read("ui/remote-node-panel.js"), /executor\.executeNodeAction/);
  assert.match(remoteNodeCertificates, /assignCertificatesForVisibleNodes/);
  assert.match(remoteNodeCertificates, /assignCertificateForNodeIndex/);
  assert.match(remoteNodeCertificates, /clearCertificatesForVisibleNodes/);
  assert.match(remoteNodeCertificates, /resolveCertificateForNodeIndex/);
  assert.match(remoteNodeCertificates, /certificateStatusFor/);
  assert.match(remoteNodeCertificates, /certificateFilterSelect/);
  assert.match(remoteNodeCertificates, /renderCertificateHealth/);
  assert.match(remoteNodeCertificates, /data-certificate-focus/);
  assert.match(remotePanel, /certificate-focus-action/);
  assert.match(remoteNodeCertificates, /Assign certificates for visible state/);
  assert.match(remoteNodeMesh, /renderMeshDiagnostics/);
  assert.match(remoteNodeMesh, /runVisibleMeshRollout/);
  assert.match(remoteNodeMesh, /recordStageFailure/);
  assert.match(remoteNodeMesh, /renderRolloutFailures/);
  assert.match(remoteNodeMesh, /retryFailedNodes/);
  assert.match(remoteNodeMesh, /retry-failures/);
  assert.match(remoteNodeMesh, /data-remote-cluster-action/);
  assert.match(remoteNodeMesh, /focus-cluster/);
  assert.match(remoteNodeMesh, /preflight-missing/);
  assert.match(remoteNodeRenderer, /dataset\.certificateStatus/);
  assert.match(remoteNodeRenderer, /remoteNodeCardIndex/);
  assert.match(remoteNodeRenderer, /remote-node-card__certificate-pill/);
  assert.match(remoteNodeRenderer, /groupRemoteNodes/);
  assert.match(remoteNodeTimeline, /selectNode/);
  assert.match(remoteNodeTimeline, /selectEntry/);
  assert.match(remoteNodeTimeline, /keepNodeContext/);
  assert.match(remoteNodeTimeline, /workflow_snapshots/);
  assert.match(remoteNodeTimeline, /remote-node-timeline__details/);
  assert.match(remoteNodeTimeline, /remote-node-timeline__summary-grid/);
  assert.match(remoteNodeTimeline, /Additional fields/);
  assert.match(remoteNodeTimeline, /renderSemanticBadges/);
  assert.match(remoteNodeTimeline, /workflowKindMeta/);
  assert.match(remoteNodeTimeline, /statusMeta/);
  assert.match(remoteNodeTimeline, /recommendedActions/);
  assert.match(remoteNodeTimeline, /Recommended actions/);
  assert.match(remoteNodeTimeline, /data-recommended-action/);
  assert.match(remoteNodeTimeline, /remote-node-timeline__recommendation-action/);
  assert.match(remoteNodeTimeline, /focus-cluster/);
  assert.match(remoteNodeTimeline, /focus-peer-endpoints/);
  assert.match(remoteNodeTimeline, /focus-certificate/);
  assert.match(remoteNodeTimeline, /data-latest/);
  assert.match(remoteNodeTimeline, /remote-node-timeline__follow-state/);
  assert.match(remoteNodeTimeline, /data-stage-tone/);
  assert.match(remoteNodeTimeline, /data-timeline-entry-index/);
  assert.match(read("ui/styles.css"), /installer-remote-nodes\.css/);
  assert.match(read("ui/styles.css"), /installer-remote-timeline\.css/);
  assert.match(remoteNodeStyles, /data-certificate-status="aligned"/);
  assert.match(remoteNodeStyles, /remote-certificate-health__metric/);
  assert.match(remoteNodeStyles, /remote-node-card__certificate-pill/);
  assert.match(remoteNodeStyles, /remote-node-grid/);
  assert.match(remoteNodeStyles, /remote-mesh-cluster/);
  assert.match(remoteTimelineStyles, /remote-mesh-rollout-failures__item/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__item/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__details/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__body/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__follow-state/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__badges/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__badge/);
  assert.match(remoteTimelineStyles, /data-stage-tone="preflight"/);
  assert.match(remoteTimelineStyles, /data-latest="true"/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__summary-slot/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__recommendation-item/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__recommendation-action/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__recommendations/);
  assert.match(remoteTimelineStyles, /remote-node-timeline__detail-section-title/);
  assert.match(remoteNodeCertificates, /certificate auto-match/);
  assert.match(bridge, /export async function invokeTauri/);
  assert.match(bridge, /export function applyDesktopState/);
  assert.doesNotMatch(bridge, /desktop-shared\/ui\/tauri-bridge\.js/);
  assert.match(platform, /export function normalizeDesktopPlatform/);
  assert.doesNotMatch(platform, /desktop-shared\/ui\/platform\.js/);
  assert.match(read("ui\/installer-startup.js"), /regression_gate_report/);
});

test("tauri backend exposes installer command surface", () => {
  const rust = read("src-tauri/src/main.rs");

  assertMatches(rust, [
    /doctor_report/,
    /guarded_mutation_action/,
    /service_status/,
    /start_log_stream/,
    /read_env_file/,
    /certificate_authority_policy/,
    /initialize_certificate_authority/,
    /issue_node_certificate/,
    /revoke_node_certificate/,
  ]);
  assert.match(rust, /remote_node_registry/);
  assert.match(read("src-tauri/src/remote_nodes.rs"), /RemoteNodeWorkflowSnapshot/);
  assert.match(read("src-tauri/src/remote_nodes.rs"), /validate_workflow_snapshot/);
  assert.match(read("src-tauri/src/remote_nodes.rs"), /workflow_snapshots/);
  assert.match(rust, /regression_gate_report/);
});
