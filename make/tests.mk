.PHONY: test test-web test-rust test-frontend workflow-preflight test-runtime-surfaces test-sdk
.PHONY: test-agent-capability-smoke test-playground
.PHONY: test-hub-gui test-installer-gui test-workbench-gui
.PHONY: test-integration test-integration-api test-integration-cluster
.PHONY: test-integration-direct-mesh test-integration-desktop-gui
.PHONY: test-integration-benchmark-profile-index
.PHONY: test-integration-direct-mesh-docker test-integration-remote-ssh-fixture
.PHONY: test-integration-direct-mesh-docker-compare
.PHONY: test-integration-direct-mesh-docker-report
.PHONY: test-integration-direct-mesh-docker-nightly
.PHONY: test-integration-workflow-mesh test-integration-workflow-mesh-nightly
.PHONY: test-integration-workflow-catalog-compare
.PHONY: test-integration-workflow-catalog-report
.PHONY: test-integration-workflow-catalog-nightly
.PHONY: test-integration-ui-mechanical test-integration-ui-thermal
.PHONY: format format-web format-rust tdd-web tdd-rust

test: test-web test-rust test-frontend test-sdk test-playground

test-web:
	@cd apps/web && mix test

test-rust:
	@$(ENTRYPOINT) rust-test

test-frontend:
	@cd apps/frontend && npm run typecheck && npm run build

workflow-preflight:
	@cd apps/frontend && npm run check:workflow-preflight

test-runtime-surfaces:
	@cd apps/frontend && npm run test:unit -- hub-runtime-surface installer-runtime-surface workbench-workflow-benchmark-surface
	@cd apps/web && mix test test/kyuubiki_web/orchestra/control_plane_surface_test.exs
	@cd workers/rust && cargo test -p kyuubiki-protocol protocol_benchmark_surface -- --nocapture

test-sdk:
	@$(ENTRYPOINT) sdk-smoke
	@$(MAKE) check-material-study-sdk-examples

test-agent-capability-smoke:
	@$(ENTRYPOINT) agent-capability-smoke --host $${AGENT_HOST:-127.0.0.1} --port $${AGENT_PORT:-5001} --profile $${AGENT_SMOKE_PROFILE:-advertised} --output $${OUTPUT:-tmp/agent-capability-smoke.json} $${AGENT_SMOKE_ARGS:-}

test-playground:
	@node --test apps/web/playground/test/fem.test.mjs

test-hub-gui:
	@cd apps/hub-gui && npm run test:smoke

test-installer-gui:
	@cd apps/installer-gui && npm run test:smoke

test-workbench-gui:
	@cd apps/workbench-gui && npm run test:smoke

test-integration: test-integration-api test-integration-cluster test-integration-direct-mesh test-integration-desktop-gui test-integration-benchmark-profile-index test-integration-ui-mechanical test-integration-ui-thermal

test-integration-api:
	@node --test tests/integration/orchestrator-agent-api-smoke.test.mjs

test-integration-cluster:
	@node --test tests/integration/distributed-control-plane-smoke.test.mjs

test-integration-direct-mesh:
	@node --test tests/integration/direct-mesh-gui-smoke.test.mjs

test-integration-desktop-gui:
	@node --test tests/integration/desktop-shell-regression.test.mjs tests/integration/workbench-shell-regression.test.mjs

test-integration-benchmark-profile-index:
	@node --test tests/integration/benchmark-profile-index.test.mjs

test-integration-direct-mesh-docker:
	@DOCKER_RUN_NETWORK=$${DOCKER_RUN_NETWORK:-host} $(ENTRYPOINT) direct-mesh-benchmark-container --repeat $${REPEAT:-3} --output-dir $${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}

test-integration-remote-ssh-fixture:
	@$(ENTRYPOINT) remote-ssh-fixture

test-integration-direct-mesh-docker-compare:
	@node ./scripts/compare-direct-mesh-benchmark.mjs --current $${CURRENT:-tmp/direct-mesh-benchmark-container/latest/summary.json} --baseline $${BASELINE:-tests/integration/benchmarks/direct-mesh-docker-baseline.json} --json-out $${COMPARE_OUT:-tmp/direct-mesh-benchmark-container/latest/compare.json} --report-out $${REPORT_OUT:-tmp/direct-mesh-benchmark-container/latest/compare.md} --fail-on-elapsed-regression-pct $${DIRECT_MESH_ELAPSED_THRESHOLD:-15} --fail-on-rss-regression-pct $${DIRECT_MESH_RSS_THRESHOLD:-20}

test-integration-direct-mesh-docker-report:
	@$(MAKE) test-integration-direct-mesh-docker REPEAT=$${REPEAT:-3} OUTPUT_DIR=$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}
	@$(MAKE) test-integration-direct-mesh-docker-compare CURRENT=$${CURRENT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/summary.json} COMPARE_OUT=$${COMPARE_OUT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/compare.json} REPORT_OUT=$${REPORT_OUT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/compare.md}

test-integration-direct-mesh-docker-nightly:
	@$(ENTRYPOINT) direct-mesh-benchmark-regression

test-integration-workflow-mesh:
	@$(ENTRYPOINT) workflow-mesh-regression

test-integration-workflow-mesh-nightly:
	@$(ENTRYPOINT) workflow-mesh-regression-remote

test-integration-workflow-catalog-compare:
	@node ./scripts/compare-workflow-catalog-benchmark.mjs --current $${CURRENT:-tmp/workflow-catalog-benchmark.json} --baseline $${BASELINE:-tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json} --json-out $${COMPARE_OUT:-tmp/workflow-catalog-benchmark.compare.json} --report-out $${REPORT_OUT:-tmp/workflow-catalog-benchmark.compare.md} --fail-on-median-regression-pct $${WORKFLOW_MEDIAN_THRESHOLD:-50} --fail-on-avg-regression-pct $${WORKFLOW_AVG_THRESHOLD:-80}

test-integration-workflow-catalog-report:
	@cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_catalog_report_test.exs
	@$(MAKE) test-integration-workflow-catalog-compare CURRENT=$${CURRENT:-tmp/workflow-catalog-benchmark.json} COMPARE_OUT=$${COMPARE_OUT:-tmp/workflow-catalog-benchmark.compare.json} REPORT_OUT=$${REPORT_OUT:-tmp/workflow-catalog-benchmark.compare.md}

test-integration-workflow-catalog-nightly:
	@$(ENTRYPOINT) workflow-catalog-benchmark-regression

test-integration-ui-mechanical:
	@node --test tests/integration/workbench-ui-mechanical-smoke.test.mjs

test-integration-ui-thermal:
	@node --test tests/integration/workbench-ui-thermal-smoke.test.mjs

format: format-web format-rust

format-web:
	@cd apps/web && mix format

format-rust:
	@cd workers/rust && cargo fmt

tdd-web:
	@cd apps/web && mix test $(FILE) $(TEST)

tdd-rust:
	@cd workers/rust && cargo test $(FILTER)
