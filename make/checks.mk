.PHONY: check-doc-book sync-doc-book-version check-toolchains check-elixir-self-host check-commercial-readiness check-install-update-disk-hygiene
.PHONY: check-make-modules check-module-topology check-module-function-matrix build-module-topology-report check-native-script-audit
.PHONY: check-language-packs check-ui-automation-contract check-gui-runtime-capability-contract check-version-line
.PHONY: check-workflow-dataset-contract check-material-score-contract check-materialization-plan-contract check-material-study-execution-plan-contract check-material-exploration-chain-contract check-material-research-bundle-contract check-material-study-sdk-examples check-operator-task-ir-contract check-operator-package-dynamic-smoke-contract
.PHONY: build-operator-qualification-readiness
.PHONY: capture-line-field-qualification-provenance capture-line-field-qualification-release-evidence
.PHONY: check-line-field-closed-form-baseline check-line-field-qualification-release-evidence
.PHONY: check-operator-reliability-rules check-operator-reliability-schemas check-operator-validation verify-operator-validation
.PHONY: capture-material-research-example check-material-research-example verify-material-research-example
.PHONY: build-material-research-bundle check-material-research-bundle verify-material-research-bundle material-research-bundle-index
.PHONY: remote-material-research-example remote-material-research-summary
.PHONY: check-operator-reliability audit-rust-lines audit-project-organization
.PHONY: audit-dependencies fuzz-smoke architecture-check verify

check-doc-book:
	@node ./scripts/check-doc-book.mjs

sync-doc-book-version:
	@node ./scripts/sync-doc-book-version.mjs

check-toolchains:
	@node ./scripts/check-toolchain-contract.mjs

check-elixir-self-host:
	@node ./scripts/check-elixir-self-host.mjs

check-commercial-readiness:
	@node ./scripts/validate-commercial-readiness.mjs

check-install-update-disk-hygiene:
	@node ./scripts/check-install-update-disk-hygiene.mjs --self-test
	@node ./scripts/check-install-update-disk-hygiene.mjs

check-make-modules:
	@node ./scripts/check-make-modules.mjs

check-module-topology:
	@node ./scripts/check-module-topology.mjs --self-test
	@node ./scripts/check-module-topology.mjs

check-module-function-matrix:
	@node ./scripts/check-module-function-matrix.mjs --self-test
	@node ./scripts/check-module-function-matrix.mjs

build-module-topology-report:
	@node ./scripts/build-module-topology-report.mjs --out-dir $${OUT_DIR:-tmp/module-topology}

check-native-script-audit:
	@$(ENTRYPOINT) native-script-audit --self-test

check-language-packs:
	@node ./scripts/validate-language-packs.mjs

check-ui-automation-contract:
	@node ./scripts/check-ui-automation-contract.mjs --self-test
	@node ./scripts/check-ui-automation-contract.mjs

check-gui-runtime-capability-contract:
	@node ./scripts/check-gui-runtime-capability-contract.mjs --self-test
	@node ./scripts/check-gui-runtime-capability-contract.mjs

check-version-line:
	@node ./scripts/create-release-snapshot.mjs --self-test
	@node ./scripts/audit-version-line.mjs --self-test
	@node ./scripts/audit-version-line.mjs

check-workflow-dataset-contract:
	@node ./scripts/check-workflow-dataset-contract.mjs --self-test
	@node ./scripts/check-workflow-dataset-contract.mjs

check-material-score-contract:
	@node ./scripts/validate-material-score-contract.mjs

check-materialization-plan-contract:
	@node ./scripts/check-materialization-plan-contract.mjs --self-test
	@node ./scripts/check-materialization-plan-contract.mjs

check-material-study-execution-plan-contract:
	@node ./scripts/check-material-study-execution-plan-contract.mjs --self-test
	@node ./scripts/check-material-study-execution-plan-contract.mjs

check-material-exploration-chain-contract:
	@node ./scripts/check-material-exploration-chain-contract.mjs --self-test
	@node ./scripts/check-material-exploration-chain-contract.mjs

check-material-research-bundle-contract:
	@node ./scripts/check-material-research-bundle-contract.mjs --self-test
	@node ./scripts/check-material-research-bundle-contract.mjs

check-material-study-sdk-examples:
	@node ./scripts/check-material-study-sdk-examples.mjs --self-test
	@node ./scripts/check-material-study-sdk-examples.mjs

check-operator-task-ir-contract:
	@node ./scripts/check-operator-task-ir-contract.mjs --self-test
	@node ./scripts/check-operator-task-ir-contract.mjs

check-operator-package-dynamic-smoke-contract:
	@node ./scripts/check-operator-package-dynamic-smoke.mjs --self-test

check-operator-reliability-rules:
	@node ./scripts/test-operator-reliability-rules.mjs

check-operator-reliability-schemas:
	@node ./scripts/check-operator-reliability-schemas.mjs --self-test
	@node ./scripts/check-operator-reliability-schemas.mjs

check-operator-validation:
	@node ./scripts/check-operator-validation.mjs --self-test
	@node ./scripts/check-operator-validation.mjs

verify-operator-validation:
	@node ./scripts/check-operator-validation.mjs --self-test
	@node ./scripts/check-operator-validation.mjs --execute

build-operator-qualification-readiness:
	@node ./scripts/check-operator-qualification-readiness.mjs --self-test
	@node ./scripts/build-operator-qualification-readiness.mjs --out $${OUT:-tmp/operator-qualification-readiness.json}
	@node ./scripts/check-operator-qualification-readiness.mjs --in $${OUT:-tmp/operator-qualification-readiness.json}

capture-line-field-qualification-provenance:
	@node ./scripts/capture-line-field-qualification-provenance.mjs --out $${OUT:-tmp/line-field-qualification-provenance.json}

capture-line-field-qualification-release-evidence:
	@node ./scripts/capture-line-field-qualification-release-evidence.mjs --out $${OUT:-tmp/line-field-qualification-release-evidence.json}

check-line-field-closed-form-baseline:
	@node ./scripts/check-line-field-closed-form-baseline.mjs

check-line-field-qualification-release-evidence:
	@node ./scripts/check-line-field-qualification-release-evidence.mjs --in $${IN:-tmp/line-field-qualification-release-evidence.json}

capture-material-research-example:
	@node ./scripts/capture-material-research-example.mjs --out $${OUT:-tmp/material-research-example.json}

check-material-research-example:
	@node ./scripts/check-material-research-example.mjs --in $${IN:-tmp/material-research-example.json}

verify-material-research-example:
	@$(MAKE) capture-material-research-example OUT=$${OUT:-tmp/material-research-example.json}
	@$(MAKE) check-material-research-example IN=$${OUT:-tmp/material-research-example.json}

build-material-research-bundle:
	@node ./scripts/build-material-research-bundle.mjs --study $${STUDY:-heat-spreader} --out $${OUT:-tmp/material-research-bundle.json}

check-material-research-bundle:
	@node ./scripts/check-material-research-bundle.mjs --self-test
	@node ./scripts/check-material-research-bundle.mjs --in $${IN:-tmp/material-research-bundle.json}

verify-material-research-bundle:
	@$(MAKE) build-material-research-bundle OUT=$${OUT:-tmp/material-research-bundle.json}
	@$(MAKE) check-material-research-bundle IN=$${OUT:-tmp/material-research-bundle.json}

material-research-bundle-index:
	@node ./scripts/build-material-research-bundle-index.mjs --self-test
	@node ./scripts/build-material-research-bundle-index.mjs --ensure-bundles --out-dir $${OUT_DIR:-tmp/material-research-bundles}

remote-material-research-example:
	@node ./scripts/run-remote-material-research-example.mjs --profile $${PROFILE:-100k} --matrix $${MATRIX:-compound-core} --repeat $${REPEAT:-1}

remote-material-research-summary:
	@node ./scripts/build-remote-material-benchmark-summary.mjs --self-test
	@node ./scripts/build-remote-material-benchmark-summary.mjs
	@node ./scripts/check-remote-material-preconditioner-health.mjs --self-test
	@node ./scripts/check-remote-material-preconditioner-health.mjs
	@node ./scripts/check-remote-material-stage-health.mjs --self-test
	@node ./scripts/check-remote-material-stage-health.mjs

check-operator-reliability: check-operator-reliability-rules check-operator-reliability-schemas check-line-field-closed-form-baseline
	@node ./scripts/check-operator-reliability.mjs

audit-rust-lines:
	@node ./scripts/audit-rust-line-counts.mjs --max $${MAX_LINES:-600}

audit-project-organization:
	@node ./scripts/audit-project-organization.mjs --self-test
	@node ./scripts/audit-project-organization.mjs

audit-dependencies:
	@node ./scripts/audit-dependencies.mjs --self-test
	@node ./scripts/audit-dependencies.mjs

fuzz-smoke:
	@cd workers/rust && cargo test -p kyuubiki-engine workflow_security_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-protocol operator_task_ir_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-protocol rpc_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-operator-sdk operator_package_manifest_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer installer_update_catalog_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer remote_artifact_manifest_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer credential_storage_contract_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer remote_host_trust_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer remote_ssh_fixture_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer remote_deployment_metadata_fuzz_smoke -- --nocapture

architecture-check:
	@$(MAKE) audit-project-organization
	@$(MAKE) check-make-modules
	@$(MAKE) check-module-topology
	@$(MAKE) check-module-function-matrix
	@$(MAKE) check-native-script-audit
	@$(MAKE) check-version-line
	@$(MAKE) check-workflow-dataset-contract
	@$(MAKE) check-material-score-contract
	@$(MAKE) check-materialization-plan-contract
	@$(MAKE) check-material-study-execution-plan-contract
	@$(MAKE) check-material-exploration-chain-contract
	@$(MAKE) check-material-research-bundle-contract
	@$(MAKE) check-material-study-sdk-examples
	@$(MAKE) check-operator-task-ir-contract
	@$(MAKE) check-operator-package-dynamic-smoke-contract
	@$(MAKE) check-operator-reliability
	@$(MAKE) check-operator-validation
	@$(MAKE) check-commercial-readiness
	@$(MAKE) check-install-update-disk-hygiene
	@$(MAKE) check-ui-automation-contract
	@$(MAKE) check-gui-runtime-capability-contract
	@$(MAKE) audit-dependencies
	@$(MAKE) operator-package-preflight
	@$(MAKE) operator-package-dynamic-smoke
	@jq empty docs/book-manifest.json
	@node ./scripts/validate-minimal-industrial-closure.mjs
	@cd apps/web && mix test test/kyuubiki_web/api/operator_task_api_test.exs test/kyuubiki_web/orchestra/operator_task_executor_test.exs test/kyuubiki_web/orchestra/operator_task_ir_test.exs
	@cd workers/rust && cargo test -p kyuubiki-cli operator_task_ir_rpc
	@cd workers/rust && cargo test -p kyuubiki-cli --test operator_task_live

verify:
	@$(MAKE) check-toolchains
	@$(MAKE) check-elixir-self-host
	@$(MAKE) check-native-script-audit
	@$(MAKE) check-language-packs
	@$(MAKE) check-version-line
	@$(MAKE) check-operator-reliability
	@$(MAKE) check-commercial-readiness
	@$(MAKE) check-install-update-disk-hygiene
	@$(MAKE) check-ui-automation-contract
	@cd apps/web && mix format --check-formatted && mix test
	@cd workers/rust && cargo fmt --check && cargo test
	@$(MAKE) audit-rust-lines
	@$(MAKE) audit-project-organization
	@$(MAKE) audit-dependencies
	@$(MAKE) operator-package-preflight
	@$(ENTRYPOINT) sdk-smoke
	@cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $${PROFILE:-10k} --repeat $${REPEAT:-3} --baseline-compare benchmarks/$${PROFILE:-10k}-baseline.json --fail-on-median-regression-pct $${BENCHMARK_MEDIAN_THRESHOLD:-25} --fail-on-rss-regression-pct $${BENCHMARK_RSS_THRESHOLD:-20} --min-baseline-median-ms $${BENCHMARK_MIN_BASELINE_MS:-5.0}
	@node --test apps/web/playground/test/fem.test.mjs
