pub(crate) fn print_help() {
    println!(
        "Kyuubiki native script runner\n\n\
Native commands:\n  \
status/start/stop/restart/export-db/hot-status\n  \
doctor validate-env install package cross-platform-audit\n  \
check-elixir-self-host\n  \
operator-package-preflight\n  \
operator-package-dynamic-smoke\n  \
  check-operator-package-dynamic-smoke\n  \
  check-operator-package-dynamic-smoke-contract\n  \
  check-operator-reliability-rules\n  \
  check-operator-reliability-schemas\n  \
  check-operator-validation\n  \
  check-line-field-closed-form-baseline\n  \
  capture-line-field-qualification-provenance\n  \
  check-line-field-qualification-release-evidence\n  \
  build-operator-qualification-readiness\n  \
  check-operator-qualification-readiness\n  \
project macro build-frontend build-orchestrator build-agent\n  \
build-hub-gui build-installer-gui build-workbench-gui\n  \
package-desktop desktop-status desktop-stage desktop-build-host\n  \
desktop-release desktop-verify\n  \
desktop-linux-remote\n  \
desktop-upload-remote desktop-release-upload-remote\n  \
generate-desktop-icon-variants\n  \
lab remote-ssh-fixture\n  \
web-test rust-test rust-line-audit frontend-test headless-test\n  \
  headless-live-test headless-rust-live-test sdk-smoke workflow-preflight\n  \
  check-make-modules\n  \
  check-doc-book\n  \
  sync-doc-book-version\n  \
  check-toolchain-contract\n  \
  check-install-update-disk-hygiene\n  \
  check-module-topology\n  \
  build-module-topology-report\n  \
  check-module-function-matrix\n  \
  check-module-function-coverage-tensor\n  \
  check-module-extension-standard\n  \
  check-verification-evidence-surface\n  \
  check-central-store-contract\n  \
  check-central-database-readiness\n  \
  build-central-readiness-report\n  \
  check-central-readiness-report\n  \
  validate-commercial-readiness\n  \
  validate-minimal-industrial-closure\n  \
  check-contracts-runtime-api-surface\n  \
  validate-language-packs\n  \
  check-ui-automation-contract\n  \
  check-gui-runtime-capability-contract\n  \
  check-workflow-dataset-contract\n  \
  check-materialization-plan-contract\n  \
  check-material-study-execution-plan-contract\n  \
  check-material-exploration-chain-contract\n  \
  check-material-research-bundle-contract\n  \
  check-operator-task-ir-contract\n  \
  validate-material-score-contract\n  \
  audit-local-paths\n  \
  audit-project-organization\n  \
  audit-dependencies\n  \
  frontend-file-lines frontend-storage-security\n  \
  benchmark-profile-remote\n  \
  benchmark-profile-plan\n  \
  direct-mesh-benchmark-container\n  \
  direct-mesh-benchmark-regression\n  \
  standard-benchmark-regression\n  \
  workflow-catalog-benchmark-regression\n  \
  workflow-mesh-regression-remote\n  \
  workflow-mesh-regression\n  \
  agent-capability-smoke\n  \
worker benchmark agent frontend format\n  \
hub-gui-dev installer-gui-dev workbench-gui-dev\n  \
native-script-audit\n"
    );
}
