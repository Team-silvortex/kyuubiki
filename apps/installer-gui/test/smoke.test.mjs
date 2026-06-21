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
    /id="remote-mesh-clusters"/,
    /id="certificate-policy-storage-root"/,
    /id="certificate-ca-fingerprint"/,
    /id="certificate-issue-agent-id"/,
    /id="certificate-revoke-id"/,
    /id="certificate-inventory-list"/,
    /mesh-preflight/,
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
  const remoteNodeMesh = read("ui/remote-node-mesh.js");
  const remotePanel = read("ui/remote-panel.js");
  const bridge = read("ui/shared/tauri-bridge.js");
  const platform = read("ui/shared/platform.js");

  assertMatches(js, [
    /doctor_report/,
    /guarded_mutation_action/,
    /invokeGuardedMutation/,
    /populateDesktopPlatformSelect/,
    /syncDesktopReleaseTargetInput/,
    /certificate_authority_policy/,
    /initialize_certificate_authority/,
    /issue_node_certificate/,
    /revoke_node_certificate/,
    /mountRemotePanel/,
  ]);
  assert.match(certificatePanel, /currentCertificatePolicyPayload/);
  assert.match(certificatePanel, /getActiveCertificates/);
  assert.match(certificatePanel, /hydrateCertificateAuthority/);
  assert.match(certificatePanel, /use-for-remote-agent/);
  assert.match(certificatePanel, /selectedOptions/);
  assert.match(js, /getActiveCertificates/);
  assert.match(js, /mountRemoteNodePanel\(\{/);
  assert.match(js, /getActiveCertificates,\s*showCompletion/);
  assert.match(regressionGatePanel, /renderRegressionGateReport/);
  assert.match(remotePanel, /mountRemotePanel/);
  assert.match(read("ui/remote-node-panel.js"), /createRemoteNodeCertificateController/);
  assert.match(read("ui/remote-node-panel.js"), /createRemoteNodeMeshController/);
  assert.match(remoteNodeCertificates, /assignCertificatesForVisibleNodes/);
  assert.match(remoteNodeCertificates, /clearCertificatesForVisibleNodes/);
  assert.match(remoteNodeCertificates, /certificateStatusFor/);
  assert.match(remoteNodeCertificates, /certificateFilterSelect/);
  assert.match(remoteNodeCertificates, /renderCertificateHealth/);
  assert.match(remoteNodeCertificates, /data-certificate-focus/);
  assert.match(remotePanel, /certificate-focus-action/);
  assert.match(remoteNodeCertificates, /Assign certificates for visible state/);
  assert.match(remoteNodeMesh, /renderMeshDiagnostics/);
  assert.match(remoteNodeMesh, /data-remote-cluster-action/);
  assert.match(remoteNodeMesh, /focus-cluster/);
  assert.match(remoteNodeMesh, /preflight-missing/);
  assert.match(read("ui/remote-node-panel.js"), /dataset\.certificateStatus/);
  assert.match(read("ui/remote-node-panel.js"), /remote-node-card__certificate-pill/);
  assert.match(read("ui/styles.css"), /data-certificate-status="aligned"/);
  assert.match(read("ui/styles.css"), /remote-certificate-health__metric/);
  assert.match(read("ui/styles.css"), /remote-node-card__certificate-pill/);
  assert.match(remoteNodeCertificates, /certificate auto-match/);
  assert.match(bridge, /desktop-shared\/ui\/tauri-bridge\.js/);
  assert.match(platform, /desktop-shared\/ui\/platform\.js/);
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
  assert.match(rust, /regression_gate_report/);
});
