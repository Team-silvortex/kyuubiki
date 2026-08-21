pub(crate) fn print_help() {
    println!(
        "Kyuubiki native script runner\n\n\
Native commands:\n  \
status/start/stop/restart/export-db/hot-status\n  \
doctor validate-env install package cross-platform-audit\n  \
check-elixir-self-host\n  \
audit-version-line\n  \
create-release-snapshot\n  \
operator-package-preflight\n  \
operator-package-dynamic-smoke\n  \
  check-operator-package-dynamic-smoke\n  \
  check-operator-package-dynamic-smoke-contract\n  \
  check-operator-reliability-rules\n  \
  check-operator-reliability\n  \
  check-operator-reliability-schemas\n  \
  check-operator-validation\n  \
  check-line-field-closed-form-baseline\n  \
  capture-line-field-qualification-provenance\n  \
  capture-line-field-qualification-release-evidence\n  \
  check-line-field-qualification-release-evidence\n  \
  check-beam-frame-qualification-release-evidence\n  \
  build-operator-qualification-readiness\n  \
  check-operator-qualification-readiness\n  \
  check-operator-qualification-release-records\n  \
  check-operator-qualification-review-decision\n  \
project create|inspect|validate|normalize|unpack|pack|diff\n  \
project automation-presets|automation-render|automation-run\n  \
kcore export|research-export|inspect|verify|extract\n  \
macro actions|inspect|validate|normalize|render|run\n  \
headless templates|suggest|init|inspect|validate|render|plan|run\n  \
build-frontend build-orchestrator build-agent\n  \
build-hub-gui build-installer-gui build-workbench-gui\n  \
sync-desktop-shared check-desktop-shared\n  \
package-desktop desktop-status desktop-stage desktop-build-host\n  \
desktop-install-host\n  \
desktop-packaged-smoke [macos] [--bundle-root <dir>] [--out <report>] [--verify-report <report>]\n  \
desktop-release desktop-verify desktop-runtime-payload\n  \
desktop-linux-remote\n  \
desktop-upload-remote desktop-release-upload-remote\n  \
generate-desktop-icon-variants\n  \
lab remote-ssh-fixture\n  \
  web-test rust-test rust-line-audit frontend-test headless-test\n  \
  rust-coverage\n  \
  headless-live-test headless-rust-live-test sdk-smoke workflow-preflight\n  \
  check-make-modules\n  \
  check-doc-book\n  \
  check-doc-inventory\n  \
  sync-doc-book-version\n  \
  check-moxi-handoff\n  \
  check-toolchain-contract\n  \
  check-install-update-disk-hygiene\n  \
  check-component-integrity-protocol\n  \
  build-installation-integrity-docs\n  \
  build-update-catalog\n  \
  check-module-topology\n  \
  build-module-topology-report\n  \
  check-module-function-matrix\n  \
  check-module-function-coverage-tensor\n  \
  check-test-coverage-posture\n  \
  check-module-extension-standard\n  \
  check-verification-evidence-surface\n  \
  check-benchmark-qualification\n  \
  check-central-store-contract\n  \
  check-central-database-readiness\n  \
  central-database-smoke\n  \
  remote-central-database-smoke\n  \
  build-central-readiness-report\n  \
  check-central-readiness-report\n  \
  validate-commercial-readiness\n  \
  validate-minimal-industrial-closure\n  \
  check-contracts-runtime-api-surface\n  \
  check-contracts-validation-qualification\n  \
  check-workbench-validation-qualification\n  \
  validate-language-packs\n  \
  report-full-language-pack-coverage\n  \
  plan-workbench-language-translations\n  \
  build-workbench-language-pack-catalog\n  \
  check-ui-automation-contract\n  \
  check-gui-runtime-capability-contract\n  \
  check-desktop-usability-journeys\n  \
  check-usability-release-gate\n  \
  check-runtime-recovery-fault-injection\n  \
  check-orchestra-recovery-fault-injection\n  \
  check-orchestra-workflow-operational-qualification\n  \
  check-installed-runtime-operational-qualification\n  \
  check-persistence-provenance-qualification\n  \
  check-installer-recovery-fault-injection\n  \
  check-workflow-dataset-contract\n  \
  check-workflow-metric-resolver-contract\n  \
  check-material-card-contract\n  \
  check-materialization-plan-contract\n  \
  check-material-study-execution-plan-contract\n  \
  check-material-exploration-chain-contract\n  \
  check-material-research-bundle-contract\n  \
  build-material-research-bundle\n  \
  build-material-research-bundle-index\n  \
  check-material-research-bundle\n  \
  check-material-research-bundle-index\n  \
  check-material-research-bundle-index-contract\n  \
  check-material-study-sdk-examples\n  \
  check-remote-material-preconditioner-health\n  \
  check-remote-material-stage-health\n  \
  build-remote-material-benchmark-summary\n  \
  remote-material-research-example\n  \
  capture-material-research-example\n  \
  check-material-research-example\n  \
  check-operator-task-ir-contract\n  \
  check-protocol-validation-qualification\n  \
  check-system-security-qualification\n  \
  check-agent-control-link-operational-qualification\n  \
  qualify-agent-control-link-operational-remote\n  \
  check-distributed-task-recovery-operational-qualification\n  \
  qualify-distributed-task-recovery-operational-remote\n  \
  check-agent-solver-operational-qualification\n  \
  qualify-agent-solver-operational-remote\n  \
  check-agent-update-operational-qualification\n  \
  qualify-agent-update-operational-remote\n  \
  check-runtime-payload-operational-qualification\n  \
  qualify-runtime-payload-operational-remote\n  \
  qualify-headless-sdk-operational-remote\n  \
  check-headless-sdk-validation-qualification\n  \
  check-headless-sdk-operational-qualification\n  \
  check-headless-workflow-qualification\n  \
  validate-material-score-contract\n  \
  audit-local-paths\n  \
  audit-project-organization\n  \
  audit-dependencies\n  \
  frontend-file-lines frontend-storage-security\n  \
  benchmark-profile-remote\n  \
  benchmark-profile-plan\n  \
  build-benchmark-profile-index\n  \
  build-regression-lane-catalog\n  \
  build-regression-gate-report\n  \
  build-nightly-artifact-overview\n  \
  direct-mesh-benchmark-container\n  \
  compare-direct-mesh-benchmark\n  \
  direct-mesh-benchmark-regression\n  \
  standard-benchmark-regression\n  \
  build-standard-benchmark-index\n  \
  workflow-catalog-benchmark-regression\n  \
  workflow-mesh-regression-remote\n  \
  build-workflow-mesh-regression-index\n  \
  build-workflow-mesh-regression-summary\n  \
  workflow-mesh-regression\n  \
  build-standard-benchmark-report\n  \
  compare-workflow-catalog-benchmark\n  \
  frontend-typecheck frontend-unit-test\n  \
  frontend-unit-coverage-test\n  \
  frontend-unit-headless-test frontend-unit-headless-live-test\n  \
  frontend-unit-workflow-test frontend-ui-layout-check\n  \
  frontend-workflow-search-layout-check frontend-workflow-topology-check\n  \
  frontend-workflow-benchmark\n  \
  playground-fem-node-test\n  \
  hub-gui-compile-ui\n  \
  hub-gui-smoke-node-test\n  \
  installer-gui-smoke-node-test\n  \
  workbench-gui-smoke-node-test\n  \
  integration-api-node-test\n  \
  integration-cluster-node-test\n  \
  integration-direct-mesh-node-test\n  \
  integration-desktop-gui-node-test\n  \
  check-desktop-ui-validation\n  \
  check-desktop-deployment-update-qualification\n  \
  integration-benchmark-profile-index-node-test\n  \
  integration-ui-mechanical-node-test\n  \
  integration-ui-thermal-node-test\n  \
  agent-capability-smoke\n  \
worker benchmark benchmark-release agent frontend format\n  \
hub-gui-dev installer-gui-dev workbench-gui-dev\n  \
native-script-audit\n"
    );
}
