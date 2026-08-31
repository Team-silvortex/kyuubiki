.PHONY: test test-web test-rust test-rust-scale-profiles test-frontend workflow-preflight test-runtime-surfaces test-sdk
.PHONY: test-agent-capability-smoke test-playground
.PHONY: test-hub-gui test-installer-gui test-workbench-gui
.PHONY: test-integration test-integration-api test-integration-cluster
.PHONY: test-integration-direct-mesh test-integration-desktop-gui qualify-desktop-ui-validation qualify-protocol-validation qualify-contracts-validation qualify-workbench-validation
.PHONY: qualify-headless-sdk-validation qualify-runtime-api-verification qualify-benchmark qualify-orchestra-benchmark qualify-headless-sdk-operational-remote check-headless-sdk-operational-qualification qualify-desktop-deployment-update qualify-system-security qualify-agent-control-link-operational-remote qualify-orchestra-takeover-operational-remote qualify-orchestra-network-partition-operational-remote qualify-orchestra-long-workflow-takeover-operational-remote qualify-orchestra-installed-takeover-operational-remote qualify-agent-solver-operational-remote qualify-agent-update-operational-remote check-agent-update-operational-qualification qualify-runtime-payload-operational-remote check-runtime-payload-operational-qualification qualify-orchestra-workflow-operational-remote qualify-persistence-provenance
.PHONY: qualify-distributed-task-recovery-operational-remote qualify-fleet-scheduling-operational-remote qualify-operator-package-acquisition-operational-remote qualify-operator-sdk-multihost-operational-remote check-operator-sdk-multihost-operational-qualification
.PHONY: qualify-operator-sdk-windows-operational check-operator-sdk-windows-operational-qualification
.PHONY: test-integration-benchmark-profile-index
.PHONY: test-integration-direct-mesh-docker test-integration-remote-ssh-fixture test-central-database-smoke remote-central-database-smoke
.PHONY: test-integration-direct-mesh-docker-compare
.PHONY: test-integration-direct-mesh-docker-report
.PHONY: test-integration-direct-mesh-docker-nightly
.PHONY: test-integration-workflow-mesh test-integration-workflow-mesh-nightly
.PHONY: test-integration-workflow-catalog-compare
.PHONY: test-integration-workflow-catalog-report
.PHONY: test-integration-workflow-catalog-nightly
.PHONY: test-integration-ui-mechanical test-integration-ui-thermal test-integration-ui-workflow test-integration-ui-invocation
.PHONY: format format-web format-rust tdd-web tdd-rust

test: test-web test-rust test-frontend test-sdk test-playground

test-web:
	@cd apps/web && mix test

test-rust:
	@$(ENTRYPOINT) rust-test

test-rust-scale-profiles:
	@cd workers/rust && cargo test -p kyuubiki-benchmark -- --ignored --test-threads=1

test-frontend:
	@cd apps/frontend && npm run typecheck && npm run build

workflow-preflight:
	@cd apps/frontend && npm run check:workflow-preflight

test-runtime-surfaces:
	@cd apps/frontend && npm run test:unit -- hub-runtime-surface installer-runtime-surface workbench-workflow-benchmark-surface
	@cd apps/web && mix test test/kyuubiki_web/orchestra/control_plane_surface_test.exs
	@cd workers/rust && cargo test -p kyuubiki-protocol --lib protocol_benchmark_surface -- --nocapture

test-sdk:
	@$(ENTRYPOINT) sdk-smoke
	@$(MAKE) check-material-study-sdk-examples

test-agent-capability-smoke:
	@$(ENTRYPOINT) agent-capability-smoke --host $${AGENT_HOST:-127.0.0.1} --port $${AGENT_PORT:-5001} --profile $${AGENT_SMOKE_PROFILE:-advertised} --output $${OUTPUT:-tmp/agent-capability-smoke.json} $${AGENT_SMOKE_ARGS:-}

test-playground:
	@$(ENTRYPOINT) playground-fem-node-test

test-hub-gui:
	@cd apps/hub-gui && npm run test:smoke

test-installer-gui:
	@cd apps/installer-gui && npm run test:smoke

test-workbench-gui:
	@cd apps/workbench-gui && npm run test:smoke

test-integration: test-integration-api test-integration-cluster test-integration-direct-mesh test-integration-desktop-gui test-integration-benchmark-profile-index test-integration-ui-workflow test-integration-ui-mechanical test-integration-ui-thermal

test-integration-api:
	@$(ENTRYPOINT) integration-api-node-test

test-integration-cluster:
	@$(ENTRYPOINT) integration-cluster-node-test

test-integration-direct-mesh:
	@$(ENTRYPOINT) integration-direct-mesh-node-test

test-integration-desktop-gui:
	@$(ENTRYPOINT) integration-desktop-gui-node-test

qualify-desktop-ui-validation:
	@$(ENTRYPOINT) check-desktop-ui-validation --out $${OUTPUT:-tmp/desktop-ui-validation-report.json}

qualify-protocol-validation:
	@$(ENTRYPOINT) check-protocol-validation-qualification --out $${OUTPUT:-tmp/protocol-validation-qualification-report.json}

qualify-contracts-validation:
	@$(ENTRYPOINT) check-contracts-validation-qualification --out $${OUTPUT:-tmp/contracts-validation-qualification-report.json}

qualify-workbench-validation:
	@$(ENTRYPOINT) check-workbench-validation-qualification --out $${OUTPUT:-tmp/workbench-validation-qualification-report.json}

qualify-system-security:
	@$(ENTRYPOINT) check-system-security-qualification --self-test
	@$(ENTRYPOINT) check-system-security-qualification --out $${OUTPUT:-tmp/system-security-qualification-report.json}

qualify-agent-solver-operational-remote:
	@$(ENTRYPOINT) qualify-agent-solver-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/agent-solver-operational-remote.json}

qualify-agent-control-link-operational-remote:
	@$(ENTRYPOINT) qualify-agent-control-link-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/agent-control-link-operational-qualification.json} $${TIMEOUT_SECONDS:+--timeout-secs $${TIMEOUT_SECONDS}}

qualify-operator-package-acquisition-operational-remote:
	@$(ENTRYPOINT) qualify-operator-package-acquisition-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/operator-package-acquisition-operational-qualification.json} $${PACKAGE_VERSION:+--package-version $${PACKAGE_VERSION}} $${TIMEOUT_SECONDS:+--timeout-secs $${TIMEOUT_SECONDS}}

qualify-orchestra-takeover-operational-remote:
	@$(ENTRYPOINT) qualify-orchestra-takeover-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/orchestra-takeover-operational-qualification.json} $${POSTGRES_IMAGE:+--postgres-image $${POSTGRES_IMAGE}} $${TIMEOUT_SECONDS:+--timeout-secs $${TIMEOUT_SECONDS}}

qualify-orchestra-network-partition-operational-remote:
	@$(ENTRYPOINT) qualify-orchestra-network-partition-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/orchestra-network-partition-operational-qualification.json} $${POSTGRES_IMAGE:+--postgres-image $${POSTGRES_IMAGE}} $${TIMEOUT_SECONDS:+--timeout-secs $${TIMEOUT_SECONDS}}

qualify-orchestra-long-workflow-takeover-operational-remote:
	@$(ENTRYPOINT) qualify-orchestra-long-workflow-takeover-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/orchestra-long-workflow-takeover-operational-qualification.json} $${POSTGRES_IMAGE:+--postgres-image $${POSTGRES_IMAGE}} $${TIMEOUT_SECONDS:+--timeout-secs $${TIMEOUT_SECONDS}}

qualify-orchestra-installed-takeover-operational-remote:
	@$(ENTRYPOINT) qualify-orchestra-installed-takeover-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/orchestra-installed-takeover-operational-qualification.json} $${POSTGRES_IMAGE:+--postgres-image $${POSTGRES_IMAGE}} $${TIMEOUT_SECONDS:+--timeout-secs $${TIMEOUT_SECONDS}}

qualify-distributed-task-recovery-operational-remote:
	@$(ENTRYPOINT) qualify-distributed-task-recovery-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/distributed-task-recovery-operational-qualification.json} $${TIMEOUT_SECONDS:+--timeout-secs $${TIMEOUT_SECONDS}}

qualify-fleet-scheduling-operational-remote:
	@$(ENTRYPOINT) qualify-fleet-scheduling-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-tmp/fleet-scheduling-operational-qualification.json} $${TIMEOUT_SECONDS:+--timeout-secs $${TIMEOUT_SECONDS}}

qualify-agent-update-operational-remote:
	@$(ENTRYPOINT) qualify-agent-update-operational-remote --host $${REMOTE:-kyuubiki-lab} $${OUTPUT:+--out $${OUTPUT}}

check-agent-update-operational-qualification:
	@$(ENTRYPOINT) check-agent-update-operational-qualification --verify-report $${REPORT:-releases/usability-evidence/2.14.3/agent-update-operational-qualification.json} --require-remote-linux

qualify-runtime-payload-operational-remote:
	@$(ENTRYPOINT) qualify-runtime-payload-operational-remote --host $${REMOTE:-kyuubiki-lab} $${OUTPUT:+--out $${OUTPUT}}

check-runtime-payload-operational-qualification:
	@$(ENTRYPOINT) check-runtime-payload-operational-qualification --verify-report $${REPORT:-releases/usability-evidence/2.14.3/runtime-payload-operational-qualification.json} --require-remote-linux

qualify-orchestra-workflow-operational-remote:
	@KYUUBIKI_LAB_HOST=$${REMOTE:-kyuubiki-lab} OUTPUT_SLUG=orchestra-workflow-operational LOCAL_OUTPUT_DIR=tmp/orchestra-workflow-operational REMOTE_OUTPUT_DIR=tmp/orchestra-workflow-operational $(ENTRYPOINT) workflow-mesh-regression-remote
	@$(ENTRYPOINT) check-orchestra-recovery-fault-injection --out tmp/orchestra-workflow-operational/orchestra-process-loss-fault-injection.json
	@$(ENTRYPOINT) check-orchestra-workflow-operational-qualification --out $${OUTPUT:-releases/usability-evidence/2.14.1/orchestra-workflow-operational-qualification.json}

qualify-persistence-provenance:
	@$(ENTRYPOINT) check-persistence-provenance-qualification --out $${OUTPUT:-tmp/persistence-provenance-qualification-report.json}

qualify-headless-sdk-validation:
	@$(ENTRYPOINT) check-headless-sdk-validation-qualification --out $${OUTPUT:-tmp/headless-sdk-validation-qualification-report.json}

qualify-runtime-api-verification:
	@$(ENTRYPOINT) check-protocol-validation-qualification --out $${PROTOCOL_OUTPUT:-releases/usability-evidence/2.15.0/protocol-runtime-api-verification.json}
	@$(ENTRYPOINT) check-headless-sdk-validation-qualification --out $${HEADLESS_OUTPUT:-releases/usability-evidence/2.15.0/headless-runtime-api-verification.json}

qualify-benchmark:
	@$(ENTRYPOINT) check-benchmark-qualification --out $${OUTPUT:-releases/usability-evidence/2.15.0/benchmark-qualification.json}

qualify-orchestra-benchmark:
	@$(ENTRYPOINT) check-orchestra-benchmark-qualification --out $${OUTPUT:-releases/usability-evidence/2.15.0/orchestra-benchmark-qualification.json}

qualify-headless-sdk-operational-remote:
	@$(ENTRYPOINT) qualify-headless-sdk-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-releases/usability-evidence/2.14.1/headless-sdk-operational-qualification.json}

check-headless-sdk-operational-qualification:
	@$(ENTRYPOINT) check-headless-sdk-operational-qualification $${REPORT:+--verify-report $${REPORT}}

qualify-operator-sdk-multihost-operational-remote:
	@$(ENTRYPOINT) qualify-operator-sdk-multihost-operational-remote --host $${REMOTE:-kyuubiki-lab} --out $${OUTPUT:-releases/usability-evidence/2.16.4/operator-sdk-multihost-operational-qualification.json}

check-operator-sdk-multihost-operational-qualification:
	@$(ENTRYPOINT) check-operator-sdk-multihost-operational-qualification --self-test --verify-report $${REPORT:-releases/usability-evidence/2.16.4/operator-sdk-multihost-operational-qualification.json}

qualify-operator-sdk-windows-operational:
	@$(ENTRYPOINT) qualify-operator-sdk-windows-operational $${OUTPUT:+--out $${OUTPUT}}

check-operator-sdk-windows-operational-qualification:
	@$(ENTRYPOINT) check-operator-sdk-windows-operational-qualification --self-test $${REPORT:+--verify-report $${REPORT}}

qualify-headless-workflow:
	@$(ENTRYPOINT) check-headless-workflow-qualification --out $${OUTPUT:-tmp/headless-workflow-qualification-report.json}

qualify-desktop-deployment-update:
	@$(ENTRYPOINT) check-desktop-deployment-update-qualification --out $${OUTPUT:-tmp/desktop-deployment-update-qualification-report.json}

test-integration-benchmark-profile-index:
	@$(ENTRYPOINT) integration-benchmark-profile-index-node-test

test-integration-direct-mesh-docker:
	@if [ "$${LOCAL_DOCKER:-0}" = "1" ]; then \
		DOCKER_RUN_NETWORK=$${DOCKER_RUN_NETWORK:-host} $(ENTRYPOINT) direct-mesh-benchmark-container --repeat $${REPEAT:-3} --output-dir $${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}; \
	else \
		$(ENTRYPOINT) direct-mesh-benchmark-regression; \
	fi

test-integration-remote-ssh-fixture:
	@$(ENTRYPOINT) remote-ssh-fixture

test-central-database-smoke:
	@$(ENTRYPOINT) central-database-smoke --mode $${MODE:-cloud} --backend $${BACKEND:-postgres}

remote-central-database-smoke:
	@$(ENTRYPOINT) remote-central-database-smoke --host $${REMOTE:-kyuubiki-lab} --mode $${MODE:-cloud} --backend $${BACKEND:-postgres}

test-integration-direct-mesh-docker-compare:
	@$(ENTRYPOINT) compare-direct-mesh-benchmark --current $${CURRENT:-tmp/direct-mesh-benchmark-container/latest/summary.json} --baseline $${BASELINE:-tests/integration/benchmarks/direct-mesh-docker-baseline.json} --json-out $${COMPARE_OUT:-tmp/direct-mesh-benchmark-container/latest/compare.json} --report-out $${REPORT_OUT:-tmp/direct-mesh-benchmark-container/latest/compare.md} --fail-on-elapsed-regression-pct $${DIRECT_MESH_ELAPSED_THRESHOLD:-15} --fail-on-rss-regression-pct $${DIRECT_MESH_RSS_THRESHOLD:-20}

test-integration-direct-mesh-docker-report:
	@if [ "$${LOCAL_DOCKER:-0}" = "1" ]; then \
		DOCKER_RUN_NETWORK=$${DOCKER_RUN_NETWORK:-host} $(ENTRYPOINT) direct-mesh-benchmark-container --repeat $${REPEAT:-3} --output-dir $${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}; \
		$(ENTRYPOINT) compare-direct-mesh-benchmark --current $${CURRENT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/summary.json} --baseline $${BASELINE:-tests/integration/benchmarks/direct-mesh-docker-baseline.json} --json-out $${COMPARE_OUT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/compare.json} --report-out $${REPORT_OUT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/compare.md} --fail-on-elapsed-regression-pct $${DIRECT_MESH_ELAPSED_THRESHOLD:-15} --fail-on-rss-regression-pct $${DIRECT_MESH_RSS_THRESHOLD:-20}; \
	else \
		$(ENTRYPOINT) direct-mesh-benchmark-regression; \
	fi

test-integration-direct-mesh-docker-nightly:
	@$(ENTRYPOINT) direct-mesh-benchmark-regression

test-integration-workflow-mesh:
	@$(ENTRYPOINT) workflow-mesh-regression

test-integration-workflow-mesh-nightly:
	@$(ENTRYPOINT) workflow-mesh-regression-remote

test-integration-workflow-catalog-compare:
	@$(ENTRYPOINT) compare-workflow-catalog-benchmark --current $${CURRENT:-tmp/workflow-catalog-benchmark.json} --baseline $${BASELINE:-tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json} --json-out $${COMPARE_OUT:-tmp/workflow-catalog-benchmark.compare.json} --report-out $${REPORT_OUT:-tmp/workflow-catalog-benchmark.compare.md} --fail-on-median-regression-pct $${WORKFLOW_MEDIAN_THRESHOLD:-50} --fail-on-avg-regression-pct $${WORKFLOW_AVG_THRESHOLD:-80}

test-integration-workflow-catalog-report:
	@cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_catalog_report_test.exs
	@$(MAKE) test-integration-workflow-catalog-compare CURRENT=$${CURRENT:-tmp/workflow-catalog-benchmark.json} COMPARE_OUT=$${COMPARE_OUT:-tmp/workflow-catalog-benchmark.compare.json} REPORT_OUT=$${REPORT_OUT:-tmp/workflow-catalog-benchmark.compare.md}

test-integration-workflow-catalog-nightly:
	@$(ENTRYPOINT) workflow-catalog-benchmark-regression

test-integration-ui-mechanical:
	@$(ENTRYPOINT) integration-ui-mechanical-node-test

test-integration-ui-thermal:
	@$(ENTRYPOINT) integration-ui-thermal-node-test

test-integration-ui-workflow:
	@$(ENTRYPOINT) integration-ui-workflow-node-test

test-integration-ui-invocation:
	@$(MAKE) check-ui-automation-contract
	@$(MAKE) test-integration-desktop-gui
	@$(MAKE) test-integration-ui-workflow
	@$(MAKE) test-integration-ui-mechanical
	@$(MAKE) test-integration-ui-thermal

format: format-web format-rust

format-web:
	@cd apps/web && mix format

format-rust:
	@cd workers/rust && cargo fmt

tdd-web:
	@cd apps/web && mix test $(FILE) $(TEST)

tdd-rust:
	@cd workers/rust && cargo test $(FILTER)
