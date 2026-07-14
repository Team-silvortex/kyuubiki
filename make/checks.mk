.PHONY: check-doc-book sync-doc-book-version check-toolchains check-elixir-self-host check-commercial-readiness check-install-update-disk-hygiene
.PHONY: check-make-modules check-module-topology check-module-function-matrix check-module-function-coverage-tensor check-module-extension-standard check-contracts-runtime-api-surface check-verification-evidence-surface check-central-store-contract check-central-database-readiness build-central-readiness-report check-central-readiness-report build-module-topology-report check-native-script-audit
.PHONY: check-language-packs check-ui-automation-contract check-gui-runtime-capability-contract check-version-line
.PHONY: check-workflow-dataset-contract check-material-score-contract check-materialization-plan-contract check-material-study-execution-plan-contract check-material-exploration-chain-contract check-material-research-bundle-contract check-material-study-sdk-examples check-operator-task-ir-contract check-operator-package-dynamic-smoke-contract
.PHONY: build-operator-qualification-readiness
.PHONY: capture-line-field-qualification-provenance capture-line-field-qualification-release-evidence capture-beam-frame-qualification-release-evidence
.PHONY: check-line-field-closed-form-baseline check-line-field-qualification-release-evidence check-beam-frame-qualification-release-evidence
.PHONY: check-operator-reliability-rules check-operator-reliability-schemas check-operator-validation verify-operator-validation
.PHONY: capture-material-research-example check-material-research-example verify-material-research-example
.PHONY: build-material-research-bundle check-material-research-bundle verify-material-research-bundle material-research-bundle-index
.PHONY: remote-material-research-example remote-material-research-summary
.PHONY: check-operator-reliability audit-rust-lines audit-project-organization
.PHONY: audit-dependencies fuzz-smoke check-minimal-industrial-closure architecture-check verify

check-doc-book:
	@$(ENTRYPOINT) check-doc-book

sync-doc-book-version:
	@$(ENTRYPOINT) sync-doc-book-version

check-toolchains:
	@$(ENTRYPOINT) check-toolchain-contract

check-elixir-self-host:
	@$(ENTRYPOINT) check-elixir-self-host

check-commercial-readiness:
	@$(ENTRYPOINT) validate-commercial-readiness

check-install-update-disk-hygiene:
	@$(ENTRYPOINT) check-install-update-disk-hygiene --self-test
	@$(ENTRYPOINT) check-install-update-disk-hygiene

check-make-modules:
	@./scripts/kyuubiki check-make-modules

check-module-topology:
	@$(ENTRYPOINT) check-module-topology --self-test
	@$(ENTRYPOINT) check-module-topology

check-module-function-matrix:
	@$(ENTRYPOINT) check-module-function-matrix --self-test
	@$(ENTRYPOINT) check-module-function-matrix

check-module-function-coverage-tensor:
	@$(ENTRYPOINT) check-module-function-coverage-tensor --self-test
	@$(ENTRYPOINT) check-module-function-coverage-tensor

check-module-extension-standard:
	@$(ENTRYPOINT) check-module-extension-standard --self-test
	@$(ENTRYPOINT) check-module-extension-standard

check-contracts-runtime-api-surface:
	@$(ENTRYPOINT) check-contracts-runtime-api-surface --self-test
	@$(ENTRYPOINT) check-contracts-runtime-api-surface

check-verification-evidence-surface:
	@$(ENTRYPOINT) check-verification-evidence-surface

check-central-store-contract:
	@$(ENTRYPOINT) check-central-store-contract --self-test
	@$(ENTRYPOINT) check-central-store-contract

check-central-database-readiness:
	@$(ENTRYPOINT) check-central-database-readiness --self-test
	@$(ENTRYPOINT) check-central-database-readiness --mode $${MODE:-local} --backend $${BACKEND:-sqlite}

build-central-readiness-report:
	@$(ENTRYPOINT) build-central-readiness-report --self-test
	@$(ENTRYPOINT) build-central-readiness-report --mode $${MODE:-local} --backend $${BACKEND:-sqlite} --out $${OUT:-tmp/central-readiness-report.json} --markdown-out $${MARKDOWN_OUT:-tmp/central-readiness-report.md}
	@$(ENTRYPOINT) check-central-readiness-report --self-test
	@$(ENTRYPOINT) check-central-readiness-report --in $${OUT:-tmp/central-readiness-report.json} --markdown-in $${MARKDOWN_OUT:-tmp/central-readiness-report.md}

check-central-readiness-report:
	@$(ENTRYPOINT) check-central-readiness-report --self-test
	@$(ENTRYPOINT) check-central-readiness-report --in $${IN:-tmp/central-readiness-report.json} --markdown-in $${MARKDOWN_IN:-tmp/central-readiness-report.md}

build-module-topology-report:
	@$(ENTRYPOINT) build-module-topology-report --out-dir $${OUT_DIR:-tmp/module-topology}

check-native-script-audit:
	@$(ENTRYPOINT) native-script-audit --self-test
	@$(ENTRYPOINT) native-script-audit

check-language-packs:
	@$(ENTRYPOINT) validate-language-packs

check-ui-automation-contract:
	@$(ENTRYPOINT) check-ui-automation-contract --self-test
	@$(ENTRYPOINT) check-ui-automation-contract

check-gui-runtime-capability-contract:
	@$(ENTRYPOINT) check-gui-runtime-capability-contract --self-test
	@$(ENTRYPOINT) check-gui-runtime-capability-contract

check-version-line:
	@$(ENTRYPOINT) create-release-snapshot --self-test
	@$(ENTRYPOINT) audit-version-line --self-test
	@$(ENTRYPOINT) audit-version-line

check-workflow-dataset-contract:
	@$(ENTRYPOINT) check-workflow-dataset-contract --self-test
	@$(ENTRYPOINT) check-workflow-dataset-contract

check-material-score-contract:
	@$(ENTRYPOINT) validate-material-score-contract

check-materialization-plan-contract:
	@$(ENTRYPOINT) check-materialization-plan-contract --self-test
	@$(ENTRYPOINT) check-materialization-plan-contract

check-material-study-execution-plan-contract:
	@$(ENTRYPOINT) check-material-study-execution-plan-contract --self-test
	@$(ENTRYPOINT) check-material-study-execution-plan-contract

check-material-exploration-chain-contract:
	@$(ENTRYPOINT) check-material-exploration-chain-contract --self-test
	@$(ENTRYPOINT) check-material-exploration-chain-contract

check-material-research-bundle-contract:
	@$(ENTRYPOINT) check-material-research-bundle-contract --self-test
	@$(ENTRYPOINT) check-material-research-bundle-contract

check-material-study-sdk-examples:
	@$(ENTRYPOINT) check-material-study-sdk-examples --self-test
	@$(ENTRYPOINT) check-material-study-sdk-examples

check-operator-task-ir-contract:
	@$(ENTRYPOINT) check-operator-task-ir-contract --self-test
	@$(ENTRYPOINT) check-operator-task-ir-contract

check-operator-package-dynamic-smoke-contract:
	@$(ENTRYPOINT) check-operator-package-dynamic-smoke-contract --self-test

check-operator-reliability-rules:
	@$(ENTRYPOINT) check-operator-reliability-rules

check-operator-reliability-schemas:
	@$(ENTRYPOINT) check-operator-reliability-schemas --self-test
	@$(ENTRYPOINT) check-operator-reliability-schemas

check-operator-validation:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation

verify-operator-validation:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute

build-operator-qualification-readiness:
	@$(ENTRYPOINT) check-operator-qualification-readiness --self-test
	@$(ENTRYPOINT) build-operator-qualification-readiness --out $${OUT:-tmp/operator-qualification-readiness.json}
	@$(ENTRYPOINT) check-operator-qualification-readiness --in $${OUT:-tmp/operator-qualification-readiness.json}

capture-line-field-qualification-provenance:
	@$(ENTRYPOINT) capture-line-field-qualification-provenance --out $${OUT:-tmp/line-field-qualification-provenance.json}

capture-line-field-qualification-release-evidence:
	@$(ENTRYPOINT) capture-line-field-qualification-release-evidence --out $${OUT:-tmp/line-field-qualification-release-evidence.json}

capture-beam-frame-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile beam-frame-classic --out $${OUT:-tmp/beam-frame-classic-qualification-release-evidence.json}

check-beam-frame-qualification-release-evidence:
	@$(ENTRYPOINT) check-beam-frame-qualification-release-evidence --in $${IN:-tmp/beam-frame-classic-qualification-release-evidence.json}

check-line-field-closed-form-baseline:
	@$(ENTRYPOINT) check-line-field-closed-form-baseline

check-line-field-qualification-release-evidence:
	@$(ENTRYPOINT) check-line-field-qualification-release-evidence --in $${IN:-tmp/line-field-qualification-release-evidence.json}

capture-material-research-example:
	@$(ENTRYPOINT) capture-material-research-example --out $${OUT:-tmp/material-research-example.json}

check-material-research-example:
	@$(ENTRYPOINT) check-material-research-example --in $${IN:-tmp/material-research-example.json}

verify-material-research-example:
	@$(MAKE) capture-material-research-example OUT=$${OUT:-tmp/material-research-example.json}
	@$(MAKE) check-material-research-example IN=$${OUT:-tmp/material-research-example.json}

build-material-research-bundle:
	@$(ENTRYPOINT) build-material-research-bundle --study $${STUDY:-heat-spreader} --out $${OUT:-tmp/material-research-bundle.json}

check-material-research-bundle:
	@$(ENTRYPOINT) check-material-research-bundle --self-test
	@$(ENTRYPOINT) check-material-research-bundle --in $${IN:-tmp/material-research-bundle.json}

verify-material-research-bundle:
	@$(MAKE) build-material-research-bundle OUT=$${OUT:-tmp/material-research-bundle.json}
	@$(MAKE) check-material-research-bundle IN=$${OUT:-tmp/material-research-bundle.json}

material-research-bundle-index:
	@$(ENTRYPOINT) build-material-research-bundle-index --self-test
	@$(ENTRYPOINT) build-material-research-bundle-index --ensure-bundles --out-dir $${OUT_DIR:-tmp/material-research-bundles}

remote-material-research-example:
	@$(ENTRYPOINT) remote-material-research-example --profile $${PROFILE:-100k} --matrix $${MATRIX:-compound-core} --repeat $${REPEAT:-1}

remote-material-research-summary:
	@$(ENTRYPOINT) build-remote-material-benchmark-summary --self-test
	@$(ENTRYPOINT) build-remote-material-benchmark-summary
	@$(ENTRYPOINT) check-remote-material-preconditioner-health --self-test
	@$(ENTRYPOINT) check-remote-material-preconditioner-health
	@$(ENTRYPOINT) check-remote-material-stage-health --self-test
	@$(ENTRYPOINT) check-remote-material-stage-health

check-operator-reliability: check-operator-reliability-rules check-operator-reliability-schemas check-line-field-closed-form-baseline build-operator-qualification-readiness
	@$(ENTRYPOINT) check-operator-reliability

audit-rust-lines:
	@$(ENTRYPOINT) rust-line-audit --max $${MAX_LINES:-600}

audit-project-organization:
	@$(ENTRYPOINT) audit-project-organization --self-test
	@$(ENTRYPOINT) audit-project-organization

audit-dependencies:
	@$(ENTRYPOINT) audit-dependencies --self-test
	@$(ENTRYPOINT) audit-dependencies

check-minimal-industrial-closure:
	@$(ENTRYPOINT) validate-minimal-industrial-closure

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
	@$(MAKE) check-module-function-coverage-tensor
	@$(MAKE) check-module-extension-standard
	@$(MAKE) check-contracts-runtime-api-surface
	@$(MAKE) check-verification-evidence-surface
	@$(MAKE) check-central-store-contract
	@$(MAKE) check-central-database-readiness
	@$(MAKE) build-central-readiness-report
	@$(MAKE) test-runtime-surfaces
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
	@$(MAKE) check-minimal-industrial-closure
	@cd apps/web && mix test test/kyuubiki_web/api/operator_task_api_test.exs test/kyuubiki_web/orchestra/operator_task_executor_test.exs test/kyuubiki_web/orchestra/operator_task_ir_test.exs
	@cd workers/rust && cargo test -p kyuubiki-cli operator_task_ir_rpc
	@cd workers/rust && cargo test -p kyuubiki-cli --test operator_task_live

verify:
	@$(MAKE) check-toolchains
	@$(MAKE) check-elixir-self-host
	@$(MAKE) check-native-script-audit
	@$(MAKE) check-central-store-contract
	@$(MAKE) check-central-database-readiness
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
	@$(ENTRYPOINT) playground-fem-node-test
